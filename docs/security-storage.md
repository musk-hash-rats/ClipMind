# Security And Storage Plan

ClipGuard's privacy model should be treated as product infrastructure, not a later hardening pass.

## MVP Storage Boundary

MVP storage should be local-only unless the project owner explicitly approves sync.

Local storage responsibilities:

- Encrypt clip payloads at rest.
- Store safe metadata separately from encrypted payloads.
- Preserve source memory without exposing sensitive content.
- Support expiry and burn-after-use cleanup jobs.
- Support panic wipe for ClipGuard-owned data only.

## Key Management

Current implementation:

- Clip payloads are encrypted with an AES-256-GCM data key.
- The data key is wrapped with a passphrase-derived Argon2id key and stored in the local ClipGuard store as `wrappedDataKey`.
- The store records Argon2id KDF algorithm/version/parameter metadata so future KDF parameter migrations can be handled deliberately.
- First unlock sets the local passphrase and migrates the previous raw key-file fallback into the wrapped-key model.
- Passphrases must be at least 12 characters.
- While unlocked, the unwrapped data key is held only in native runtime memory; locking clears it.
- Startup always forces locked state and clears runtime key material.

Remaining platform hardening:

- macOS: prefer Keychain-backed app secret.
- Windows: prefer DPAPI/Credential Manager-backed app secret.
- Cross-platform fallback: user passphrase-derived wrapped data key.

## Recovery Policy

ClipGuard is local-first encrypted memory. If the passphrase is forgotten, encrypted payload recovery is not available in the current fallback model. The in-app reset flow requires typed `RESET`, clears runtime key material, removes local ClipGuard store/export/key artifacts, and creates a fresh locked store. Reset does not recover payloads.

## Data Classes

- Public metadata: clip type, created time, session name, non-sensitive source labels.
- Private metadata: precise URL, file path, sender/origin identity, window title.
- Secret payload: copied text, images, screenshots, file bytes, OCR text, transform outputs.

Private metadata should be encrypted when it can reveal sensitive context.

## Panic Wipe

Panic wipe must:

- Require confirmation.
- Delete ClipGuard-owned encrypted payloads and indexes.
- Clear active capture state.
- Preserve no recoverable local payload copy.
- Avoid deleting unrelated user files.

## Export Boundary

Agent handoff must:

- Include only selected clips/sessions.
- Show what fields are included before export.
- Redact masked clips unless explicitly revealed.
- Create an audit event with destination, clip IDs, and timestamp.
