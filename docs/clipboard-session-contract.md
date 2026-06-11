# Clipboard And Session Contract

This contract describes how native clipboard capture should connect to work sessions.

## Capture States

- `active`: new clipboard items are captured and attached to the active session.
- `paused`: no new clipboard items are captured.
- `stopped`: no active work session exists.

The UI must make capture state visible at all times.

## Capture Rules

1. Ignore duplicate clipboard contents within a short debounce window.
2. Never capture while paused.
3. Attach captured clips to the active session.
4. If no session is active, queue the clip for manual session assignment or skip capture based on user setting.
5. Apply default privacy rules before showing previews.
6. Store source metadata with confidence and fallback reason.

## Session Corrections

Users must be able to move a clip to a different session after capture. Corrections should not alter the original capture timestamp.

## Platform Notes

macOS:

- Requires permission-aware clipboard access.
- Menu bar control should mirror capture state.
- Window/app source data may be limited.

Windows:

- Native listener should handle clipboard format changes.
- System tray control should mirror capture state.
- Source window data may need fallback handling.

## Non-Goals For MVP

- Cloud sync.
- Shared team workspaces.
- Background capture when the app is locked and the user has disabled locked capture.
