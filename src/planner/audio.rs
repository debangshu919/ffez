use crate::cli::AudioFormat;
use crate::error::FfezError;
use crate::planner::ExecutionPlan;
use crate::planner::utils::ensure_extension;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

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

    // 2. Resolve output path and guarantee it has an extension
    let final_output = match output {
        Some(out) => ensure_extension(out, resolved_format.extension()),
        None => derive_audio_output(input, resolved_format.extension())?,
    };

    if final_output == input {
        return Err(FfezError::InputOutputSame(final_output));
    }

    // 3. Build format-specific codec flags
    let mut ffmpeg_args = vec![
        OsString::from("-hide_banner"),
        OsString::from("-loglevel"),
        OsString::from("error"),
        OsString::from("-i"),
        input.as_os_str().to_os_string(),
        OsString::from("-vn"),
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

/// Converts `path/to/video.mp4` -> `path/to/video.mp3`
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

    #[test]
    fn test_audio_appends_format_extension_when_output_has_none() {
        let input = Path::new("interview.mp4");
        let custom_out = Path::new("podcast_episode");

        // With explicit --format wav and extensionless -o
        let plan = plan_audio(input, Some(custom_out), Some(AudioFormat::Wav), false).unwrap();
        assert_eq!(plan.final_output, PathBuf::from("podcast_episode.wav"));

        // With default format (mp3) and extensionless -o
        let plan_default = plan_audio(input, Some(custom_out), None, false).unwrap();
        assert_eq!(
            plan_default.final_output,
            PathBuf::from("podcast_episode.mp3")
        );
    }

    #[test]
    fn test_audio_rejects_output_identical_to_input() {
        let input = Path::new("song.mp3");
        let err = plan_audio(input, Some(Path::new("song.mp3")), None, false).unwrap_err();
        assert!(matches!(err, FfezError::InputOutputSame(_)));

        // Also caught when user omits extension in output
        let err2 = plan_audio(input, Some(Path::new("song")), None, false).unwrap_err();
        assert!(matches!(err2, FfezError::InputOutputSame(_)));
    }

    #[test]
    fn test_audio_all_codec_flags() {
        let input = Path::new("input.mp4");

        // MP3
        let plan_mp3 = plan_audio(input, None, Some(AudioFormat::Mp3), false).unwrap();
        assert!(
            plan_mp3
                .ffmpeg_args
                .windows(2)
                .any(|w| w[0] == "-c:a" && w[1] == "libmp3lame")
        );
        assert!(
            plan_mp3
                .ffmpeg_args
                .windows(2)
                .any(|w| w[0] == "-b:a" && w[1] == "192k")
        );

        // WAV
        let plan_wav = plan_audio(input, None, Some(AudioFormat::Wav), false).unwrap();
        assert!(
            plan_wav
                .ffmpeg_args
                .windows(2)
                .any(|w| w[0] == "-c:a" && w[1] == "pcm_s16le")
        );

        // M4A
        let plan_m4a = plan_audio(input, None, Some(AudioFormat::M4a), false).unwrap();
        assert!(
            plan_m4a
                .ffmpeg_args
                .windows(2)
                .any(|w| w[0] == "-c:a" && w[1] == "aac")
        );
        assert!(
            plan_m4a
                .ffmpeg_args
                .windows(2)
                .any(|w| w[0] == "-b:a" && w[1] == "192k")
        );
        assert!(
            plan_m4a
                .ffmpeg_args
                .windows(2)
                .any(|w| w[0] == "-movflags" && w[1] == "+faststart")
        );

        // FLAC
        let plan_flac = plan_audio(input, None, Some(AudioFormat::Flac), false).unwrap();
        assert!(
            plan_flac
                .ffmpeg_args
                .windows(2)
                .any(|w| w[0] == "-c:a" && w[1] == "flac")
        );
    }

    #[test]
    fn test_audio_format_precedence_over_output_extension() {
        let input = Path::new("input.mp4");
        // User provides -o track.wav, but explicitly requests -f flac
        let custom_out = Path::new("track.wav");
        let plan = plan_audio(input, Some(custom_out), Some(AudioFormat::Flac), false).unwrap();

        // Target codec is FLAC because format flag takes precedence
        assert!(
            plan.ffmpeg_args
                .windows(2)
                .any(|w| w[0] == "-c:a" && w[1] == "flac")
        );
        // Existing extension is preserved on output path
        assert_eq!(plan.final_output, PathBuf::from("track.wav"));
    }

    #[test]
    fn test_audio_with_extensionless_input() {
        let input = Path::new("myvideo");
        let plan = plan_audio(input, None, None, false).unwrap();
        assert_eq!(plan.final_output, PathBuf::from("myvideo.mp3"));
    }
}
