# Security And Storage Plan

ClipMind's privacy model should be treated as product infrastructure, not a later hardening pass.

## MVP Storage Boundary

MVP storage should be local-only unless the project owner explicitly approves sync.

Local storage responsibilities:

- Encrypt clip payloads at rest.
- Store safe metadata separately from encrypted payloads.
- Preserve source memory without exposing sensitive content.
- Support expiry and burn-after-use cleanup jobs.
- Support panic wipe for ClipMind-owned data only.

## Key Management

Open implementation decision:

- macOS: prefer Keychain-backed app secret.
- Windows: prefer DPAPI/Credential Manager-backed app secret.
- Cross-platform fallback: user passphrase-derived key.

The UI can show an app lock before key management is fully wired, but implementation should not call privacy complete until unlock/key retrieval exists.

## Data Classes

- Public metadata: clip type, created time, session name, non-sensitive source labels.
- Private metadata: precise URL, file path, sender/origin identity, window title.
- Secret payload: copied text, images, screenshots, file bytes, OCR text, transform outputs.

Private metadata should be encrypted when it can reveal sensitive context.

## Panic Wipe

Panic wipe must:

- Require confirmation.
- Delete ClipMind-owned encrypted payloads and indexes.
- Clear active capture state.
- Preserve no recoverable local payload copy.
- Avoid deleting unrelated user files.

## Export Boundary

Agent handoff must:

- Include only selected clips/sessions.
- Show what fields are included before export.
- Redact masked clips unless explicitly revealed.
- Create an audit event with destination, clip IDs, and timestamp.
