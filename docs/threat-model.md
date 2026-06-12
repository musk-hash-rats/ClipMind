# ClipGuard Threat Model

Status: initial planning model. Update before alpha and whenever storage, capture, export, or sync behavior changes.

## Assets

- Clipboard payloads: text, links, screenshots, images, and file references.
- Private source metadata: URLs, file paths, sender/origin identity, window titles.
- Session context: grouped clips and project/task names.
- Encryption keys and unlock state.
- Agent export bundles and audit records.

## Trust Boundaries

- Native OS clipboard APIs.
- Tauri command boundary between TypeScript UI and Rust backend.
- Local encrypted store.
- Export boundary from ClipGuard to agents or files.
- Installer/update distribution pipeline.

## Primary Risks

1. Sensitive clipboard data is stored without encryption.
2. Masked clips leak through previews, logs, search indexes, or exports.
3. Capture continues when the user believes it is paused.
4. Panic wipe deletes unrelated user data or fails to delete ClipGuard payloads.
5. Agent exports include more clips or metadata than the user selected.
6. Source metadata reveals private browsing, file, or sender context.
7. A malicious dependency or compromised build pipeline ships unsafe installers.

## Required Controls

- Opt-in, visible, pauseable capture.
- Encrypted payload storage before real user data is retained.
- Private metadata encryption when metadata reveals sensitive context.
- Explicit reveal before masked content appears in export.
- Audit event for export, reveal, panic wipe, expiry, and burn-after-use.
- Dependency review, CodeQL, and lockfile-backed installs in CI.
- Signed/notarized installers before public release.

## Out Of Scope For MVP

- Cloud sync.
- Team/shared vaults.
- Remote agent access to live clipboard state.

Adding any out-of-scope item requires a new threat-model pass before implementation.
