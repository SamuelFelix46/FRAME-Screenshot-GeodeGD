# FRAME Screenshot Studio

[![Build FRAME](https://github.com/SamuelFelix46/FRAME-Screenshot-GeodeGD/actions/workflows/build.yml/badge.svg)](https://github.com/SamuelFelix46/FRAME-Screenshot-GeodeGD/actions/workflows/build.yml)

A fullscreen screenshot gallery and editor for Geometry Dash, by **zemci**. Written in Rust. One mod includes English and French; fresh installations start in English.

**Windows x64 · Geometry Dash 2.2081 · Geode 5.10.1 / compatible 5.x**

[Download a release](https://github.com/SamuelFelix46/FRAME-Screenshot-GeodeGD/releases) · [Guide français](README-fr.md) · [Report an issue](https://github.com/SamuelFelix46/FRAME-Screenshot-GeodeGD/issues)

## Install

Close Geometry Dash. Copy **zemci.frame.geode** from the release into **Geometry Dash/geode/mods**, then start the game. There is only one package for both languages.

**Updating an earlier FRAME installation:** use the full release ZIP, extract it, then run `source/install.ps1` with PowerShell. The installer identifies previous FRAME packages by their manifests, copies missing screenshots and settings into the new `geode/config/zemci.frame` directory, backs up previous packages and installs a single mod. Existing destination files and previous data remain intact. For a different game location:

```powershell
./install.ps1 -GamePath 'D:\Games\Geometry Dash'
```

For a data folder from an older installation, add `-PreviousDataDirectory 'D:\Backup\FRAME'`. Manual replacement of a `.geode` does not import another mod ID's data automatically. Use the installer before starting the new version if you want the previous language/settings imported.

## Capture and edit

**F6** opens or closes the studio. **F8** captures the game. Change these keys and modifier combinations in **Settings > Shortcuts**. When the studio is closed, game clicks and keys pass through, including during the passive preview and flash. Pause GD before editing; the game keeps running behind the studio.

- **Text:** choose Text, click the image and type. Enter confirms, Shift+Enter adds a line, Escape cancels. Double-click existing text to edit it.
- **Shapes:** hold and drag to draw. Tools remain selected for successive annotations. Click the active tool again or choose Select / move to deselect it.
- **Move and resize:** select an annotation, drag it or its corner handles. Shift constrains a shape to a square/circle. Properties adjust position, size, fill, color and stroke.
- **View and crop:** pan with Hand, zoom with the wheel or controls; crop freely, by ratio or with precise coordinates.
- **Layers and history:** duplicate, reorder and delete with small icons. Ctrl+Z/Y undo/redo, Ctrl+D duplicates, Delete removes, arrows move by 1 px and Shift+arrows by 10 px.
- **Gallery:** favorites, recoverable trash and direct inline renaming.

Original screenshots stay separate from editing documents. **Copy** copies the edited/cropped image to the Windows clipboard. **Save as** chooses a PNG/JPEG destination. **Export** creates a copy in `exports`. Original and trash directories are protected against export overwrites.

## Settings

Configure flash, shutter sound, preview size/position/duration, clipboard copying, gallery, editor defaults, shortcuts, interface scale and language. Switch **English / Français** in **Settings > Interface and files**; the choice applies immediately and is saved.

**Copy every screenshot to the clipboard** copies both manual and automatic captures when enabled. It is off by default. A clipboard failure does not discard the saved capture.

Optionally select a local `.exe` in Gallery and export to open exports with your preferred app. FRAME does not download or install applications.

**Capture on death** is off by default. Configure percentage threshold/range, cooldown and practice filter. GD must confirm a living player's death before a request is queued. FRAME reads one image on the next rendered frame before its own flash/preview. It performs no continuous screenshot reads during gameplay. Dual deaths are filtered; a reset or exit before rendering cancels the request. Death effects may appear in the image. PNG saving runs in the background; the single GPU read can still cause a short stall at high resolutions.

## Build from source

Install Rust stable x64 MSVC with rustfmt, Visual Studio C++ Build Tools (MSVC linker and Windows SDK) and LLVM 21+ / libclang. The mod's implementation is Rust; vendored SDK headers are used to generate Rust bindings.

```powershell
$env:LIBCLANG_PATH = 'C:\Program Files\LLVM\bin'
./build.ps1
```

The script runs the tests, builds a release DLL and creates `dist/zemci.frame.geode` plus its SHA256 checksum. `-SkipTests` is available for local rebuilds. Dependencies are pinned in Cargo.lock and necessary Geode bindings are vendored. GitHub Actions runs the same tested build on Windows.

See [validation and limits](docs/VALIDATION.md), [Geode publication notes](docs/PUBLISHING.md) and [third-party credits](THIRD_PARTY.md). Source is MIT licensed. This repository was developed with AI assistance; Geode Index acceptance is subject to its code and authorship review.
