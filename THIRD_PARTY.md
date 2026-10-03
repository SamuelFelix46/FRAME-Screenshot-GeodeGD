# Third-party notices

- **geode-rs / geode-egui:** https://github.com/zeozeozeo/geode-rs — Unlicense, included in vendor/geode-rs/LICENSE. Necessary crates, SDK headers and bindings are vendored. FRAME adjusts Cocos type allowlisting, conditionally enables the egui keyboard hook so FRAME shortcuts are handled first, and represents Geode SeedValue types as arrays of two/three integers to preserve GD field offsets. A generator test covers the layout correction.
- **Geode / Cocos / FMOD headers:** original notices remain in the vendored files. These headers generate Rust declarations; FRAME does not compile its own C++ code or distribute these engines' binaries.
- **egui / epaint / epaint_default_fonts / egui_glow:** https://github.com/emilk/egui — MIT or Apache-2.0; fonts retain their respective licenses.
- **Ubuntu Light:** Ubuntu Font Licence 1.0. Font licenses are in font-licenses.
- **image, imageproc, ab_glyph, serde, serde_json and transitive dependencies:** versions pinned in Cargo.lock. Available license texts are in dependency-licenses; index.json records SPDX identifiers and project links.

Dependency, font and geode-rs license texts also accompany the .geode package. Third-party authorship is retained. FRAME's own code, camera icon and shutter sound are provided under the root MIT license, by zemci.
