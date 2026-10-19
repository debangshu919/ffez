// use bytesize::ByteSize;
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "ffez", version, about = "ffmpeg made easy")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Print the underlying ffmpeg command without running it
    #[arg(long, global = true)]
    pub dry_run: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    // /// Compress a video to a target file size or platform preset
    // Compress {
    //     /// Input video file
    //     input: PathBuf,
    //
    //     /// Target size (e.g., "15MB", "25M", "800KB")
    //     #[arg(short, long, conflicts_with = "preset")]
    //     size: Option<ByteSize>,
    //
    //     /// Social platform preset
    //     #[arg(long = "for", value_enum, conflicts_with = "size")]
    //     preset: Option<Platform>,
    //
    //     /// output file path
    //     #[arg(short, long)]
    //     output: Option<PathBuf>,
    // },
    /// Mute a video
    Mute {
        /// Input video file
        input: PathBuf,

        /// Optional output file path (defaults to <input>_muted.<ext>)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Overwrite output file if it already exists
        #[arg(short = 'y', long)]
        overwrite: bool,
    },
}

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum Platform {
    Discord,
    Whatsapp,
}
