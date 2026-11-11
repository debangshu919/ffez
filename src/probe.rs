use crate::error::FfezError;
use crate::planner::VideoSource;
use crate::runner::check_ffprobe;
use std::path::Path;
use std::process::{Command, Stdio};

/// Reads duration, picture size, frame rate, and whether an audio stream exists.
pub fn probe(input: &Path) -> Result<VideoSource, FfezError> {
    check_ffprobe()?;

    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration:stream=codec_type,width,height,avg_frame_rate,duration",
            "-of",
            "default=noprint_wrappers=1",
        ])
        .arg(input)
        .stdin(Stdio::null())
        .output()?;

    if !output.status.success() {
        return Err(FfezError::FfprobeFailed {
            code: output.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    let text = String::from_utf8_lossy(&output.stdout);
    parse_ffprobe_output(&text).map_err(|err| match err {
        ProbeParseError::NoVideo => FfezError::NoVideoStream(input.to_path_buf()),
        ProbeParseError::NoDuration => FfezError::UnknownDuration(input.to_path_buf()),
    })
}

#[derive(Debug, PartialEq)]
enum ProbeParseError {
    NoVideo,
    NoDuration,
}

#[derive(Clone, Copy, PartialEq)]
enum StreamKind {
    Video,
    Audio,
    Other,
}

struct PartialStream {
    kind: StreamKind,
    width: Option<u32>,
    height: Option<u32>,
    fps: Option<f64>,
}

/// Parses `ffprobe -of default=noprint_wrappers=1` output.
///
/// The format duration is requested as well as each stream duration. The last
/// positive value wins, which is the container duration when ffprobe has one
/// and a stream duration when the container does not.
fn parse_ffprobe_output(text: &str) -> Result<VideoSource, ProbeParseError> {
    let mut current: Option<PartialStream> = None;
    let mut video: Option<(u32, u32, f64)> = None;
    let mut has_audio = false;
    let mut duration_secs: Option<f64> = None;

    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();

        if key == "codec_type" {
            finish_stream(current.take(), &mut video, &mut has_audio);
            current = Some(PartialStream {
                kind: match value {
                    "video" => StreamKind::Video,
                    "audio" => StreamKind::Audio,
                    _ => StreamKind::Other,
                },
                width: None,
                height: None,
                fps: None,
            });
            continue;
        }

        if key == "duration" {
            if let Some(secs) = parse_positive_f64(value) {
                duration_secs = Some(secs);
            }
            continue;
        }

        if let Some(stream) = current
            .as_mut()
            .filter(|stream| stream.kind == StreamKind::Video)
        {
            match key {
                "width" => stream.width = value.parse().ok().filter(|width| *width > 0),
                "height" => stream.height = value.parse().ok().filter(|height| *height > 0),
                "avg_frame_rate" => stream.fps = parse_rate(value),
                _ => {}
            }
        }
    }
    finish_stream(current.take(), &mut video, &mut has_audio);

    let Some((width, height, fps)) = video else {
        return Err(ProbeParseError::NoVideo);
    };
    let Some(duration_secs) = duration_secs else {
        return Err(ProbeParseError::NoDuration);
    };

    Ok(VideoSource {
        duration_secs,
        has_audio,
        width,
        height,
        fps,
    })
}

fn finish_stream(
    stream: Option<PartialStream>,
    video: &mut Option<(u32, u32, f64)>,
    has_audio: &mut bool,
) {
    let Some(stream) = stream else {
        return;
    };
    match stream.kind {
        StreamKind::Audio => *has_audio = true,
        StreamKind::Video if video.is_none() => {
            if let (Some(width), Some(height)) = (stream.width, stream.height) {
                *video = Some((width, height, stream.fps.unwrap_or(0.0)));
            }
        }
        StreamKind::Video | StreamKind::Other => {}
    }
}

fn parse_positive_f64(value: &str) -> Option<f64> {
    let parsed: f64 = value.parse().ok()?;
    if parsed.is_finite() && parsed > 0.0 {
        Some(parsed)
    } else {
        None
    }
}

fn parse_rate(value: &str) -> Option<f64> {
    let rate = if let Some((num, den)) = value.split_once('/') {
        let num: f64 = num.trim().parse().ok()?;
        let den: f64 = den.trim().parse().ok()?;
        num / den
    } else {
        value.parse().ok()?
    };
    if rate.is_finite() && rate > 0.0 {
        Some(rate)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;

    const VIDEO_AND_AUDIO: &str = "\
codec_type=video
width=320
height=240
avg_frame_rate=24/1
duration=2.000000
codec_type=audio
avg_frame_rate=0/0
duration=2.000000
duration=2.040000
";

    #[test]
    fn test_parse_video_and_audio_prefers_container_duration() {
        let source = parse_ffprobe_output(VIDEO_AND_AUDIO).unwrap();
        assert_eq!(source.width, 320);
        assert_eq!(source.height, 240);
        assert!((source.fps - 24.0).abs() < f64::EPSILON);
        assert!(source.has_audio);
        assert!((source.duration_secs - 2.04).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_video_only() {
        let text = "\
codec_type=video
width=1920
height=1080
avg_frame_rate=30000/1001
duration=10.000000
duration=10.500000
";
        let source = parse_ffprobe_output(text).unwrap();
        assert!(!source.has_audio);
        assert_eq!(source.width, 1920);
        assert_eq!(source.height, 1080);
        assert!((source.fps - (30_000.0 / 1001.0)).abs() < 1e-9);
        assert!((source.duration_secs - 10.5).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_ignores_audio_dimensions_and_zero_rate() {
        let text = "\
codec_type=audio
avg_frame_rate=0/0
width=1
height=1
duration=3
codec_type=video
width=640
height=360
avg_frame_rate=N/A
duration=3
";
        let source = parse_ffprobe_output(text).unwrap();
        assert!(source.has_audio);
        assert_eq!((source.width, source.height), (640, 360));
        assert_eq!(source.fps, 0.0);
    }

    #[test]
    fn test_parse_rejects_audio_only_and_missing_duration() {
        let audio_only = "codec_type=audio\navg_frame_rate=0/0\nduration=3\n";
        assert_eq!(
            parse_ffprobe_output(audio_only).unwrap_err(),
            ProbeParseError::NoVideo
        );

        let no_duration = "codec_type=video\nwidth=320\nheight=240\navg_frame_rate=24/1\n";
        assert_eq!(
            parse_ffprobe_output(no_duration).unwrap_err(),
            ProbeParseError::NoDuration
        );
    }

    #[test]
    fn test_probe_reads_a_synthetic_video() {
        if which::which("ffmpeg").is_err() || which::which("ffprobe").is_err() {
            return;
        }

        let dir = tempfile::tempdir().unwrap();
        let sample = dir.path().join("sample.mp4");
        let status = Command::new("ffmpeg")
            .args([
                "-y",
                "-nostdin",
                "-hide_banner",
                "-loglevel",
                "error",
                "-f",
                "lavfi",
                "-i",
                "testsrc=duration=1:size=320x240:rate=1",
                "-f",
                "lavfi",
                "-i",
                "sine=duration=1:frequency=440",
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
                "-c:a",
                "aac",
            ])
            .arg(&sample)
            .status()
            .unwrap();
        assert!(status.success());

        let source = probe(&sample).unwrap();
        assert!(source.has_audio);
        assert_eq!((source.width, source.height), (320, 240));
        assert!((source.fps - 1.0).abs() < 0.01);
        assert!((source.duration_secs - 1.0).abs() < 0.1);
    }

    #[test]
    fn test_probe_corrupt_file_returns_ffprobe_stderr() {
        if which::which("ffprobe").is_err() {
            return;
        }

        let dir = tempfile::tempdir().unwrap();
        let bad = dir.path().join("bad.mp4");
        fs::write(&bad, b"not a media file").unwrap();

        let err = probe(&bad).unwrap_err();
        assert!(matches!(err, FfezError::FfprobeFailed { .. }));
        assert!(err.ffmpeg_stderr().is_some());
    }
}
