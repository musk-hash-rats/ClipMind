# Release Security Checklist

ClipMind is not release-ready until every required item is complete or explicitly waived by the project owner.

## Required Before Alpha

- [ ] `npm run check` passes.
- [ ] CI continuous checks pass on `main`.
- [ ] Dependency review passes for pull requests.
- [ ] CodeQL completes without high-severity findings.
- [x] Native clipboard capture is opt-in and pauseable.
- [x] Capture state is always visible.
- [x] Clip payloads are encrypted at rest.
- [ ] Key management is implemented for macOS and Windows.
- [x] Passphrase recovery/reset policy is visible to users before passphrase setup.
- [ ] Private metadata handling is documented and implemented.
- [x] Panic wipe is scoped to ClipMind-owned data only.
- [x] Agent export requires explicit selected clips/sessions.
- [x] Agent export creates an audit event.
- [ ] Masked clips are not exported unless explicitly revealed or included.
- [ ] No logs include payloads, secrets, tokens, or private metadata.
- [ ] Accessibility pass covers keyboard navigation, focus states, labels, and contrast.

## Required Before Public Release

- [ ] macOS installer is signed and notarized.
- [ ] Windows installer is signed.
- [ ] macOS clipboard and screen-recording/screenshot permission behavior is validated.
- [ ] Windows clipboard, screenshot, and file-drop behavior is validated.
- [ ] Local Tesseract OCR engine availability and temp-file cleanup behavior is validated on target platforms.
- [ ] System tray/menu behavior is implemented locally and still needs validation on macOS and Windows.
- [ ] Auto-start behavior is validated on macOS and Windows.
- [ ] App data paths are documented and validated on macOS and Windows.
- [x] Release artifacts include provenance or checksums.
- [ ] Vulnerability reporting path is published.
- [ ] Privacy policy reflects actual local storage/export behavior.
- [ ] Final standards-compliance review is complete.
