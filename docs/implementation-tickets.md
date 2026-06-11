# ClipMind Implementation Tickets

Status: started after contract approval on 2026-06-11.

## UI And UX

1. Wire session navigation to persisted session records.
2. Replace mock clips with clip records from the local store.
3. Add new-session creation flow.
4. Add selected-clip detail drawer states for text, link, image, screenshot, and file clips.
5. Add explicit reveal/mask flow behind app unlock.
6. Add panic-wipe confirmation modal before any destructive action.
7. Add empty states for first run, paused capture, no search results, and locked store.
8. Add app settings for capture on launch, auto-launch, lock timeout, masking default, and export audit log.

## Tauri Desktop Shell

1. Add tray/menu bar controls for show app, pause capture, lock app, and quit.
2. Add native notification hooks for capture status and export completion.
3. Add app window behavior for macOS and Windows.
4. Add OS permission checks for clipboard and screenshot/OCR support.
5. Add startup behavior for optional auto-launch.

## Capture And Sessions

1. Implement native clipboard listener on macOS.
2. Implement native clipboard listener on Windows.
3. Normalize text, link, image, screenshot, and file captures into one clip schema.
4. Route new clips into the active session.
5. Add manual reassignment when a clip lands in the wrong session.
6. Add paused capture state that never stores clipboard changes.

## Storage And Security

1. Select encrypted local database/file-store approach.
2. Define key management and unlock flow.
3. Store clip metadata separately from encrypted payload only if the risk is accepted.
4. Add auto-expiry and burn-after-use enforcement.
5. Add panic-wipe scope and recovery expectations.
6. Ensure logs never include clip payloads or secrets.

## Search And Transform

1. Add exact text search.
2. Add metadata filters by session, source, origin, timestamp, and clip type.
3. Add semantic search milestone and embedding storage decision.
4. Implement clean formatting.
5. Implement Markdown conversion.
6. Implement JSON repair/pretty-print.
7. Implement link tracking cleanup.
8. Add OCR and image resize after image/screenshot support lands.

## Agent Handoff

1. Export selected clips as Markdown.
2. Export selected clips as JSON with source metadata.
3. Add visible export audit trail.
4. Require explicit user selection before agent context export.
5. Keep local-only clips out of exports by default.

## Current Verification

- `npm run typecheck` passes.
- `npm run build` passes.
- Native `npm run tauri:dev` is blocked until Rust/Cargo is installed.
