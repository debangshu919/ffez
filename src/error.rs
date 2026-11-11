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

    #[error(
        "ffprobe not found in PATH. It ships with FFmpeg. Install it via `winget install Gyan.FFmpeg`, `brew install ffmpeg`, or your package manager."
    )]
    MissingFfprobe,

    #[error("ffprobe failed (exit code {code})")]
    FfprobeFailed { code: i32, stderr: String },

    #[error("No video stream found in '{0}'.")]
    NoVideoStream(PathBuf),

    #[error("Could not read a duration from '{0}'.")]
    UnknownDuration(PathBuf),

    #[error("Target size must be greater than zero.")]
    EmptyTarget,

    #[error(
        "Target size {target} is too small for a {duration_secs:.1}s video. Smallest useful size is about {minimum}."
    )]
    TargetTooSmall {
        target: String,
        duration_secs: f64,
        minimum: String,
    },

    #[error("Could not fit the video under {target} even at the lowest quality (CRF {crf:.0}).")]
    QualityExceedsTarget { target: String, crf: f64 },
}

impl FfezError {
    pub fn ffmpeg_stderr(&self) -> Option<&str> {
        let stderr = match self {
            FfezError::FfmpegFailed { stderr, .. } | FfezError::FfprobeFailed { stderr, .. } => {
                stderr
            }
            _ => return None,
        };
        if stderr.trim().is_empty() {
            None
        } else {
            Some(stderr)
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

    #[test]
    fn test_ffprobe_errors_expose_stderr_and_messages() {
        let missing = FfezError::MissingFfprobe;
        assert!(missing.to_string().contains("ffprobe not found in PATH"));
        assert_eq!(missing.ffmpeg_stderr(), None);

        let failed = FfezError::FfprobeFailed {
            code: 1,
            stderr: "Invalid data found when processing input\n".into(),
        };
        assert_eq!(failed.to_string(), "ffprobe failed (exit code 1)");
        assert_eq!(
            failed.ffmpeg_stderr(),
            Some("Invalid data found when processing input\n")
        );

        let empty = FfezError::FfprobeFailed {
            code: 1,
            stderr: " \n".into(),
        };
        assert_eq!(empty.ffmpeg_stderr(), None);
    }

    #[test]
    fn test_compress_input_errors() {
        let path = PathBuf::from("clip.mp4");
        assert_eq!(
            FfezError::NoVideoStream(path.clone()).to_string(),
            "No video stream found in 'clip.mp4'."
        );
        assert_eq!(
            FfezError::UnknownDuration(path).to_string(),
            "Could not read a duration from 'clip.mp4'."
        );
        assert_eq!(
            FfezError::EmptyTarget.to_string(),
            "Target size must be greater than zero."
        );
    }

    #[test]
    fn test_target_too_small_message() {
        let err = FfezError::TargetTooSmall {
            target: "500.0 KB".into(),
            duration_secs: 3600.0,
            minimum: "60.6 MB".into(),
        };
        assert_eq!(
            err.to_string(),
            "Target size 500.0 KB is too small for a 3600.0s video. Smallest useful size is about 60.6 MB."
        );
        assert_eq!(err.ffmpeg_stderr(), None);
    }

    #[test]
    fn test_quality_exceeds_target_message() {
        let err = FfezError::QualityExceedsTarget {
            target: "8.0 MB".into(),
            crf: 35.0,
        };
        assert_eq!(
            err.to_string(),
            "Could not fit the video under 8.0 MB even at the lowest quality (CRF 35)."
        );
        assert_eq!(err.ffmpeg_stderr(), None);
    }
}
