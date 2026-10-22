use crate::error::FfezError;
use crate::planner::ExecutionPlan;
use crate::planner::utils::{derive_suffixed_output, ensure_extension};
use std::ffi::OsString;
use std::path::Path;

pub fn plan_mute(
    input: &Path,
    output: Option<&Path>,
    overwrite: bool,
) -> Result<ExecutionPlan, FfezError> {
    let input_ext = input.extension().and_then(|e| e.to_str()).unwrap_or("mp4");

    let final_output = match output {
        Some(out) => ensure_extension(out, input_ext),
        None => derive_suffixed_output(input, "muted", input_ext)?,
    };

    if final_output == input {
        return Err(FfezError::InputOutputSame(final_output));
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

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
    fn test_mute_appends_input_extension_when_output_has_none() {
        let input = Path::new("clips/gameplay.mkv");
        let custom_out = Path::new("clips/silent_version");
        let plan = plan_mute(input, Some(custom_out), false).unwrap();

        assert_eq!(plan.final_output, PathBuf::from("clips/silent_version.mkv"));
    }

    #[test]
    fn test_rejects_output_identical_to_input() {
        let input = Path::new("video.mp4");
        // Even if user passes `-o video` without extension, `ensure_extension` resolves it
        // to `video.mp4` and catches the self-overwrite!
        let err = plan_mute(input, Some(Path::new("video")), true).unwrap_err();
        assert!(matches!(err, FfezError::InputOutputSame(_)));
    }

    #[test]
    fn test_mute_with_extensionless_input() {
        let input = Path::new("myvideo");
        let plan = plan_mute(input, None, false).unwrap();
        assert_eq!(plan.final_output, PathBuf::from("myvideo_muted.mp4"));
    }

    #[test]
    fn test_mute_with_nested_directories_and_spaces() {
        let input = Path::new("clips/vacation 2026/beach sunset.mkv");
        let plan = plan_mute(input, None, true).unwrap();
        assert_eq!(
            plan.final_output,
            PathBuf::from("clips/vacation 2026/beach sunset_muted.mkv")
        );
        assert!(plan.overwrite);
    }

    #[test]
    fn test_mute_with_unicode_characters() {
        let input = Path::new("vidéo_été_2026.mp4");
        let plan = plan_mute(input, None, false).unwrap();
        assert_eq!(plan.final_output, PathBuf::from("vidéo_été_2026_muted.mp4"));
    }

    #[test]
    fn test_mute_with_multiple_dots_in_filename() {
        let input = Path::new("backup.archive.v1.mp4");
        let plan = plan_mute(input, None, false).unwrap();
        assert_eq!(
            plan.final_output,
            PathBuf::from("backup.archive.v1_muted.mp4")
        );
    }

    #[test]
    fn test_mute_ffmpeg_args_structure() {
        let input = Path::new("input.mov");
        let plan = plan_mute(input, None, false).unwrap();

        let expected_args: Vec<OsString> = vec![
            "-hide_banner".into(),
            "-loglevel".into(),
            "error".into(),
            "-i".into(),
            "input.mov".into(),
            "-c:v".into(),
            "copy".into(),
            "-an".into(),
            "-movflags".into(),
            "+faststart".into(),
        ];
        assert_eq!(plan.ffmpeg_args, expected_args);
    }
}
