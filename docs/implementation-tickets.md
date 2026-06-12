# ClipGuard Implementation Tickets

Status: started after contract approval on 2026-06-11.

## UI And UX

1. Wire session navigation to persisted session records. Done.
2. Replace mock clips with clip records from the local store. Done.
3. Add new-session creation flow. Done with in-app modal.
4. Add selected-clip detail drawer states for text, link, image, screenshot, and file clips.
5. Add explicit reveal/mask flow behind app unlock. Partially done with native lock state, passphrase-backed unlock, audited lock/unlock events, command gating, locked-state metadata/title/preview redaction, forced lock on startup, and transient reveal responses.
6. Add panic-wipe confirmation modal before any destructive action. Done with in-app typed `WIPE` confirmation.
7. Add empty states for first run, paused capture, no search results, and locked store. Partially done for first run, paused capture, and no search results.
8. Add app settings for capture on launch, auto-launch, lock timeout, masking default, and export audit log. Partially done; masking default is wired into native session/capture creation, export audit is always on, OS launch/capture and lock timeout controls are disabled until native hooks exist.
9. Replace browser prompt/confirm flows with in-app desktop modals. Done for new session, selected export, session export, and panic wipe.
10. Add pending/loading state for async native actions. Done for lock, capture, reveal, copy, transform, export, session create, and panic wipe.
11. Disable protected actions while locked. Done for capture, export, paste/copy, transform, reveal, panic wipe, and session creation.

## Tauri Desktop Shell

1. Add tray/menu bar controls for show app, pause/resume capture, lock app, and quit. Done with native tray menu; macOS/Windows behavior still needs validation.
2. Add native notification hooks for capture status and export completion.
3. Add app window behavior for macOS and Windows.
4. Add OS permission checks for clipboard and screenshot/OCR support.
5. Add startup behavior for optional auto-launch.
6. Move clipboard watcher out of renderer polling and into native app/tray lifecycle. Done for native app lifecycle background capture and native tray controls; macOS/Windows validation remains pending.

## Capture And Sessions

1. Implement native clipboard listener on macOS. Partially done with native app lifecycle polling while unlocked and armed; macOS permission/tray validation remains pending.
2. Implement native clipboard listener on Windows. Partially done with native app lifecycle polling while unlocked and armed; Windows permission/tray validation remains pending.
3. Normalize text, link, image, screenshot, and file captures into one clip schema. Done for text/image clipboard payloads plus encrypted file, drag/drop, and screenshot import payloads; live screen capture remains pending.
4. Route new clips into the active session. Done for text and image clipboard payloads while auto capture is armed.
5. Add manual reassignment when a clip lands in the wrong session.
6. Add paused capture state that never stores clipboard changes.
7. Enforce a single active capture session. Done in native capture-state routing; activating one session pauses any other active session.

## Storage And Security

1. Select encrypted local database/file-store approach. Done for AES-256-GCM JSON file store.
2. Define key management and unlock flow. Done for passphrase-backed Argon2id wrapped data key fallback with stored KDF metadata; OS keychain/DPAPI integration remains pending.
3. Store clip metadata separately from encrypted payload only if the risk is accepted.
4. Add auto-expiry and burn-after-use enforcement. Burn-after-use is enforced for reveal, original copy, transform, selected export, and session export; auto-expiry remains pending.
5. Add panic-wipe scope and recovery expectations. Done for selected-clip panic wipe and distinct local-store reset audit events.
6. Ensure logs never include clip payloads or secrets.
7. Add passphrase recovery/reset UX. Done with typed `RESET` local-store wipe flow; OS recovery remains pending.

## Search And Transform

1. Add exact text search. Done in native over decrypted payloads while unlocked, with locked local fallback over redacted metadata/previews.
2. Add metadata filters by session, source, origin, timestamp, and clip type.
3. Add semantic search milestone and embedding storage decision. Done for first local encrypted hash-vector index with exact/semantic UI mode and rebuild flow; neural embedding model remains an upgrade path in `docs/semantic-search-plan.md`.
4. Implement clean formatting. Done.
5. Implement Markdown conversion. Done.
6. Implement JSON repair/pretty-print. Done.
7. Implement link tracking cleanup. Done.
8. Add OCR and image resize after image/screenshot support lands. Resize is done for image/screenshot payloads; OCR is implemented through a local Tesseract backend and requires host validation.
9. Add real image preview/detail UI. Done for revealed ClipGuard RGBA image payloads and imported image/screenshot files.
10. Copy original selected clip to system clipboard. Done with native decrypt, audit, and burn-after-use handling.
11. Remove hardcoded semantic match score until semantic search exists. Done.
12. Make transform history outputs actionable. Done for copying previous transform output back to clipboard.

## Agent Handoff

1. Export selected clips as Markdown.
2. Export selected clips as JSON with source metadata. Done.
3. Add visible export audit trail. Done.
4. Require explicit user selection before agent context export. Done.
5. Keep local-only clips out of exports by default. Done for session exports by redacting to safe previews.
6. Surface export output path after native export. Done via `lastExportPath` in native state.

## Current Verification

- `npm run typecheck` passes on 2026-06-12.
- `npm run build` passes on 2026-06-12.
- `cargo check` passes on 2026-06-12.
- `npm run check` passes on 2026-06-12.
- `cargo test` passes on 2026-06-12.
- Capture routing and KDF metadata unit tests pass on 2026-06-12.
- Resize payload and semantic vector unit tests pass on 2026-06-12.
- OCR backend requirement is documented in `docs/ocr.md`; host Tesseract validation remains pending.
- Native app lifecycle clipboard watcher and local tray controls pass local checks on 2026-06-12; platform permission validation remains pending.
- `npm run tauri:build` passes on 2026-06-12 and produces Linux `.deb`, `.rpm`, and AppImage artifacts.
- Platform validation evidence and remaining macOS/Windows checklist live in `docs/platform-validation.md`.
- Rust/Cargo is installed in this workspace as of 2026-06-12; macOS `.dmg` and Windows `.exe` installers still need native target builds or GitHub Actions.
