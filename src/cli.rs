use bytesize::ByteSize;
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

    /// Compress a video to a target file size
    #[command(
        about = "Compress a video to a target file size",
        long_about = "Re-encodes a video so the output file lands under the requested size.\n\n\
                      Probes the duration with ffprobe, reserves bitrate for AAC audio, and\n\
                      spends the rest on a two-pass H.264 encode. A small share of --size is\n\
                      held back so container overhead still fits under the cap. When the\n\
                      bitrate would be too low for the current resolution, the picture is\n\
                      scaled down. --dry-run probes the input, so the file has to exist."
    )]
    Compress {
        /// Input video file
        input: PathBuf,

        /// Target file size (for example 25M, 15MB, 800KB)
        #[arg(short, long, value_parser = parse_target_size)]
        size: ByteSize,

        /// Optional output file path (defaults to <input>_compressed.mp4)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Overwrite output file if it already exists
        #[arg(short = 'y', long)]
        overwrite: bool,
    },
}

fn parse_target_size(raw: &str) -> Result<ByteSize, String> {
    let size = raw
        .parse::<ByteSize>()
        .map_err(|_| format!("invalid size '{raw}'. Expected a value like 25M, 15MB, or 800KB"))?;
    if size.as_u64() == 0 {
        return Err("target size must be greater than zero".to_string());
    }
    Ok(size)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_format_extensions() {
        assert_eq!(AudioFormat::Mp3.extension(), "mp3");
        assert_eq!(AudioFormat::Wav.extension(), "wav");
        assert_eq!(AudioFormat::M4a.extension(), "m4a");
        assert_eq!(AudioFormat::Flac.extension(), "flac");
    }

    #[test]
    fn test_audio_format_from_extension() {
        // Lowercase
        assert_eq!(AudioFormat::from_extension("mp3"), Some(AudioFormat::Mp3));
        assert_eq!(AudioFormat::from_extension("wav"), Some(AudioFormat::Wav));
        assert_eq!(AudioFormat::from_extension("m4a"), Some(AudioFormat::M4a));
        assert_eq!(AudioFormat::from_extension("aac"), Some(AudioFormat::M4a));
        assert_eq!(AudioFormat::from_extension("flac"), Some(AudioFormat::Flac));

        // Case-insensitivity
        assert_eq!(AudioFormat::from_extension("MP3"), Some(AudioFormat::Mp3));
        assert_eq!(AudioFormat::from_extension("Wav"), Some(AudioFormat::Wav));
        assert_eq!(AudioFormat::from_extension("M4A"), Some(AudioFormat::M4a));
        assert_eq!(AudioFormat::from_extension("AAC"), Some(AudioFormat::M4a));
        assert_eq!(AudioFormat::from_extension("FLAC"), Some(AudioFormat::Flac));

        // Unsupported or invalid extensions
        assert_eq!(AudioFormat::from_extension("ogg"), None);
        assert_eq!(AudioFormat::from_extension("opus"), None);
        assert_eq!(AudioFormat::from_extension("mp4"), None);
        assert_eq!(AudioFormat::from_extension(""), None);
    }

    #[test]
    fn test_parse_mute_minimal() {
        let cli = Cli::try_parse_from(["ffez", "mute", "clip.mp4"]).unwrap();
        assert!(!cli.dry_run);
        match cli.command {
            Commands::Mute {
                input,
                output,
                overwrite,
            } => {
                assert_eq!(input, PathBuf::from("clip.mp4"));
                assert_eq!(output, None);
                assert!(!overwrite);
            }
            _ => panic!("Expected Commands::Mute"),
        }
    }

    #[test]
    fn test_parse_mute_with_all_options() {
        let cli = Cli::try_parse_from(["ffez", "mute", "clip.mp4", "-o", "out_silent.mp4", "-y"])
            .unwrap();

        match cli.command {
            Commands::Mute {
                input,
                output,
                overwrite,
            } => {
                assert_eq!(input, PathBuf::from("clip.mp4"));
                assert_eq!(output, Some(PathBuf::from("out_silent.mp4")));
                assert!(overwrite);
            }
            _ => panic!("Expected Commands::Mute"),
        }
    }

    #[test]
    fn test_parse_mute_long_overwrite_flag() {
        let cli = Cli::try_parse_from(["ffez", "mute", "clip.mp4", "--overwrite"]).unwrap();
        match cli.command {
            Commands::Mute { overwrite, .. } => assert!(overwrite),
            _ => panic!("Expected Commands::Mute"),
        }
    }

    #[test]
    fn test_parse_audio_minimal() {
        let cli = Cli::try_parse_from(["ffez", "audio", "video.mkv"]).unwrap();
        assert!(!cli.dry_run);
        match cli.command {
            Commands::Audio {
                input,
                format,
                output,
                overwrite,
            } => {
                assert_eq!(input, PathBuf::from("video.mkv"));
                assert_eq!(format, None);
                assert_eq!(output, None);
                assert!(!overwrite);
            }
            _ => panic!("Expected Commands::Audio"),
        }
    }

    #[test]
    fn test_parse_audio_all_formats() {
        let formats = [
            ("mp3", AudioFormat::Mp3),
            ("wav", AudioFormat::Wav),
            ("m4a", AudioFormat::M4a),
            ("flac", AudioFormat::Flac),
        ];

        for (format_str, expected_format) in formats {
            let cli = Cli::try_parse_from([
                "ffez",
                "audio",
                "video.mp4",
                "-f",
                format_str,
                "-o",
                "audio_out",
                "-y",
            ])
            .unwrap();

            match cli.command {
                Commands::Audio {
                    input,
                    format,
                    output,
                    overwrite,
                } => {
                    assert_eq!(input, PathBuf::from("video.mp4"));
                    assert_eq!(format, Some(expected_format));
                    assert_eq!(output, Some(PathBuf::from("audio_out")));
                    assert!(overwrite);
                }
                _ => panic!("Expected Commands::Audio"),
            }
        }
    }

    #[test]
    fn test_parse_dry_run_global_flag() {
        // Dry run before subcommand
        let cli1 = Cli::try_parse_from(["ffez", "--dry-run", "mute", "video.mp4"]).unwrap();
        assert!(cli1.dry_run);

        // Dry run after subcommand
        let cli2 = Cli::try_parse_from(["ffez", "mute", "video.mp4", "--dry-run"]).unwrap();
        assert!(cli2.dry_run);

        // Dry run on audio command
        let cli3 = Cli::try_parse_from(["ffez", "audio", "video.mp4", "--dry-run"]).unwrap();
        assert!(cli3.dry_run);
    }

    #[test]
    fn test_parse_missing_subcommand_fails() {
        assert!(Cli::try_parse_from(["ffez"]).is_err());
    }

    #[test]
    fn test_parse_missing_input_fails() {
        assert!(Cli::try_parse_from(["ffez", "mute"]).is_err());
        assert!(Cli::try_parse_from(["ffez", "audio"]).is_err());
    }

    #[test]
    fn test_parse_invalid_audio_format_fails() {
        assert!(Cli::try_parse_from(["ffez", "audio", "video.mp4", "-f", "ogg"]).is_err());
        assert!(Cli::try_parse_from(["ffez", "audio", "video.mp4", "-f", "opus"]).is_err());
    }

    #[test]
    fn test_parse_unknown_flag_fails() {
        assert!(Cli::try_parse_from(["ffez", "mute", "video.mp4", "--unknown-flag"]).is_err());
    }

    #[test]
    fn test_parse_compress_size_units() {
        let cases = [
            ("25M", 25_000_000),
            ("15MB", 15_000_000),
            ("800KB", 800_000),
            ("1.5KB", 1_500),
        ];

        for (raw, bytes) in cases {
            let cli = Cli::try_parse_from([
                "ffez",
                "compress",
                "clip.mp4",
                "--size",
                raw,
                "-o",
                "small.mp4",
                "-y",
            ])
            .unwrap();
            match cli.command {
                Commands::Compress {
                    input,
                    size,
                    output,
                    overwrite,
                } => {
                    assert_eq!(input, PathBuf::from("clip.mp4"));
                    assert_eq!(size.as_u64(), bytes);
                    assert_eq!(output, Some(PathBuf::from("small.mp4")));
                    assert!(overwrite);
                }
                _ => panic!("Expected Commands::Compress"),
            }
        }
    }

    #[test]
    fn test_parse_compress_short_size_flag() {
        let cli = Cli::try_parse_from(["ffez", "compress", "clip.mp4", "-s", "25M"]).unwrap();
        match cli.command {
            Commands::Compress { size, output, .. } => {
                assert_eq!(size.as_u64(), 25_000_000);
                assert_eq!(output, None);
            }
            _ => panic!("Expected Commands::Compress"),
        }
    }

    #[test]
    fn test_parse_compress_requires_a_positive_size() {
        assert!(Cli::try_parse_from(["ffez", "compress", "clip.mp4"]).is_err());
        assert!(Cli::try_parse_from(["ffez", "compress", "--size", "25M"]).is_err());
        assert!(Cli::try_parse_from(["ffez", "compress", "clip.mp4", "--size", "nope"]).is_err());
        assert!(Cli::try_parse_from(["ffez", "compress", "clip.mp4", "--size", "0"]).is_err());
        assert!(Cli::try_parse_from(["ffez", "compress", "clip.mp4", "--size", "0B"]).is_err());
    }
}
