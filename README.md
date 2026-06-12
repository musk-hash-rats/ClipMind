# ClipGuard Desktop

ClipGuard is a macOS and Windows desktop app for encrypted clipboard working memory. It ships as native desktop software, not as a hosted web app.

## Status

- Stack: Tauri v2 native shell + Rust backend + TypeScript UI.
- Target: macOS and Windows desktop first.
- Current state: scaffolded repo, interactive consumer UI shell, encrypted local text capture, sessions, search, text transforms, selected clip/session export, panic wipe, and audit trail.
- Local verification: Rust/Cargo is installed in this workspace, so native Rust checks can run here. Final macOS `.dmg` and Windows `.exe` installers still need native platform builds or GitHub Actions.

## Desktop Deliverables

The product target is installable desktop software:

- macOS: `.dmg`
- Windows: `.exe` installer

The TypeScript files are the app's desktop UI layer, rendered inside the Tauri native shell. The release artifacts are produced by the Tauri build pipeline, not by deploying the Vite dev server.

This Linux OpenClaw host can run the frontend dev server for quick preview, but it cannot produce the final macOS `.dmg` or Windows `.exe` locally. Those installers should be built on their native platforms, or through the GitHub Actions workflow in:

```text
.github/workflows/desktop-installers.yml
```

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

5. Build native installers on the matching OS:

```bash
npm run tauri:build
```

On GitHub, run the `Desktop installers` workflow manually or push a `v*` tag. The workflow uploads separate macOS and Windows installer artifacts.

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
- `docs/standards-compliance.md`
- `docs/threat-model.md`
- `docs/release-security-checklist.md`
- `SECURITY.md`

## Initial Ownership

- streets: GUI, desktop UX, prototype-to-app flow, task list/docs.
- rats_claude: architecture, encrypted storage, clipboard/session contract, agent handoff boundaries, security review.

## Agent Workflow Gate

ClipGuard work follows the shared OpenClaw routing protocol:

- Manager routes non-trivial app work and records acceptance criteria.
- Backend Builder owns implementation-heavy app, native bridge, storage, automation, and verification changes.
- Code Review must review meaningful code changes before the work is reported complete.
- Streets and rats_claude can coordinate, summarize, or handle narrow ownership tasks, but should call the specialist agents when work crosses into implementation or release confidence.
