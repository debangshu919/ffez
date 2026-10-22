pub mod cli;
pub mod commands;
pub mod error;
pub mod planner;
pub mod runner;

use cli::{Cli, Commands};
use error::FfezError;

pub fn run(cli: Cli) -> Result<(), FfezError> {
    match cli.command {
        Commands::Mute {
            input,
            output,
            overwrite,
        } => commands::mute::run(&input, output.as_deref(), overwrite, cli.dry_run),
        Commands::Audio {
            input,
            format,
            output,
            overwrite,
        } => commands::audio::run(&input, output.as_deref(), format, overwrite, cli.dry_run),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_run_mute_dry_run_succeeds_without_input_file() {
        let cli = Cli {
            command: Commands::Mute {
                input: PathBuf::from("nonexistent_video.mp4"),
                output: None,
                overwrite: false,
            },
            dry_run: true,
        };

        // dry_run does not execute or check input file existence
        assert!(run(cli).is_ok());
    }

    #[test]
    fn test_run_audio_dry_run_succeeds_without_input_file() {
        let cli = Cli {
            command: Commands::Audio {
                input: PathBuf::from("nonexistent_video.mp4"),
                format: None,
                output: None,
                overwrite: false,
            },
            dry_run: true,
        };

        assert!(run(cli).is_ok());
    }

    #[test]
    fn test_run_fails_when_input_missing_and_not_dry_run() {
        let cli = Cli {
            command: Commands::Mute {
                input: PathBuf::from("ghost_video.mp4"),
                output: None,
                overwrite: false,
            },
            dry_run: false,
        };

        let err = run(cli).unwrap_err();
        assert!(matches!(err, FfezError::InputNotFound(_)));
    }

    #[test]
    fn test_run_fails_when_input_equals_output() {
        let cli = Cli {
            command: Commands::Mute {
                input: PathBuf::from("video.mp4"),
                output: Some(PathBuf::from("video.mp4")),
                overwrite: true,
            },
            dry_run: false,
        };

        let err = run(cli).unwrap_err();
        assert!(matches!(err, FfezError::InputOutputSame(_)));
    }
}
