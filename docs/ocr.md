# OCR Backend

ClipMind OCR is implemented as a pluggable local-engine path.

## Current Backend

- Engine: local `tesseract` CLI.
- Supported input: revealed image/screenshot payloads that ClipMind can decode.
- Processing: ClipMind writes a temporary PNG under its app data `tmp/` directory, runs `tesseract <image> stdout`, captures text, and removes the temporary PNG.
- Timeout: OCR is killed if Tesseract runs longer than 20 seconds.
- Storage: OCR output is stored as an encrypted transform payload.
- Clipboard: OCR output is copied to the system clipboard as text.

## Requirement

The host must have `tesseract` installed and available on `PATH`.

If `tesseract` is missing or returns no text, the OCR transform fails with a visible error and does not create a transform.

## Validation Still Required

- macOS Tesseract install/path validation.
- Windows Tesseract install/path validation.
- OCR temp-file cleanup validation after success and failure.
- OCR quality test set for common screenshots.
