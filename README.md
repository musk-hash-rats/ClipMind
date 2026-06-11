# ClipMind Desktop

ClipMind is a macOS and Windows desktop app for encrypted clipboard working memory.

## Status

- Stack: Tauri v2 + TypeScript UI.
- Target: macOS and Windows desktop first.
- Current state: scaffolded repo, interactive consumer UI shell, native command stubs.
- Blocker: Rust/Cargo is not installed in this workspace, so Tauri builds cannot run here yet.

## Setup

1. Install Rust from https://rustup.rs.
2. Install frontend dependencies:

```bash
npm install
```

3. Run the desktop app:

```bash
npm run tauri:dev
```

4. Run frontend checks:

```bash
npm run typecheck
npm run build
```

## Product Contract

The approved product contract is stored in Google Drive under `Clip_Mind/ClipMind_Code_Contract.txt` and locally at:

```text
/root/.openclaw/workspace/ClipMind_Code_Contract.txt
```

The implementation task list is stored at:

```text
/root/.openclaw/workspace/projects/current_project/specs/clipmind-code-task-list.md
```

Implementation tickets are tracked at:

```text
/root/.openclaw/workspace/projects/clipmind-desktop/docs/implementation-tickets.md
```

Project architecture notes:

- `docs/architecture.md`
- `docs/data-contract.md`
- `docs/security-storage.md`
- `docs/clipboard-session-contract.md`

## Initial Ownership

- streets: GUI, desktop UX, prototype-to-app flow, task list/docs.
- rats_claude: architecture, encrypted storage, clipboard/session contract, agent handoff boundaries, security review.
