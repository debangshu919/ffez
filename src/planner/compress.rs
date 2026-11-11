use crate::error::FfezError;
use crate::planner::utils::{derive_suffixed_output, ensure_extension};
use bytesize::ByteSize;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// Picture and timing facts `ffprobe` reports for the input.
#[derive(Debug, Clone, PartialEq)]
pub struct VideoSource {
    pub duration_secs: f64,
    pub has_audio: bool,
    pub width: u32,
    pub height: u32,
    /// `0` when the frame rate is missing or unknown.
    pub fps: f64,
}

/// Two ffmpeg invocations that encode `input` toward a file-size cap.
#[derive(Debug, Clone, PartialEq)]
pub struct CompressPlan {
    pub input: PathBuf,
    pub final_output: PathBuf,
    pub overwrite: bool,
    pub video_bps: u64,
    pub audio_bps: Option<u64>,
    pub scale_height: Option<u32>,
    /// Pass 1, including the null-muxer output. The runner inserts `-passlogfile`.
    pub pass1_args: Vec<OsString>,
    /// Pass 2, without the destination path. The runner appends the temp file.
    pub pass2_args: Vec<OsString>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BitrateBudget {
    video_bps: u64,
    audio_bps: Option<u64>,
}

/// Share of the requested size left for audio and video. The rest covers the
/// container and the small miss two-pass rate control still has.
const SIZE_SAFETY: f64 = 0.95;
const MIN_VIDEO_BPS: u64 = 64_000;
const AUDIO_BITRATES_BPS: [u64; 3] = [128_000, 96_000, 64_000];
/// Below this, the same bitrate looks better on a smaller frame.
const MIN_BITS_PER_PIXEL: f64 = 0.07;
const HEIGHT_LADDER: [u32; 5] = [1080, 720, 480, 360, 240];

pub fn compress_output_path(input: &Path, output: Option<&Path>) -> Result<PathBuf, FfezError> {
    let final_output = match output {
        Some(out) => ensure_extension(out, "mp4"),
        None => derive_suffixed_output(input, "compressed", "mp4")?,
    };

    if final_output == input {
        return Err(FfezError::InputOutputSame(final_output));
    }

    Ok(final_output)
}

pub fn plan_compress(
    input: &Path,
    output: Option<&Path>,
    overwrite: bool,
    size: ByteSize,
    source: &VideoSource,
) -> Result<CompressPlan, FfezError> {
    let final_output = compress_output_path(input, output)?;
    let budget =
        allocate_bitrate(size.as_u64(), source.duration_secs, source.has_audio).map_err(|err| {
            match err {
                BudgetError::Empty => FfezError::EmptyTarget,
                BudgetError::BadDuration => FfezError::UnknownDuration(input.to_path_buf()),
                BudgetError::TooSmall => FfezError::TargetTooSmall {
                    target: format_si(size.as_u64()),
                    duration_secs: source.duration_secs,
                    minimum: format_si(minimum_target_bytes(
                        source.duration_secs,
                        source.has_audio,
                    )),
                },
            }
        })?;

    let scale_height = scale_height(source.width, source.height, source.fps, budget.video_bps);
    let (pass1_args, pass2_args) = encode_args(
        input,
        &final_output,
        budget.video_bps,
        budget.audio_bps,
        scale_height,
        source.fps,
    );

    Ok(CompressPlan {
        input: input.to_path_buf(),
        final_output,
        overwrite,
        video_bps: budget.video_bps,
        audio_bps: budget.audio_bps,
        scale_height,
        pass1_args,
        pass2_args,
    })
}

#[derive(Debug, PartialEq)]
enum BudgetError {
    Empty,
    BadDuration,
    TooSmall,
}

fn allocate_bitrate(
    target_bytes: u64,
    duration_secs: f64,
    has_audio: bool,
) -> Result<BitrateBudget, BudgetError> {
    if target_bytes == 0 {
        return Err(BudgetError::Empty);
    }
    if !(duration_secs.is_finite() && duration_secs > 0.0) {
        return Err(BudgetError::BadDuration);
    }

    let total_bps = (target_bytes as f64) * 8.0 * SIZE_SAFETY / duration_secs;
    if !total_bps.is_finite() {
        return Err(BudgetError::BadDuration);
    }

    if !has_audio {
        let video_bps = total_bps.floor() as u64;
        if video_bps < MIN_VIDEO_BPS {
            return Err(BudgetError::TooSmall);
        }
        return Ok(BitrateBudget {
            video_bps,
            audio_bps: None,
        });
    }

    for audio_bps in AUDIO_BITRATES_BPS {
        let video_bps = total_bps - audio_bps as f64;
        if video_bps >= MIN_VIDEO_BPS as f64 {
            return Ok(BitrateBudget {
                video_bps: video_bps.floor() as u64,
                audio_bps: Some(audio_bps),
            });
        }
    }

    Err(BudgetError::TooSmall)
}

fn minimum_target_bytes(duration_secs: f64, has_audio: bool) -> u64 {
    let audio_bps = if has_audio {
        AUDIO_BITRATES_BPS[AUDIO_BITRATES_BPS.len() - 1]
    } else {
        0
    };
    let min_bps = MIN_VIDEO_BPS + audio_bps;
    let raw = (min_bps as f64) * duration_secs / (8.0 * SIZE_SAFETY);
    if !raw.is_finite() || raw <= 0.0 {
        return 1;
    }

    let mut bytes = raw.ceil() as u64;
    for _ in 0..8 {
        if allocate_bitrate(bytes, duration_secs, has_audio).is_ok() {
            return bytes;
        }
        bytes = bytes.saturating_add(bytes / 1000 + 1);
    }
    bytes
}

/// Largest even height on the ladder whose frame still clears `MIN_BITS_PER_PIXEL`.
/// Returns the smallest rung when nothing clears it, and `None` when the source
/// is already small enough or the frame rate is unknown.
fn scale_height(width: u32, height: u32, fps: f64, video_bps: u64) -> Option<u32> {
    if width == 0 || height == 0 || video_bps == 0 || !(fps.is_finite() && fps > 0.0) {
        return None;
    }
    if bits_per_pixel(width, height, fps, video_bps) >= MIN_BITS_PER_PIXEL {
        return None;
    }

    let mut smallest_fit: Option<u32> = None;
    for rung in HEIGHT_LADDER {
        if rung >= height {
            continue;
        }
        let scaled_w = even_width(width, height, rung);
        if scaled_w < 2 {
            continue;
        }
        smallest_fit = Some(rung);
        if bits_per_pixel(scaled_w, rung, fps, video_bps) >= MIN_BITS_PER_PIXEL {
            return Some(rung);
        }
    }
    smallest_fit
}

fn even_width(width: u32, height: u32, target_h: u32) -> u32 {
    let exact = (width as f64) * (target_h as f64) / (height as f64);
    let rounded = exact.round() as u32;
    (rounded & !1).max(2)
}

fn bits_per_pixel(width: u32, height: u32, fps: f64, video_bps: u64) -> f64 {
    video_bps as f64 / (width as f64 * height as f64 * fps)
}

fn encode_args(
    input: &Path,
    output: &Path,
    video_bps: u64,
    audio_bps: Option<u64>,
    scale_height: Option<u32>,
    fps: f64,
) -> (Vec<OsString>, Vec<OsString>) {
    let mut shared = vec![
        OsString::from("-nostdin"),
        OsString::from("-hide_banner"),
        OsString::from("-loglevel"),
        OsString::from("error"),
        OsString::from("-stats"),
        OsString::from("-i"),
        input.as_os_str().to_os_string(),
        OsString::from("-map"),
        OsString::from("0:v:0"),
        OsString::from("-c:v"),
        OsString::from("libx264"),
        OsString::from("-preset"),
        OsString::from("medium"),
        OsString::from("-pix_fmt"),
        OsString::from("yuv420p"),
        OsString::from("-b:v"),
        OsString::from(video_bps.to_string()),
    ];
    // Variable-frame-rate phone video produces equal timestamps. The pass-1
    // null muxer rejects those, so both passes emit a constant frame rate.
    shared.push(OsString::from("-fps_mode"));
    shared.push(OsString::from("cfr"));
    if let Some(rate) = format_frame_rate(fps) {
        shared.push(OsString::from("-r"));
        shared.push(OsString::from(rate));
    }
    if let Some(height) = scale_height {
        shared.push(OsString::from("-vf"));
        shared.push(OsString::from(format!("scale=-2:{height}")));
    }

    let mut pass1 = shared.clone();
    pass1.extend([
        OsString::from("-pass"),
        OsString::from("1"),
        OsString::from("-an"),
        OsString::from("-f"),
        OsString::from("null"),
        OsString::from("-"),
    ]);

    let mut pass2 = shared;
    pass2.push(OsString::from("-pass"));
    pass2.push(OsString::from("2"));
    match audio_bps {
        Some(bps) => {
            pass2.extend([
                OsString::from("-map"),
                OsString::from("0:a:0"),
                OsString::from("-c:a"),
                OsString::from("aac"),
                OsString::from("-b:a"),
                OsString::from(format!("{}k", bps / 1000)),
            ]);
        }
        None => pass2.push(OsString::from("-an")),
    }
    if container_uses_faststart(output) {
        pass2.push(OsString::from("-movflags"));
        pass2.push(OsString::from("+faststart"));
    }

    (pass1, pass2)
}

/// A rate the encoder can use without turning a variable-frame-rate clip into
/// tens of thousands of frames per second. Unknown or absurd rates are omitted.
fn format_frame_rate(fps: f64) -> Option<String> {
    if !(fps.is_finite() && (1.0..=240.0).contains(&fps)) {
        return None;
    }

    const KNOWN: [(f64, &str); 3] = [
        (24_000.0 / 1001.0, "24000/1001"),
        (30_000.0 / 1001.0, "30000/1001"),
        (60_000.0 / 1001.0, "60000/1001"),
    ];
    for (value, label) in KNOWN {
        if (fps - value).abs() < 0.01 {
            return Some(label.to_string());
        }
    }

    let rounded = fps.round();
    if (fps - rounded).abs() < 0.01 {
        return Some(format!("{}", rounded as u32));
    }
    Some(format!("{fps:.3}"))
}

fn container_uses_faststart(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| matches!(ext.to_ascii_lowercase().as_str(), "mp4" | "m4v" | "mov"))
}

