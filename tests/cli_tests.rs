use assert_cmd::Command;
use predicates::prelude::*;
use std::fs::File;
use tempfile::tempdir;

#[test]
fn test_cli_help_flag() {
    let mut cmd = Command::cargo_bin("ffez").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains("ffez"))
        .stdout(predicate::str::contains("Commands:"))
        .stdout(predicate::str::contains("mute"))
        .stdout(predicate::str::contains("audio"));

    // Short flag -h
    let mut short_cmd = Command::cargo_bin("ffez").unwrap();
    short_cmd.arg("-h").assert().success().code(0);
}

#[test]
fn test_cli_version_flag() {
    let mut cmd = Command::cargo_bin("ffez").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains("ffez 0.1.0"));

    // Short flag -V
    let mut short_cmd = Command::cargo_bin("ffez").unwrap();
    short_cmd
        .arg("-V")
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains("ffez 0.1.0"));
}

#[test]
fn test_subcommand_mute_help() {
    let mut cmd = Command::cargo_bin("ffez").unwrap();
    cmd.args(["mute", "--help"])
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains("Removes all audio streams"))
        .stdout(predicate::str::contains("-o, --output"))
        .stdout(predicate::str::contains("-y, --overwrite"));

    // Short help contains the brief 'about'
    let mut short_cmd = Command::cargo_bin("ffez").unwrap();
    short_cmd
        .args(["mute", "-h"])
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains("Mute a video"));
}

#[test]
fn test_subcommand_audio_help() {
    let mut cmd = Command::cargo_bin("ffez").unwrap();
    cmd.args(["audio", "--help"])
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains(
            "Extracts audio from a video file into mp3",
        ))
        .stdout(predicate::str::contains("-f, --format"))
        .stdout(predicate::str::contains("mp3"))
        .stdout(predicate::str::contains("wav"))
        .stdout(predicate::str::contains("m4a"))
        .stdout(predicate::str::contains("flac"));

    // Short help contains the brief 'about'
    let mut short_cmd = Command::cargo_bin("ffez").unwrap();
    short_cmd
        .args(["audio", "-h"])
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains(
            "Extract the audio track from a video file",
        ));
}

#[test]
fn test_cli_no_args_fails_with_code_2() {
    let mut cmd = Command::cargo_bin("ffez").unwrap();
    cmd.assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("Usage:"));
}

#[test]
fn test_cli_mute_missing_input_fails_with_code_2() {
    let mut cmd = Command::cargo_bin("ffez").unwrap();
    cmd.arg("mute")
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("<INPUT>"));
}

#[test]
fn test_cli_audio_missing_input_fails_with_code_2() {
    let mut cmd = Command::cargo_bin("ffez").unwrap();
    cmd.arg("audio")
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("<INPUT>"));
}

#[test]
fn test_cli_audio_invalid_format_enum_fails_with_code_2() {
    let mut cmd = Command::cargo_bin("ffez").unwrap();
    cmd.args(["audio", "video.mp4", "-f", "ogg"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("invalid value 'ogg'"))
        .stderr(predicate::str::contains(
            "[possible values: mp3, wav, m4a, flac]",
        ));
}

#[test]
fn test_cli_unknown_option_fails_with_code_2() {
    let mut cmd = Command::cargo_bin("ffez").unwrap();
    cmd.args(["mute", "video.mp4", "--nonexistent-option"])
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains(
            "unexpected argument '--nonexistent-option'",
        ));
}

#[test]
fn test_cli_missing_input_file_exits_with_code_1() {
    let mut cmd = Command::cargo_bin("ffez").unwrap();
    cmd.args(["mute", "non_existent_file_xyz_123.mp4"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains(
            "Input file not found: 'non_existent_file_xyz_123.mp4'",
        ));
}

#[test]
fn test_cli_output_exists_without_overwrite_exits_with_code_1() {
    let dir = tempdir().unwrap();
    let input_path = dir.path().join("input.mp4");
    let output_path = dir.path().join("output.mp4");

    File::create(&input_path).unwrap();
    File::create(&output_path).unwrap();

    let mut cmd = Command::cargo_bin("ffez").unwrap();
    cmd.args([
        "mute",
        input_path.to_str().unwrap(),
        "-o",
        output_path.to_str().unwrap(),
    ])
    .assert()
    .failure()
    .code(1)
    .stderr(predicate::str::contains("Output file already exists"))
    .stderr(predicate::str::contains(
        "Pass --overwrite (-y) to replace it.",
    ));
}

#[test]
fn test_cli_input_equals_output_exits_with_code_1() {
    let dir = tempdir().unwrap();
    let input_path = dir.path().join("same_video.mp4");
    File::create(&input_path).unwrap();

    let mut cmd = Command::cargo_bin("ffez").unwrap();
    cmd.args([
        "mute",
        input_path.to_str().unwrap(),
        "-o",
        input_path.to_str().unwrap(),
        "-y",
    ])
    .assert()
    .failure()
    .code(1)
    .stderr(predicate::str::contains(
        "Output path cannot be the same as input path",
    ));
}

#[test]
fn test_cli_dry_run_mute() {
    let mut cmd = Command::cargo_bin("ffez").unwrap();
    cmd.args(["mute", "clip.mp4", "--dry-run"])
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains(
            "ffmpeg -y -hide_banner -loglevel error -i clip.mp4 -c:v copy -an -movflags +faststart clip_muted.mp4",
        ));
}

#[test]
fn test_cli_dry_run_global_flag_before_subcommand() {
    let mut cmd = Command::cargo_bin("ffez").unwrap();
    cmd.args(["--dry-run", "mute", "clip.mp4"])
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains("ffmpeg -y -hide_banner"))
        .stdout(predicate::str::contains("clip_muted.mp4"));
}

#[test]
fn test_cli_dry_run_audio_explicit_format() {
    let mut cmd = Command::cargo_bin("ffez").unwrap();
    cmd.args(["audio", "clip.mp4", "-f", "wav", "--dry-run"])
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains("-vn"))
        .stdout(predicate::str::contains("-c:a pcm_s16le"))
        .stdout(predicate::str::contains("clip.wav"));
}

#[test]
fn test_cli_dry_run_audio_inferred_format_from_output() {
    let mut cmd = Command::cargo_bin("ffez").unwrap();
    cmd.args(["audio", "clip.mp4", "-o", "extracted.flac", "--dry-run"])
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains("-vn"))
        .stdout(predicate::str::contains("-c:a flac"))
        .stdout(predicate::str::contains("extracted.flac"));
}

#[test]
fn test_cli_dry_run_quotes_paths_with_spaces() {
    let mut cmd = Command::cargo_bin("ffez").unwrap();
    cmd.args([
        "mute",
        "raw vacation video.mp4",
        "-o",
        "silent vacation video.mp4",
        "--dry-run",
    ])
    .assert()
    .success()
    .code(0)
    .stdout(predicate::str::contains("\"raw vacation video.mp4\""))
    .stdout(predicate::str::contains("\"silent vacation video.mp4\""));
}
