use crate::error::FfezError;
use crate::planner;
use crate::probe;
use crate::runner;
use bytesize::ByteSize;
use std::path::Path;

pub fn run(
    input: &Path,
    output: Option<&Path>,
    size: ByteSize,
    overwrite: bool,
    dry_run: bool,
) -> Result<(), FfezError> {
    if size.as_u64() == 0 {
        return Err(FfezError::EmptyTarget);
    }

    let final_output = planner::compress_output_path(input, output)?;
    if !dry_run && final_output.exists() && !overwrite {
        return Err(FfezError::OutputExists(final_output));
    }
    if !input.exists() {
        return Err(FfezError::InputNotFound(input.to_path_buf()));
    }

    let source = probe::probe(input)?;
    let plan = planner::plan_compress(input, output, overwrite, size, &source)?;

    if dry_run {
        println!("{}", runner::format_compress_dry_run(&plan));
        return Ok(());
    }

    runner::execute_compress(&plan)?;
    println!(
        "✔ Compressed video saved to: {}",
        plan.final_output.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;

    #[test]
    fn test_missing_input_fails_before_probe() {
        let err = run(
            Path::new("missing-clip.mp4"),
            None,
            ByteSize::mb(25),
            false,
            true,
        )
        .unwrap_err();
        assert!(matches!(err, FfezError::InputNotFound(_)));
    }

    #[test]
    fn test_same_path_fails_before_probe() {
        let err = run(
            Path::new("video.mp4"),
            Some(Path::new("video.mp4")),
            ByteSize::mb(25),
            true,
            true,
        )
        .unwrap_err();
        assert!(matches!(err, FfezError::InputOutputSame(_)));
    }

    #[test]
    fn test_existing_output_fails_before_probe() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("clip.mp4");
        let output = dir.path().join("clip_compressed.mp4");
        File::create(&input).unwrap();
        File::create(&output).unwrap();

        let err = run(&input, Some(&output), ByteSize::mb(25), false, false).unwrap_err();
        assert!(matches!(err, FfezError::OutputExists(_)));
    }

    #[test]
    fn test_zero_size_fails_before_touching_the_file() {
        let err = run(Path::new("clip.mp4"), None, ByteSize::b(0), false, true).unwrap_err();
        assert!(matches!(err, FfezError::EmptyTarget));
    }
}
