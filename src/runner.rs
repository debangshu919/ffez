use crate::error::FfezError;
use crate::planner::{CompressPlan, ExecutionPlan};
use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::process::{Command, Stdio};
use tempfile::Builder;

pub fn check_ffmpeg() -> Result<(), FfezError> {
    which::which("ffmpeg").map_err(|_| FfezError::MissingFfmpeg)?;
    Ok(())
}

pub fn check_ffprobe() -> Result<(), FfezError> {
    which::which("ffprobe").map_err(|_| FfezError::MissingFfprobe)?;
    Ok(())
}

pub fn format_dry_run(plan: &ExecutionPlan) -> String {
    format_ffmpeg_invocation(&plan.ffmpeg_args, Some(&plan.final_output))
}

pub fn format_compress_dry_run(plan: &CompressPlan) -> String {
    let pass1 = format_ffmpeg_invocation(&plan.pass1_args, None);
    let pass2 = format_ffmpeg_invocation(&plan.pass2_args, Some(&plan.final_output));
    format!("{pass1}\n{pass2}")
}

fn format_ffmpeg_invocation(args: &[OsString], output: Option<&Path>) -> String {
    let mut parts = vec!["ffmpeg".to_string(), "-y".to_string()];
    for arg in args {
        parts.push(quote_arg(arg));
    }
    if let Some(output) = output {
        parts.push(quote_arg(output.as_os_str()));
    }
    parts.join(" ")
}

fn quote_arg(arg: &OsStr) -> String {
    let text = arg.to_string_lossy();
    if text.contains(' ') {
        format!("\"{text}\"")
    } else {
        text.into_owned()
    }
}

pub fn execute(plan: &ExecutionPlan) -> Result<(), FfezError> {
    if !plan.input.exists() {
        return Err(FfezError::InputNotFound(plan.input.clone()));
    }

    if plan.final_output.exists() && !plan.overwrite {
        return Err(FfezError::OutputExists(plan.final_output.clone()));
    }

    check_ffmpeg()?;

    // Keep the extension on the temp file so FFmpeg knows which muxer (e.g. .mp4, .mkv) to use
    let ext = plan
        .final_output
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("mp4");

    // Create the temp file in the same directory as the output so `fs::rename` is an instant atomic move
    let output_dir = plan
        .final_output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| std::path::Path::new("."));

    let temp_file = Builder::new()
        .prefix(".ffez_tmp_")
        .suffix(&format!(".{ext}"))
        .tempfile_in(output_dir)?;

    let temp_path = temp_file.path().to_path_buf();

    let output = Command::new("ffmpeg")
        .arg("-y") // Safe because we are writing to our own isolated temp file
        .args(&plan.ffmpeg_args)
        .arg(&temp_path)
        .output()?;

    if !output.status.success() {
        let code = output.status.code().unwrap_or(-1);
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        // `temp_file` automatically deletes itself on drop here!
        return Err(FfezError::FfmpegFailed { code, stderr });
    }

    // Persist the completed temp file to the final destination
    temp_file.persist(&plan.final_output).map_err(|e| e.error)?;

    Ok(())
}

pub fn execute_compress(plan: &CompressPlan) -> Result<(), FfezError> {
    if !plan.input.exists() {
        return Err(FfezError::InputNotFound(plan.input.clone()));
    }

    if plan.final_output.exists() && !plan.overwrite {
        return Err(FfezError::OutputExists(plan.final_output.clone()));
    }

    check_ffmpeg()?;

    let log_dir = tempfile::tempdir()?;
    let passlog = log_dir.path().join("ffezpass");

    println!("Analyzing video (pass 1 of 2)...");
    run_ffmpeg(&with_passlogfile(&plan.pass1_args, &passlog))?;

    let ext = plan
        .final_output
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("mp4");
    let output_dir = plan
        .final_output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let temp_file = Builder::new()
        .prefix(".ffez_tmp_")
        .suffix(&format!(".{ext}"))
        .tempfile_in(output_dir)?;

    let mut pass2 = with_passlogfile(&plan.pass2_args, &passlog);
    pass2.push(temp_file.path().as_os_str().to_os_string());

    println!("Encoding video (pass 2 of 2)...");
    run_ffmpeg(&pass2)?;

    temp_file.persist(&plan.final_output).map_err(|e| e.error)?;
    Ok(())
}

fn with_passlogfile(args: &[OsString], passlog: &Path) -> Vec<OsString> {
    let mut rewritten = Vec::with_capacity(args.len() + 2);
    let mut inserted = false;
    for arg in args {
        if !inserted && arg == "-pass" {
            rewritten.push(OsString::from("-passlogfile"));
            rewritten.push(passlog.as_os_str().to_os_string());
            inserted = true;
        }
        rewritten.push(arg.clone());
    }
    rewritten
}

