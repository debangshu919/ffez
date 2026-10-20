use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::Path;
use tempfile::tempdir;

/// Generates a valid 1-second video with both video and audio streams using FFmpeg's lavfi test sources.
/// Returns false if FFmpeg is not installed or if generation fails.
fn create_synthetic_video(output_path: &Path) -> bool {
    if which::which("ffmpeg").is_err() {
        return false;
    }

    let status = std::process::Command::new("ffmpeg")
        .args([
            "-y",
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
        .arg(output_path)
        .status();

    match status {
        Ok(s) => s.success() && output_path.exists(),
        Err(_) => false,
    }
}

/// Helper to ensure no leftover atomic temporary files remain in the target directory
fn assert_no_temp_files(dir: &Path) {
    let entries = fs::read_dir(dir).expect("Failed to read directory");
    for entry in entries {
        let entry = entry.expect("Invalid entry");
        let name = entry.file_name().to_string_lossy().into_owned();
        assert!(
            !name.starts_with(".ffez_tmp_"),
            "Leftover temporary file detected: {name}"
        );
    }
}

#[test]
fn test_e2e_mute_video() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("input_sample.mp4");
    let expected_output = dir.path().join("input_sample_muted.mp4");

    if !create_synthetic_video(&input) {
        eprintln!("Skipping test_e2e_mute_video: FFmpeg is not available in PATH.");
        return;
    }

    let mut cmd = Command::cargo_bin("ffez").unwrap();
    cmd.args(["mute", input.to_str().unwrap()])
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains("✔ Muted video saved to:"));

    assert!(
        expected_output.exists(),
        "Expected muted video was not created"
    );
    assert!(
        fs::metadata(&expected_output).unwrap().len() > 0,
        "Muted video file is empty"
    );

    assert_no_temp_files(dir.path());
}

#[test]
fn test_e2e_extract_audio_mp3_and_wav() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("podcast_raw.mp4");
    let expected_mp3 = dir.path().join("podcast_raw.mp3");
    let expected_wav = dir.path().join("podcast_raw.wav");

    if !create_synthetic_video(&input) {
        eprintln!("Skipping test_e2e_extract_audio_mp3_and_wav: FFmpeg is not available in PATH.");
        return;
    }

    // 1. Extract MP3
    let mut cmd_mp3 = Command::cargo_bin("ffez").unwrap();
    cmd_mp3
        .args(["audio", input.to_str().unwrap(), "-f", "mp3"])
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains("✔ Audio saved to:"));

    assert!(expected_mp3.exists(), "Expected MP3 file was not created");
    assert!(
        fs::metadata(&expected_mp3).unwrap().len() > 0,
        "MP3 file is empty"
    );

    // 2. Extract WAV
    let mut cmd_wav = Command::cargo_bin("ffez").unwrap();
    cmd_wav
        .args(["audio", input.to_str().unwrap(), "-f", "wav"])
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains("✔ Audio saved to:"));

    assert!(expected_wav.exists(), "Expected WAV file was not created");
    assert!(
        fs::metadata(&expected_wav).unwrap().len() > 0,
        "WAV file is empty"
    );

    assert_no_temp_files(dir.path());
}

#[test]
fn test_e2e_custom_output_and_overwrite_protection() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("lecture.mp4");
    let custom_output = dir.path().join("custom_muted_lecture.mp4");

    if !create_synthetic_video(&input) {
        eprintln!("Skipping test: FFmpeg is not available in PATH.");
        return;
    }

    // First run creates custom output
    let mut cmd1 = Command::cargo_bin("ffez").unwrap();
    cmd1.args([
        "mute",
        input.to_str().unwrap(),
        "-o",
        custom_output.to_str().unwrap(),
    ])
    .assert()
    .success()
    .code(0);

    assert!(custom_output.exists());

    // Second run without -y must fail because output already exists
    let mut cmd2 = Command::cargo_bin("ffez").unwrap();
    cmd2.args([
        "mute",
        input.to_str().unwrap(),
        "-o",
        custom_output.to_str().unwrap(),
    ])
    .assert()
    .failure()
    .code(1)
    .stderr(predicate::str::contains("Output file already exists"));

    // Third run with -y must overwrite successfully
    let mut cmd3 = Command::cargo_bin("ffez").unwrap();
    cmd3.args([
        "mute",
        input.to_str().unwrap(),
        "-o",
        custom_output.to_str().unwrap(),
        "-y",
    ])
    .assert()
    .success()
    .code(0)
    .stdout(predicate::str::contains("✔ Muted video saved to:"));

    assert_no_temp_files(dir.path());
}

#[test]
fn test_e2e_audio_format_inferred_from_output_extension() {
    let dir = tempdir().unwrap();
    let input = dir.path().join("song.mp4");
    let flac_output = dir.path().join("extracted_lossless.flac");

    if !create_synthetic_video(&input) {
        eprintln!("Skipping test: FFmpeg is not available in PATH.");
        return;
    }

    let mut cmd = Command::cargo_bin("ffez").unwrap();
    cmd.args([
        "audio",
        input.to_str().unwrap(),
        "-o",
        flac_output.to_str().unwrap(),
    ])
    .assert()
    .success()
    .code(0)
    .stdout(predicate::str::contains("✔ Audio saved to:"));

    assert!(flac_output.exists(), "Inferred FLAC output was not created");
    assert!(fs::metadata(&flac_output).unwrap().len() > 0);
    assert_no_temp_files(dir.path());
}
