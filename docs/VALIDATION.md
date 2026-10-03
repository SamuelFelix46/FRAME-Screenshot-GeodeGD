# Validation

Target: Windows x64, Geometry Dash 2.2081, Geode 5.10.1. FRAME's implementation is Rust, with locally corrected community geode-rs bindings. No Android/macOS support is advertised.

## Automated checks

74 Rust tests: 44 core/file tests, 25 editor interaction tests, 3 native capture adapter tests, 1 native GD field-layout test and 1 generator layout test. `build.ps1` runs these checks, Rust formatting and the PowerShell data-import checks before packaging.

- Capture gate: percentage/range, practice filter, cooldown and dual duplicates. The actual native adapter reads zero images across 1,000 normal rendered frames, then one image after a confirmed death, retaining death percentage/attempt. Reset cancels pending death capture. These adapter tests use a controlled image source, without an OpenGL context.
- Editor: actual egui events for text entry, multiline confirmation/cancellation, movement, anchored corner resizing, repeated shapes with the same tool, undo/redo and modifier keys.
- Files: real PNG/JPEG decoding and export, BGRA/bottom-up CF_DIB clipboard representation, protected originals and trash, pending-write retries and safe document reopening.
- Language/settings: English default, French/English changes at runtime, persisted language, shortcuts and clipboard preferences; existing screenshot bytes preserved.
- Installer import: missing files only, existing destination/settings unchanged, originals/edits/trash preserved, repeated import idempotent and nested destination rejected.

## Native game checks and limits

The retained editor/capture features were exercised in an isolated GD copy during 1.4.0/1.4.1 development: closed-studio clicks reached GD, drag/move/resize/text gestures worked, undo/redo and JPEG export produced files, and the Windows clipboard contained the edited 1920 × 1200 image. English UI was observed in-game. A Save as dialog opened, but automated selection of its final file was not confirmed end to end; file writing is covered by backend tests.

The new one-read death timing is verified by adapter tests. Exact FPS improvement has not been measured on the user's machine. The single OpenGL read may briefly stall at high resolutions. Version 1.4.2 changes identity/startup storage and removes diagnostic commands; earlier native observations do not constitute a fresh native test of every 1.4.2 feature.

Multi-mod compatibility, all DPI/resolution combinations and every physical keyboard layout have not been exhaustively tested. Pause GD before using the editor; gameplay continues behind the studio. Capture on death and automatic clipboard copying are off by default.

## Release checks

The package must contain `mod.json` with ID `zemci.frame`, developer `zemci`, GD Windows 2.2081, Geode 5.10.1 and the public source URL; exactly one native DLL named `zemci.frame.dll`; the Geode entry point; logo, documentation and third-party licenses. Original camera/shutter assets are embedded in the DLL and included in source. Source archives exclude build outputs, logs, test captures, credentials and toolchains. SHA256 checksums accompany release files.

GitHub Actions runs the tested build from published source. Its successful artifact provides a reproducible build route, rather than a claim of byte-for-byte identical binaries across compiler versions. Geode Index acceptance is a separate moderator decision.
