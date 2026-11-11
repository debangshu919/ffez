# Unreleased

- Add `ffez compress`, a two-pass encode that targets a file size.
- Give compress a constant frame rate so variable-frame-rate video does not flood the terminal during the analysis pass.
- Add `ffez compress --fast` and `ffez compress --quality`.
- Add `ffez compress --gpu` to encode with NVIDIA NVENC when the driver is available.

# v0.1.0

Initial release with `audio` and `mute` commands.