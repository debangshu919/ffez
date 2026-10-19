use crate::{error::FfezError, planner::ExecutionPlan};
use std::process::Command;
use tempfile::Builder;

pub fn check_ffmpeg() -> Result<(), FfezError> {
    which::which("ffmpeg").map_err(|_| FfezError::MissingFfmpeg)?;
    Ok(())
}

pub fn format_dry_run(plan: &ExecutionPlan) -> String {
    let mut parts = vec!["ffmpeg".to_string(), "-y".to_string()];
    for arg in &plan.ffmpeg_args {
        let s = arg.to_string_lossy();
        if s.contains(' ') {
            parts.push(format!("\"{s}\""));
        } else {
            parts.push(s.into_owned());
        }
    }

    let out = plan.final_output.to_string_lossy();
    if out.contains(' ') {
        parts.push(format!("\"{out}\""));
    } else {
        parts.push(out.into_owned());
    }

    parts.join(" ")
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
}
