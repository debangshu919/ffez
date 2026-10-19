use clap::Parser;
use ffez::cli::Cli;
use ffez::run;
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = Cli::parse();

    if let Err(err) = run(cli) {
        eprintln!("\x1b[1;31merror:\x1b[0m {err}");
        if let Some(stderr) = err.ffmpeg_stderr() {
            eprintln!("\n--- ffmpeg details ---\n{}", stderr.trim());
        }
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
