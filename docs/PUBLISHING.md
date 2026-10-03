# Publishing FRAME

The repository and versioned releases are public. Publication on GitHub does not mean approval by the Geode Index.

## Current Geode requirements

Consult the official [guidelines](https://github.com/geode-sdk/docs/blob/main/mods/guidelines.md), [publishing guide](https://github.com/geode-sdk/docs/blob/main/mods/publishing.md) and [manifest guide](https://github.com/geode-sdk/docs/blob/main/mods/configuring.md) before submitting. Requirements may change.

Geode reviews functionality, metadata, icon/tags, source availability, safety, privacy and performance. It prefers a package built by CI from reviewable source. FRAME provides Windows-only metadata, an original icon, utility/offline/interface tags, public source and issues, licenses and a tested Windows CI workflow. Files are stored through Geode's configuration directory API. Screenshot encoding and editing/export work run in the worker; the one-frame GPU read still occurs on the render thread.

This project was developed with AI assistance. Geode's guidelines can reject mostly AI-generated code when the submitting developer does not understand it. A clean manifest and passing tests do not waive that requirement. The submitting author must understand the code, maintain it and answer review questions. Do not hide AI assistance or claim guaranteed acceptance.

FRAME makes no network requests in normal use and downloads no executable code. Opening exports in a local application is an explicit user preference. Automatic death capture and clipboard copying are disabled by default.

## Release and submission

1. Update Cargo.toml, the root package entry in Cargo.lock, mod.json and CHANGELOG.md together. Never replace an already published version with different bytes.
2. Run `build.ps1`, review changes and commit. Push a matching `vX.Y.Z` tag. The workflow validates the tag against mod.json and uploads the tested Windows package.
3. The release job downloads that successful CI artifact and publishes `zemci.frame.geode`, clean sources/full ZIP and checksums on the matching tag. A release that already exists is not overwritten. Retain license notices and never upload a package built from different source.
4. Submit the direct versioned `.geode` asset URL through the Geode website or its CLI according to the publishing guide. Complete its review and respond to moderators. Do not treat a GitHub release as an Index listing.

For this release the ID is `zemci.frame`, author `zemci`, version 1.4.2, Windows GD 2.2081 and Geode 5.10.1. Only platforms actually validated should be added in future releases.
