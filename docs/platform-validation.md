# Platform Validation

Last updated: 2026-06-12.

## Current Evidence

- Linux release packaging passes with `npm run tauri:build`.
- Linux artifacts generated:
  - `src-tauri/target/release/bundle/deb/ClipGuard_0.1.0_amd64.deb`
  - `src-tauri/target/release/bundle/rpm/ClipGuard-0.1.0-1.x86_64.rpm`
  - `src-tauri/target/release/bundle/appimage/ClipGuard_0.1.0_amd64.AppImage`
- Local checks pass:
  - `npm run check`
  - `cargo check`
  - `cargo test`
- Release artifact evidence is configured:
  - `npm run release:checksums` writes `src-tauri/target/release/bundle/SHA256SUMS`.
  - `npm run release:checksums` writes `src-tauri/target/release/bundle/provenance.json`.
  - `.github/workflows/desktop-installers.yml` uploads installer artifacts with checksum and provenance files.
- CI coverage is configured for frontend checks, Rust `cargo check`, Rust `cargo test`, dependency review, and CodeQL for TypeScript and Rust.

## Native Platform Validation Still Required

These items require macOS and Windows hosts or CI runners. They are not validated from the current Linux workspace.

### macOS

- Clipboard read/write behavior while window is focused, hidden, and minimized.
- Screenshot/screen-recording permission prompts for live screen capture.
- Tray/menu behavior for show, pause capture, resume capture, lock, and quit.
- App data path location and file permissions.
- `.dmg`/app bundle signing and notarization.
- Installer smoke test on a clean user profile.
- Generated checksum/provenance files attached to the macOS installer workflow artifact.

### Windows

- Clipboard read/write behavior while window is focused, hidden, and minimized.
- File import and file-drop behavior.
- Screenshot permission behavior for live screen capture.
- Tray/menu behavior for show, pause capture, resume capture, lock, and quit.
- App data path location and file permissions.
- Installer signing.
- Installer smoke test on a clean user profile.
- Generated checksum/provenance files attached to the Windows installer workflow artifact.
