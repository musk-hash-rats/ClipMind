# OCR Validation Fixtures

No binary fixture images are checked into the project yet.

## Fixture Set To Add

- High-contrast screenshot with one short sentence.
- Dark-mode screenshot with white text.
- Small UI screenshot with mixed labels and numbers.
- Blurry/low-resolution screenshot expected to produce partial or no text.

## Validation Cases

- Tesseract missing: transform fails visibly and creates no transform output.
- Tesseract timeout: process is killed after 20 seconds and temporary PNG is removed.
- Tesseract success: OCR text is encrypted as transform output and copied to clipboard.
- Tesseract no text: transform fails visibly and creates no transform output.
- Cleanup: temporary PNG is removed on success, failure, and timeout.
