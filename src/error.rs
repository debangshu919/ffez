use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FfezError {
    #[error(
        "FFmpeg not found in PATH. Install it via `winget install Gyan.FFmpeg`, `brew install ffmpeg`, or your package manager."
    )]
    MissingFfmpeg,

    #[error("Input file not found: '{0}'")]
    InputNotFound(PathBuf),

    #[error("Output file already exists: '{0}'. Pass --overwrite (-y) to replace it.")]
    OutputExists(PathBuf),

    #[error("Invalid file path: '{0}'")]
    InvalidPath(PathBuf),

    #[error("FFmpeg failed (exit code {code})")]
    FfmpegFailed { code: i32, stderr: String },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Output path cannot be the same as input path: '{0}'")]
    InputOutputSame(PathBuf),
}

impl FfezError {
    pub fn ffmpeg_stderr(&self) -> Option<&str> {
        match self {
            FfezError::FfmpegFailed { stderr, .. } if !stderr.trim().is_empty() => Some(stderr),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_missing_ffmpeg_message() {
        let err = FfezError::MissingFfmpeg;
        let msg = err.to_string();
        assert!(msg.contains("FFmpeg not found in PATH"));
        assert!(msg.contains("winget install Gyan.FFmpeg"));
        assert_eq!(err.ffmpeg_stderr(), None);
    }

    #[test]
    fn test_input_not_found_message() {
        let path = PathBuf::from("ghost.mp4");
        let err = FfezError::InputNotFound(path);
        assert_eq!(err.to_string(), "Input file not found: 'ghost.mp4'");
        assert_eq!(err.ffmpeg_stderr(), None);
    }

    #[test]
    fn test_output_exists_message() {
        let path = PathBuf::from("already_here.mp4");
        let err = FfezError::OutputExists(path);
        assert_eq!(
            err.to_string(),
            "Output file already exists: 'already_here.mp4'. Pass --overwrite (-y) to replace it."
        );
        assert_eq!(err.ffmpeg_stderr(), None);
    }

    #[test]
    fn test_invalid_path_message() {
        let path = PathBuf::from("bad/path");
        let err = FfezError::InvalidPath(path);
        assert_eq!(err.to_string(), "Invalid file path: 'bad/path'");
        assert_eq!(err.ffmpeg_stderr(), None);
    }

    #[test]
    fn test_ffmpeg_failed_message_and_stderr() {
        let err_with_stderr = FfezError::FfmpegFailed {
            code: 1,
            stderr: "Error while decoding stream #0:0\n".into(),
        };
        assert_eq!(err_with_stderr.to_string(), "FFmpeg failed (exit code 1)");
        assert_eq!(
            err_with_stderr.ffmpeg_stderr(),
            Some("Error while decoding stream #0:0\n")
        );

        // Empty stderr
        let err_empty = FfezError::FfmpegFailed {
            code: 1,
            stderr: "".into(),
        };
        assert_eq!(err_empty.ffmpeg_stderr(), None);

        // Whitespace-only stderr
        let err_whitespace = FfezError::FfmpegFailed {
            code: 1,
            stderr: "   \n\t  ".into(),
        };
        assert_eq!(err_whitespace.ffmpeg_stderr(), None);
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let err = FfezError::from(io_err);
        assert_eq!(err.to_string(), "I/O error: access denied");
        assert_eq!(err.ffmpeg_stderr(), None);
    }

    #[test]
    fn test_input_output_same_message() {
        let path = PathBuf::from("same.mp4");
        let err = FfezError::InputOutputSame(path);
        assert_eq!(
            err.to_string(),
            "Output path cannot be the same as input path: 'same.mp4'"
        );
        assert_eq!(err.ffmpeg_stderr(), None);
    }
}
