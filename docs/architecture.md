# ClipMind Architecture Notes

## Approved Product Shape

ClipMind is an encrypted working-memory desktop app for macOS and Windows. It should not be framed as generic clipboard history.

## Initial Modules

- UI shell: Tauri webview with TypeScript.
- Native shell: Rust/Tauri commands, tray/menu bar hooks, OS permissions.
- Capture service: native clipboard listener with visible paused/active state.
- Storage service: encrypted local clip/session store.
- Session service: automatic routing of captured clips into the active work session.
- Transform service: formatting, Markdown, summary, translation, JSON repair, link cleanup, OCR, image resize, note-to-task/message/email.
- Agent handoff: explicit export of selected clips/sessions with metadata and audit trail.

Detailed contracts:

- `data-contract.md`
- `security-storage.md`
- `clipboard-session-contract.md`

## Security Concerns To Resolve Early

- Key management and app unlock flow.
- Panic wipe scope and confirmation path.
- Sensitive preview masking defaults.
- Local-only storage vs. future sync boundary.
- Export audit trail: what was shared, when, and from which session.

## Current Blockers

- Native clipboard capture behavior needs separate macOS and Windows implementation notes.
- Final macOS `.dmg` and Windows `.exe` installer builds need native target validation or GitHub Actions.
- App unlock/keychain hardening is still needed beyond the current local AES-256-GCM file-store.
