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
}