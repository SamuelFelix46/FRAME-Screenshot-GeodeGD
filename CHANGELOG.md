# Changelog

## 1.4.2

- Public author and mod identity are now `zemci` / `zemci.frame`, including the packaged DLL and configuration directory.
- The installer imports missing previous FRAME data, preserves existing files and keeps only one FRAME package installed, with previous packages backed up.
- Clean English documentation, source archive, license notices and a Windows GitHub Actions build. Removed development-only native commands and file polling.
- Use Geode's configuration directory API and omit personal build paths from the release DLL.

## 1.4.1

- Removed continuous screenshot caching during gameplay. Capture on death waits for a confirmed death and reads one image on the next render; reset/exit cancel pending captures.
- One bilingual mod, English by default, remembered language and settings.
- Drawing tools remain selected until manually changed or deselected.

## 1.4.0

- Fullscreen studio, persistent drawing tools, inline text, selection, movement and resize handles, crop, layers and history.
- Configurable shortcuts, clipboard copying, Save as PNG/JPEG, optional local export application and passive capture preview.
- Input passthrough while the studio is closed; safer pending saves, trash and original-image protection.