fn format_si(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1000.0 && unit < UNITS.len() - 1 {
        value /= 1000.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(duration: f64, audio: bool, width: u32, height: u32, fps: f64) -> VideoSource {
        VideoSource {
            duration_secs: duration,
            has_audio: audio,
            width,
            height,
            fps,
        }
    }

    #[test]
    fn test_default_output_is_mp4() {
        let plan = plan_compress(
            Path::new("clips/holiday.mkv"),
            None,
            false,
            ByteSize::mb(25),
            &source(60.0, true, 320, 240, 24.0),
        )
        .unwrap();
        assert_eq!(
            plan.final_output,
            PathBuf::from("clips/holiday_compressed.mp4")
        );
        assert!(
            plan.pass2_args
                .windows(2)
                .any(|pair| { pair[0] == "-movflags" && pair[1] == "+faststart" })
        );
    }

    #[test]
    fn test_custom_mkv_output_skips_faststart() {
        let plan = plan_compress(
            Path::new("clip.mp4"),
            Some(Path::new("out/small.mkv")),
            true,
            ByteSize::mb(25),
            &source(60.0, false, 320, 240, 24.0),
        )
        .unwrap();
        assert_eq!(plan.final_output, PathBuf::from("out/small.mkv"));
        assert!(plan.overwrite);
        assert!(!plan.pass2_args.iter().any(|arg| arg == "-movflags"));
        assert_eq!(plan.audio_bps, None);
    }

    #[test]
    fn test_extensionless_output_becomes_mp4() {
        let path =
            compress_output_path(Path::new("clip.mp4"), Some(Path::new("renders/small"))).unwrap();
        assert_eq!(path, PathBuf::from("renders/small.mp4"));
    }

    #[test]
    fn test_rejects_output_identical_to_input() {
        let err =
            compress_output_path(Path::new("video.mp4"), Some(Path::new("video"))).unwrap_err();
        assert!(matches!(err, FfezError::InputOutputSame(_)));
    }

    #[test]
    fn test_twenty_five_megabyte_budget_for_one_minute() {
        let budget = allocate_bitrate(25_000_000, 60.0, true).unwrap();
        assert_eq!(budget.audio_bps, Some(128_000));
        assert_eq!(budget.video_bps, 3_038_666);

        let silent = allocate_bitrate(25_000_000, 60.0, false).unwrap();
        assert_eq!(silent.audio_bps, None);
        assert_eq!(silent.video_bps, 3_166_666);
    }

    #[test]
    fn test_audio_ladder_steps_down() {
        let mid = allocate_bitrate(236_842, 10.0, true).unwrap();
        assert_eq!(mid.audio_bps, Some(96_000));
        assert!(mid.video_bps >= MIN_VIDEO_BPS);

        let low = allocate_bitrate(184_210, 10.0, true).unwrap();
        assert_eq!(low.audio_bps, Some(64_000));
        assert!(low.video_bps >= MIN_VIDEO_BPS);
    }

    #[test]
    fn test_rejects_empty_and_impossible_targets() {
        assert_eq!(
            allocate_bitrate(0, 10.0, true).unwrap_err(),
            BudgetError::Empty
        );
        assert_eq!(
            allocate_bitrate(1_000, 0.0, true).unwrap_err(),
            BudgetError::BadDuration
        );
        assert_eq!(
            allocate_bitrate(1_000, f64::NAN, false).unwrap_err(),
            BudgetError::BadDuration
        );
        assert_eq!(
            allocate_bitrate(1_000, 30.0, true).unwrap_err(),
            BudgetError::TooSmall
        );

        let err = plan_compress(
            Path::new("long.mp4"),
            None,
            false,
            ByteSize::kb(500),
            &source(3600.0, true, 1280, 720, 30.0),
        )
        .unwrap_err();
        let FfezError::TargetTooSmall {
            target,
            duration_secs,
            minimum,
        } = err
        else {
            panic!("expected TargetTooSmall");
        };
        assert_eq!(target, "500.0 KB");
        assert!((duration_secs - 3600.0).abs() < f64::EPSILON);
        assert_eq!(minimum, "60.6 MB");

        let minimum_bytes = minimum_target_bytes(3600.0, true);
        assert!(allocate_bitrate(minimum_bytes, 3600.0, true).is_ok());
        assert!(allocate_bitrate(minimum_bytes - 1, 3600.0, true).is_err());
    }

    #[test]
    fn test_scale_picks_the_largest_height_that_holds_detail() {
        assert_eq!(scale_height(1920, 1080, 30.0, 3_038_666), Some(720));
        assert_eq!(scale_height(1920, 1080, 30.0, 500_000), Some(360));
        assert_eq!(scale_height(1920, 1080, 30.0, 200_000), Some(240));
        assert_eq!(scale_height(1920, 1080, 30.0, 8_000_000), None);
        assert_eq!(scale_height(1920, 1080, 0.0, 200_000), None);
        assert_eq!(scale_height(160, 100, 30.0, 1_000), None);
    }

    #[test]
    fn test_plan_args_for_a_sized_encode() {
        let plan = plan_compress(
            Path::new("clip.mp4"),
            None,
            false,
            ByteSize::mb(25),
            &source(60.0, true, 320, 240, 24.0),
        )
        .unwrap();

        assert_eq!(plan.video_bps, 3_038_666);
        assert_eq!(plan.audio_bps, Some(128_000));
        assert_eq!(plan.scale_height, None);
        assert!(
            plan.pass1_args
                .windows(2)
                .any(|pair| pair[0] == "-pass" && pair[1] == "1")
        );
        assert!(
            plan.pass1_args
                .windows(2)
                .any(|pair| pair[0] == "-f" && pair[1] == "null")
        );
        assert_eq!(
            plan.pass1_args.last().map(OsString::as_os_str),
            Some(std::ffi::OsStr::new("-"))
        );
        assert!(
            plan.pass2_args
                .windows(2)
                .any(|pair| pair[0] == "-b:v" && pair[1] == "3038666")
        );
        assert!(
            plan.pass2_args
                .windows(2)
                .any(|pair| pair[0] == "-b:a" && pair[1] == "128k")
        );
        assert!(
            plan.pass2_args
                .windows(2)
                .any(|pair| pair[0] == "-pass" && pair[1] == "2")
        );
    }

    #[test]
    fn test_both_passes_use_a_constant_frame_rate() {
        let plan = plan_compress(
            Path::new("clip.mp4"),
            None,
            false,
            ByteSize::mb(25),
            &source(60.0, true, 1920, 1080, 30_000.0 / 1001.0),
        )
        .unwrap();

        for args in [&plan.pass1_args, &plan.pass2_args] {
            assert!(
                args.windows(2)
                    .any(|pair| pair[0] == "-fps_mode" && pair[1] == "cfr")
            );
            assert!(
                args.windows(2)
                    .any(|pair| pair[0] == "-r" && pair[1] == "30000/1001")
            );
        }

        let unknown = plan_compress(
            Path::new("clip.mp4"),
            None,
            false,
            ByteSize::mb(25),
            &source(60.0, true, 320, 240, 0.0),
        )
        .unwrap();
        assert!(
            unknown
                .pass1_args
                .windows(2)
                .any(|pair| pair[0] == "-fps_mode" && pair[1] == "cfr")
        );
        assert!(!unknown.pass1_args.iter().any(|arg| arg == "-r"));
        assert!(!unknown.pass2_args.iter().any(|arg| arg == "-r"));
    }

    #[test]
    fn test_plan_scales_a_large_frame() {
        let plan = plan_compress(
            Path::new("clip.mp4"),
            None,
            false,
            ByteSize::mb(25),
            &source(60.0, true, 1920, 1080, 30.0),
        )
        .unwrap();
        assert_eq!(plan.scale_height, Some(720));
        let filter = OsString::from("scale=-2:720");
        assert!(plan.pass1_args.contains(&filter));
        assert!(plan.pass2_args.contains(&filter));
    }
}
