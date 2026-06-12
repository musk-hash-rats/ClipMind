# ClipMind Agent Workflow

Status: active workflow correction, 2026-06-12.

## Why This Exists

ClipMind is no longer only a planning artifact. The implementation repo exists at `projects/clipmind-desktop`, and work in this repo must follow the studio's specialist-agent workflow.

The previous drift happened because GUI/app implementation started moving as a general task after planning docs were written, while the task split still said no repo existed. That made it easy to skip the manager/backend/review handoff.

## Required Routing

- Manager owns task decomposition, acceptance criteria, dependency tracking, and final integration summary.
- Streets owns Discord-visible status plus front-end UX work when explicitly assigned.
- Backend Builder owns Rust/Tauri backend code, encrypted storage, clipboard capture, schemas, migrations, exports, and verification scripts.
- Code Review owns final review of diffs, tests, security/privacy risks, accessibility, destructive operations, and release readiness.
- rats_claude owns security/storage architecture review and coordinates with Code Review instead of replacing it.

## Before Editing

1. Run `git status --short`.
2. Identify current uncommitted files and assume they may belong to another agent.
3. Read the relevant docs in `docs/` plus the current assignment in `projects/current_project/specs/clipmind-agent-task-split.md`.
4. Create or update a visible handoff before substantial changes.
5. If the work touches storage, clipboard capture, export, privacy, permissions, or release behavior, assign Backend Builder and Code Review explicitly.

## Done Means

- Owner and reviewer are named.
- Changed files are listed.
- Tests or verification commands are recorded.
- Security/privacy impact is stated.
- Review status is one of: not reviewed, changes requested, approved with notes, approved.
- Discord-facing summaries do not claim completion before the review gate.

## Handoff Template

```text
FROM: <agent>
TO: manager / backend_builder / code_review / rats_claude / streets
GOAL: <ClipMind outcome>
REASONING: <why this owner is responsible, what risk exists>
OUTPUT: <files changed or artifact path>
DELEGATED TASKS: <required follow-up owner or NONE>
BLOCKERS: <open questions or NONE>
REVIEW STATUS: not reviewed / changes requested / approved with notes / approved
```
