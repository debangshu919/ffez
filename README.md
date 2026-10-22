<div align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/debangshu919/ffez/refs/heads/main/assets/images/ffez_light.svg" width="200px">
    <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/debangshu919/ffez/refs/heads/main/assets/images/ffez_dark.svg" width="200px">
    <img alt="ffez Logo" src="https://raw.githubusercontent.com/debangshu919/ffez/refs/heads/main/assets/images/ffez_light.svg" width="200px">
  </picture>
  <br/>
  <img alt="Crates.io Version" src="https://img.shields.io/crates/v/ffez?style=for-the-badge">
  <a href="https://github.com/debangshu919/ffez/actions/workflows/ci.yml"><img alt="CI Status" src="https://img.shields.io/github/actions/workflow/status/debangshu919/ffez/ci.yml?branch=main&style=for-the-badge"></a>
  <img alt="Crates.io License (version)" src="https://img.shields.io/crates/l/ffez/0.1.0?style=for-the-badge&cacheSeconds=3600">
</div>
<h3 align="center"><b>A minimal FFmpeg CLI wrapper built for your everyday audio and video tasks.</b></h3>

<p align="center">
  No need to memorize complex FFmpeg flags. <b>ffez</b> handles the syntax, ensures safe atomic file writes, and lets you preview underlying commands before running them.
</p>

<p align="center">
  <a href="#prerequisites">Prerequisites</a> •
  <a href="#installation">Installation</a> •
  <a href="#usage">How to Use</a> •
  <a href="#command-reference">Command Reference</a>
</p>

---

## Prerequisites

- Rust and Cargo for installation via Cargo.
- [FFmpeg](https://ffmpeg.org/download.html) must be installed and available in your system `PATH`.

---

## Installation

Install using Cargo
```bash
cargo install ffez
```
Make sure FFmpeg is installed and available on your system's PATH.

---

## Usage

### Mute a Video

Strips audio tracks without re-encoding video for instant, lossless output.

```bash
# Saves output to <input>_muted.<ext>
ffez mute video.mp4

# Specify a custom output path
ffez mute video.mp4 -o silent.mp4

# Overwrite existing destination file
ffez mute video.mp4 -y
```

### Extract Audio

Extracts the audio track from a video file. Supports `mp3` (default), `wav`, `m4a`, and `flac`.

```bash
# Extracts to <input>.mp3
ffez audio video.mp4

# Specify format explicitly
ffez audio video.mp4 -f wav

# Automatically infers format from the output extension
ffez audio video.mp4 -o soundtrack.flac
```

### Dry Run Preview

Inspect the exact FFmpeg command that would be executed without running it:

```bash
ffez mute video.mp4 --dry-run
ffez audio video.mp4 -o track.wav --dry-run
```

---

## Command Reference

| Command | Description | Default Output |
| :--- | :--- | :--- |
| `ffez mute <INPUT>` | Strips audio from video (lossless copy) | `<input>_muted.<ext>` |
| `ffez audio <INPUT>` | Extracts audio track (`mp3`, `wav`, `m4a`, `flac`) | `<input>.<format>` |

### Common Flags

| Flag | Description |
| :--- | :--- |
| `-o, --output <PATH>` | Custom output file path |
| `-y, --overwrite` | Overwrite destination file if it already exists |
| `--dry-run` | Print the underlying FFmpeg command without executing it |
| `-h, --help` | Print help information |
| `-V, --version` | Print version |

---

## License

Copyright (c) 2026 [Debangshu Das](https://debangshu.dev).

`ffez` is made available under the terms of either the MIT License or the Apache License 2.0, at your option.

See the [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT) files for license details.
