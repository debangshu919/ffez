use crate::error::FfezError;
use std::path::{Path, PathBuf};

/// Appends `default_ext` if `path` has no file extension
pub fn ensure_extension(path: &Path, default_ext: &str) -> PathBuf {
    match path.extension() {
        Some(ext) if !ext.is_empty() => path.to_path_buf(),
        _ => path.with_extension(default_ext),
    }
}

/// Converts `path/to/video.mp4` -> `path/to/video_{suffix}.{ext}`
pub fn derive_suffixed_output(input: &Path, suffix: &str, ext: &str) -> Result<PathBuf, FfezError> {
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| FfezError::InvalidPath(input.to_path_buf()))?;

    let parent = input.parent().unwrap_or_else(|| Path::new(""));
    Ok(parent.join(format!("{stem}_{suffix}.{ext}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ensure_extension_with_existing() {
        assert_eq!(
            ensure_extension(Path::new("video.mp4"), "mkv"),
            PathBuf::from("video.mp4")
        );
    }

    #[test]
    fn test_ensure_extension_without_existing() {
        assert_eq!(
            ensure_extension(Path::new("video"), "mp4"),
            PathBuf::from("video.mp4")
        );
    }

    #[test]
    fn test_derive_suffixed_output() {
        let path = Path::new("nested/video.mp4");
        let derived = derive_suffixed_output(path, "muted", "mp4").unwrap();
        assert_eq!(derived, PathBuf::from("nested/video_muted.mp4"));
    }
}
