use crate::cli::AudioFormat;
use crate::error::FfezError;
use crate::planner;
use crate::runner;
use std::path::Path;

pub fn run(
    input: &Path,
    output: Option<&Path>,
    format: Option<AudioFormat>,
    overwrite: bool,
    dry_run: bool,
) -> Result<(), FfezError> {
    let plan = planner::plan_audio(input, output, format, overwrite)?;

    if dry_run {
        println!("{}", runner::format_dry_run(&plan));
        return Ok(());
    }

    runner::execute(&plan)?;
    println!("✔ Audio saved to: {}", plan.final_output.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_audio_command_dry_run() {
        // dry run should succeed even if input does not exist on disk
        let res = run(Path::new("dummy.mp4"), None, None, false, true);
        assert!(res.is_ok());
    }
}
