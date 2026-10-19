pub mod cli;
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
        } => {
            let plan = planner::plan_mute(&input, output.as_deref(), overwrite)?;

            if cli.dry_run {
                println!("{}", runner::format_dry_run(&plan));
                return Ok(());
            }

            runner::execute(&plan)?;
            println!("✔ Muted video saved to: {}", plan.final_output.display());
            Ok(())
        }
    }
}
