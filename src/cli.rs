// use bytesize::ByteSize;
use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "ffez",
    version,
    about,
    long_about = "ffez is a fast, ergonomic FFmpeg CLI wrapper built for everyday media tasks.\n\
                  It simplifies common audio and video operations into clean, memorable commands\n\
                  so you don't have to remember complex FFmpeg flags.\n\n\
                  All operations write to a temporary file before moving to the final destination,\n\
                  protecting your files from partial or interrupted writes. Use --dry-run to inspect\n\
                  the exact FFmpeg command before executing it.",
    author
)]
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
    #[command(
        about = "Mute a video",
        long_about = "Removes all audio streams from a video file.\n\n\
                      Uses stream copy (-c:v copy) to instantly produce lossless video output\n\
                      without re-encoding."
    )]
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

    /// Extract the audio track from a video file
    #[command(
        about = "Extract the audio track from a video file",
        long_about = "Extracts audio from a video file into mp3, wav, m4a, or flac.\n\n\
                      If --format is omitted, it will be inferred from the output filename's\n\
                      extension (e.g., -o song.wav). If no extension is found or provided,\n\
                      it defaults to mp3."
    )]
    Audio {
        /// Input video file
        input: PathBuf,

        /// Audio format to extract into (inferred from --output if omitted, otherwise defaults to mp3)
        #[arg(short, long, value_enum)]
        format: Option<AudioFormat>,

        /// Optional output file path (defaults to <input>.<format>)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Overwrite output file if it already exists
        #[arg(short = 'y', long)]
        overwrite: bool,
    },
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioFormat {
    Mp3,
    Wav,
    M4a,
    Flac,
}

impl AudioFormat {
    pub fn extension(&self) -> &'static str {
        match self {
            AudioFormat::Mp3 => "mp3",
            AudioFormat::Wav => "wav",
            AudioFormat::M4a => "m4a",
            AudioFormat::Flac => "flac",
        }
    }

    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_ascii_lowercase().as_str() {
            "mp3" => Some(AudioFormat::Mp3),
            "wav" => Some(AudioFormat::Wav),
            "m4a" | "aac" => Some(AudioFormat::M4a),
            "flac" => Some(AudioFormat::Flac),
            _ => None,
        }
    }
}

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum Platform {
    Discord,
    Whatsapp,
}
