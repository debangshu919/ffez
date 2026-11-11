# Unreleased

- Add `ffez compress`, a two-pass encode that targets a file size.
- Give compress a constant frame rate so variable-frame-rate video does not flood the terminal during the analysis pass.
- Add `ffez compress --fast` and `ffez compress --quality`.

# v0.1.0

Initial release with `audio` and `mute` commands.