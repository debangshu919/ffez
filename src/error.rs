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
