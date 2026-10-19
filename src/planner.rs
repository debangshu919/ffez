use crate::cli::AudioFormat;
use crate::error::FfezError;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq)]
pub struct ExecutionPlan {
    pub input: PathBuf,
    pub final_output: PathBuf,
    pub overwrite: bool,
    /// Arguments excluding the final output path (injected by the runner for temp-file safety)
    pub ffmpeg_args: Vec<OsString>,
}

pub fn plan_mute(
    input: &Path,
    output: Option<&Path>,
    overwrite: bool,
) -> Result<ExecutionPlan, FfezError> {
    let final_output = match output {
        Some(out) => out.to_path_buf(),
        None => derive_default_output(input, "muted")?,
    };

    let ffmpeg_args = vec![
        OsString::from("-hide_banner"),
        OsString::from("-loglevel"),
        OsString::from("error"),
        OsString::from("-i"),
        input.as_os_str().to_os_string(),
        OsString::from("-c:v"),
        OsString::from("copy"),
        OsString::from("-an"),
        OsString::from("-movflags"),
        OsString::from("+faststart"),
    ];

    Ok(ExecutionPlan {
        input: input.to_path_buf(),
        final_output,
        overwrite,
        ffmpeg_args,
    })
}

pub fn plan_audio(
    input: &Path,
    output: Option<&Path>,
    format: Option<AudioFormat>,
    overwrite: bool,
) -> Result<ExecutionPlan, FfezError> {
    // 1. Resolve format: explicit --format > inferred from -o extension > default Mp3
    let resolved_format = format
        .or_else(|| {
            output
                .and_then(|p| p.extension())
                .and_then(|e| e.to_str())
                .and_then(AudioFormat::from_extension)
        })
        .unwrap_or(AudioFormat::Mp3);

    // 2. Resolve output path
    let final_output = match output {
        Some(out) => out.to_path_buf(),
        None => derive_audio_output(input, resolved_format.extension())?,
    };

    // 3. Build format-specific codec flags
    let mut ffmpeg_args = vec![
        OsString::from("-hide_banner"),
        OsString::from("-loglevel"),
        OsString::from("error"),
        OsString::from("-i"),
        input.as_os_str().to_os_string(),
        OsString::from("-vn"), // Strip video track
    ];

    match resolved_format {
        AudioFormat::Mp3 => {
            ffmpeg_args.extend([
                OsString::from("-c:a"),
                OsString::from("libmp3lame"),
                OsString::from("-b:a"),
                OsString::from("192k"),
            ]);
        }
        AudioFormat::Wav => {
            ffmpeg_args.extend([OsString::from("-c:a"), OsString::from("pcm_s16le")]);
        }
        AudioFormat::M4a => {
            ffmpeg_args.extend([
                OsString::from("-c:a"),
                OsString::from("aac"),
                OsString::from("-b:a"),
                OsString::from("192k"),
                OsString::from("-movflags"),
                OsString::from("+faststart"),
            ]);
        }
        AudioFormat::Flac => {
            ffmpeg_args.extend([OsString::from("-c:a"), OsString::from("flac")]);
        }
    }

    Ok(ExecutionPlan {
        input: input.to_path_buf(),
        final_output,
        overwrite,
        ffmpeg_args,
    })
}

/// Converts `path/to/video.mp4` -> `path/to/video_muted.mp4`
fn derive_default_output(input: &Path, suffix: &str) -> Result<PathBuf, FfezError> {
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| FfezError::InvalidPath(input.to_path_buf()))?;

    let ext = input.extension().and_then(|e| e.to_str()).unwrap_or("mp4");
    let parent = input.parent().unwrap_or_else(|| Path::new(""));

    Ok(parent.join(format!("{stem}_{suffix}.{ext}")))
}

/// Converts `path/to/video.mp4` -> `path/to/video.mp3` (or `video_audio.mp3` if input is already `.mp3`)
fn derive_audio_output(input: &Path, target_ext: &str) -> Result<PathBuf, FfezError> {
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| FfezError::InvalidPath(input.to_path_buf()))?;

    let parent = input.parent().unwrap_or_else(|| Path::new(""));
    let candidate = parent.join(format!("{stem}.{target_ext}"));

    if candidate == input {
        Ok(parent.join(format!("{stem}_audio.{target_ext}")))
    } else {
        Ok(candidate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_output_naming() {
        let input = Path::new("clips/gameplay.mp4");
        let plan = plan_mute(input, None, false).unwrap();
        assert_eq!(plan.final_output, PathBuf::from("clips/gameplay_muted.mp4"));
    }

    #[test]
    fn test_mute_args_contain_copy_and_an() {
        let input = Path::new("test.mp4");
        let plan = plan_mute(input, None, false).unwrap();
        assert!(plan.ffmpeg_args.contains(&OsString::from("copy")));
        assert!(plan.ffmpeg_args.contains(&OsString::from("-an")));
    }

    #[test]
    fn test_audio_defaults_to_mp3() {
        let input = Path::new("clips/interview.mp4");
        let plan = plan_audio(input, None, None, false).unwrap();

        assert_eq!(plan.final_output, PathBuf::from("clips/interview.mp3"));
        assert!(plan.ffmpeg_args.contains(&OsString::from("-vn")));
        assert!(plan.ffmpeg_args.contains(&OsString::from("libmp3lame")));
    }

    #[test]
    fn test_audio_explicit_format_flag() {
        let input = Path::new("interview.mp4");
        let plan = plan_audio(input, None, Some(AudioFormat::Wav), false).unwrap();

        assert_eq!(plan.final_output, PathBuf::from("interview.wav"));
        assert!(plan.ffmpeg_args.contains(&OsString::from("pcm_s16le")));
    }

    #[test]
    fn test_audio_infers_format_from_custom_output_extension() {
        let input = Path::new("interview.mp4");
        let custom_out = Path::new("extracted/voice.flac");
        let plan = plan_audio(input, Some(custom_out), None, false).unwrap();

        assert_eq!(plan.final_output, PathBuf::from("extracted/voice.flac"));
        assert!(plan.ffmpeg_args.contains(&OsString::from("flac")));
    }

    #[test]
    fn test_audio_avoids_colliding_with_input_file() {
        let input = Path::new("already_audio.mp3");
        let plan = plan_audio(input, None, Some(AudioFormat::Mp3), false).unwrap();

        assert_eq!(plan.final_output, PathBuf::from("already_audio_audio.mp3"));
    }
}