fn run_ffmpeg(args: &[OsString]) -> Result<(), FfezError> {
    let status = Command::new("ffmpeg")
        .arg("-y")
        .args(args)
        .stdin(Stdio::null())
        .status()?;

    if status.success() {
        Ok(())
    } else {
        Err(FfezError::FfmpegFailed {
            code: status.code().unwrap_or(-1),
            stderr: String::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planner::plan_mute;
    use std::fs;
    use std::fs::File;
    use std::path::Path;
    use tempfile::tempdir;

    #[test]
    fn test_format_dry_run_quotes_paths_with_spaces() {
        let input = Path::new("my clips/raw video.mp4");
        let plan = plan_mute(input, None, false).unwrap();
        let cmd = format_dry_run(&plan);

        assert!(cmd.starts_with("ffmpeg -y -hide_banner"));
        assert!(cmd.contains(&format!("\"{}\"", input.display())));
        assert!(cmd.contains(&format!("\"{}\"", plan.final_output.display())));
    }

    #[test]
    fn test_execute_fails_when_input_missing() {
        let dir = tempdir().unwrap();
        let missing_input = dir.path().join("ghost.mp4");
        let plan = plan_mute(&missing_input, None, false).unwrap();

        let err = execute(&plan).unwrap_err();
        assert!(matches!(err, FfezError::InputNotFound(_)));
    }

    #[test]
    fn test_execute_blocks_overwrite_when_flag_is_false() {
        let dir = tempdir().unwrap();
        let input = dir.path().join("video.mp4");
        let output = dir.path().join("video_muted.mp4");

        // Create dummy files for both input and output
        File::create(&input).unwrap();
        File::create(&output).unwrap();

        let plan = plan_mute(&input, Some(&output), false).unwrap();
        let err = execute(&plan).unwrap_err();

        assert!(matches!(err, FfezError::OutputExists(_)));
    }

    #[test]
    fn test_execute_cleans_up_temp_file_on_corrupt_input() {
        if check_ffmpeg().is_err() {
            return; // Skip if running in a minimal environment without ffmpeg
        }

        let dir = tempdir().unwrap();
        let corrupt_input = dir.path().join("corrupt.mp4");
        let output = dir.path().join("corrupt_muted.mp4");

        // Write garbage bytes so ffmpeg fails with non-zero exit code
        fs::write(&corrupt_input, b"not a valid mp4 header").unwrap();

        let plan = plan_mute(&corrupt_input, Some(&output), false).unwrap();
        let err = execute(&plan).unwrap_err();

        // 1. Verify we captured FfmpegFailed and its stderr
        assert!(matches!(err, FfezError::FfmpegFailed { .. }));
        assert!(err.ffmpeg_stderr().is_some());

        // 2. Verify final output was never created AND no leftover `.ffez_tmp_` files exist!
        assert!(!output.exists());
        let remaining_files: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(remaining_files, vec!["corrupt.mp4"]);
    }

    #[test]
    fn test_format_dry_run_without_spaces() {
        let input = Path::new("clip.mp4");
        let plan = plan_mute(input, None, false).unwrap();
        let cmd = format_dry_run(&plan);

        assert_eq!(
            cmd,
            "ffmpeg -y -hide_banner -loglevel error -i clip.mp4 -c:v copy -an -movflags +faststart clip_muted.mp4"
        );
    }

    #[test]
    fn test_format_dry_run_audio_plan() {
        use crate::planner::plan_audio;
        let input = Path::new("interview.mp4");
        let plan = plan_audio(input, None, None, false).unwrap();
        let cmd = format_dry_run(&plan);

        assert!(cmd.contains("-vn"));
        assert!(cmd.contains("-c:a libmp3lame"));
        assert!(cmd.contains("-b:a 192k"));
        assert!(cmd.ends_with("interview.mp3"));
    }

    #[test]
    fn test_execute_allows_overwrite_when_flag_is_true() {
        if check_ffmpeg().is_err() {
            return;
        }

        let dir = tempdir().unwrap();
        let input = dir.path().join("video.mp4");
        let output = dir.path().join("video_muted.mp4");

        // Create dummy files for both input and output
        File::create(&input).unwrap();
        File::create(&output).unwrap();

        let plan = plan_mute(&input, Some(&output), true).unwrap();
        let err = execute(&plan).unwrap_err();

        // Should NOT fail with OutputExists because overwrite is true.
        // It fails with FfmpegFailed because the dummy file has invalid video content.
        assert!(
            matches!(err, FfezError::FfmpegFailed { .. }),
            "Expected FfmpegFailed but got: {err:?}"
        );
    }

    #[test]
    fn test_execute_audio_cleans_up_temp_file_on_corrupt_input() {
        if check_ffmpeg().is_err() {
            return;
        }

        use crate::planner::plan_audio;
        let dir = tempdir().unwrap();
        let corrupt_input = dir.path().join("corrupt_video.mp4");
        let output = dir.path().join("extracted_audio.mp3");

        fs::write(&corrupt_input, b"not valid video or audio").unwrap();

        let plan = plan_audio(&corrupt_input, Some(&output), None, false).unwrap();
        let err = execute(&plan).unwrap_err();

        assert!(matches!(err, FfezError::FfmpegFailed { .. }));
        assert!(!output.exists());
        let remaining_files: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(remaining_files, vec!["corrupt_video.mp4"]);
    }

    #[test]
    fn test_check_ffmpeg_reflects_environment() {
        let has_ffmpeg = which::which("ffmpeg").is_ok();
        let res = check_ffmpeg();
        if has_ffmpeg {
            assert!(res.is_ok());
        } else {
            assert!(matches!(res.unwrap_err(), FfezError::MissingFfmpeg));
        }
    }

    #[test]
    fn test_check_ffprobe_reflects_environment() {
        let has_ffprobe = which::which("ffprobe").is_ok();
        let res = check_ffprobe();
        if has_ffprobe {
            assert!(res.is_ok());
        } else {
            assert!(matches!(res.unwrap_err(), FfezError::MissingFfprobe));
        }
    }

    #[test]
    fn test_format_compress_dry_run_prints_both_passes() {
        use crate::planner::{VideoSource, plan_compress};
        use bytesize::ByteSize;

        let source = VideoSource {
            duration_secs: 60.0,
            has_audio: true,
            width: 320,
            height: 240,
            fps: 24.0,
        };
        let plan = plan_compress(
            Path::new("clip.mp4"),
            None,
            false,
            ByteSize::mb(25),
            &source,
        )
        .unwrap();
        let cmd = format_compress_dry_run(&plan);

        assert_eq!(
            cmd,
            "\
ffmpeg -y -nostdin -hide_banner -loglevel error -stats -i clip.mp4 -map 0:v:0 -c:v libx264 -preset medium -pix_fmt yuv420p -b:v 3038666 -fps_mode cfr -r 24 -pass 1 -an -f null -
ffmpeg -y -nostdin -hide_banner -loglevel error -stats -i clip.mp4 -map 0:v:0 -c:v libx264 -preset medium -pix_fmt yuv420p -b:v 3038666 -fps_mode cfr -r 24 -pass 2 -map 0:a:0 -c:a aac -b:a 128k -movflags +faststart clip_compressed.mp4"
        );
    }

    #[test]
    fn test_format_compress_dry_run_quotes_paths_with_spaces() {
        use crate::planner::{VideoSource, plan_compress};
        use bytesize::ByteSize;

        let source = VideoSource {
            duration_secs: 60.0,
            has_audio: false,
            width: 320,
            height: 240,
            fps: 24.0,
        };
        let input = Path::new("raw vacation video.mp4");
        let output = Path::new("small vacation video.mp4");
        let plan = plan_compress(input, Some(output), false, ByteSize::mb(25), &source).unwrap();
        let cmd = format_compress_dry_run(&plan);

        assert!(cmd.contains("\"raw vacation video.mp4\""));
        assert!(cmd.contains("\"small vacation video.mp4\""));
        assert!(!cmd.contains("scale=-2:"));
    }

    #[test]
    fn test_execute_compress_cleans_up_after_corrupt_input() {
        if check_ffmpeg().is_err() {
            return;
        }

        use crate::planner::{VideoSource, plan_compress};
        use bytesize::ByteSize;

        let dir = tempdir().unwrap();
        let corrupt_input = dir.path().join("corrupt.mp4");
        let output = dir.path().join("corrupt_compressed.mp4");
        fs::write(&corrupt_input, b"not a valid mp4 header").unwrap();

        let source = VideoSource {
            duration_secs: 2.0,
            has_audio: true,
            width: 320,
            height: 240,
            fps: 24.0,
        };
        let plan = plan_compress(
            &corrupt_input,
            Some(&output),
            false,
            ByteSize::kb(500),
            &source,
        )
        .unwrap();
        let err = execute_compress(&plan).unwrap_err();

        assert!(matches!(err, FfezError::FfmpegFailed { .. }));
        assert!(!output.exists());
        let remaining_files: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        assert_eq!(remaining_files, vec!["corrupt.mp4"]);
    }

    #[test]
    fn test_with_passlogfile_inserts_before_pass() {
        let args = vec![
            OsString::from("-i"),
            OsString::from("clip.mp4"),
            OsString::from("-pass"),
            OsString::from("1"),
        ];
        let rewritten = with_passlogfile(&args, Path::new("/tmp/ffezpass"));
        assert_eq!(
            rewritten,
            vec![
                OsString::from("-i"),
                OsString::from("clip.mp4"),
                OsString::from("-passlogfile"),
                OsString::from("/tmp/ffezpass"),
                OsString::from("-pass"),
                OsString::from("1"),
            ]
        );
    }
}
