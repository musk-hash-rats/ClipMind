# ClipMind Data Contract

This contract defines the MVP data shapes that the UI, native capture layer, encrypted store, and agent export layer should share.

## Clip

A clip is a single captured clipboard item. It must be usable without losing privacy, source context, or session history.

Required fields:

- `id`: stable local identifier.
- `type`: `text`, `link`, `image`, `screenshot`, or `file`.
- `createdAt`: ISO timestamp.
- `updatedAt`: ISO timestamp.
- `sessionId`: active work session at capture time.
- `source`: app/window/url/file/sender origin data when available.
- `sourceConfidence`: `high`, `medium`, or `fallback`.
- `privacy`: sensitivity, masking, expiry, burn-after-use, local-only flags.
- `content`: encrypted payload reference plus safe preview data.
- `transforms`: generated outputs and transform history.

## Source Memory

Source metadata should be preserved separately from clip content so the app can show provenance without exposing the full payload.

Source fields:

- `appName`: browser, editor, terminal, files, messages, or unknown.
- `windowTitle`: optional user-visible window title.
- `url`: optional source URL.
- `filePath`: optional local path or filename.
- `sender`: optional sender/origin identity for copied messages or shared content.
- `deviceId`: local device identifier.
- `capturedVia`: `clipboard-listener`, `manual-import`, `drag-drop`, or `agent-import`.
- `fallbackReason`: why origin data is incomplete.

## Work Session

A work session groups copied material into a task context.

Session fields:

- `id`
- `title`
- `createdAt`
- `updatedAt`
- `captureState`: `active`, `paused`, or `stopped`
- `defaultPrivacy`
- `clipCount`
- `lastClipAt`

## Agent Export

Agent exports must be explicit and auditable.

Export fields:

- `id`
- `sessionId`
- `clipIds`
- `format`: `markdown` or `json`
- `createdAt`
- `destination`: selected agent or local file
- `includedFields`
- `redactions`
- `userConfirmed`

Every export should create an audit event.

## Audit Event

Audit events track security-sensitive actions.

Events:

- app unlocked
- capture started or paused
- clip masked or revealed
- clip expired
- burn-after-use clip consumed
- panic wipe requested
- panic wipe completed
- agent export created

Audit fields:

- `id`
- `type`
- `createdAt`
- `actor`: local user or system
- `targetId`
- `summary`
- `metadata`
