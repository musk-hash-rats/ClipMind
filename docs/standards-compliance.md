# Standards Compliance Pass

This document tracks ClipMind against the studio security and coding standards gate. It should be updated whenever the standard changes or a release boundary moves.

## Current Verdict

Status: not release-compliant yet.

ClipMind has an approved desktop direction, security planning docs, shared TypeScript contracts, and passing frontend build checks. It does not yet have the native security implementation needed for release.

## Coding Standards Gate

Required before feature branches merge:

- `npm run check` passes locally and in CI.
- TypeScript compile errors are treated as blockers.
- Production dependency audit has no high or critical vulnerabilities.
- Generated folders such as `node_modules`, `dist`, and `src-tauri/target` stay out of Git.
- Product decisions that affect storage, capture, export, or privacy are reflected in `docs/`.

Current coverage:

- Implemented: TypeScript typecheck.
- Implemented: frontend production build.
- Implemented: production dependency audit.
- Implemented: GitHub CI workflow for main and pull requests.
- Pending: linting/formatting policy after the team chooses ESLint, Biome, or another standard.
- Pending: automated Rust checks once Rust/Cargo is available in the build environment.

## Security Standards Gate

Required before alpha release:

- Clipboard capture must be opt-in and pauseable.
- The current capture state must be visible in the UI.
- Copied payloads must be encrypted at rest.
- Private metadata such as URLs, file paths, sender identity, and window titles must be protected when sensitive.
- Masked clips must not be exported to agents unless the user explicitly reveals or includes them.
- Agent exports must include only selected clips and must create audit events.
- Panic wipe must affect ClipMind-owned data only and must not delete unrelated user files.
- macOS and Windows permission prompts must be documented and handled in product UI.

Current coverage:

- Implemented in contract/docs: capture states, session grouping rules, privacy flags, source confidence, export boundary, audit event shape.
- Implemented in UI prototype: visible capture state, masking/reveal controls, selected clip export staging, panic-wipe pending state.
- Pending native implementation: clipboard listener, encrypted payload store, key management, metadata encryption, audit log persistence, panic wipe execution.
- Pending installer hardening: code signing, macOS notarization, Windows signing, release artifact provenance.

## Standards Source

The team referenced studio standards from Discord channel `<#1512929541897846815>`. The exact channel content was not available in this agent turn, so this pass is based on the repo, the approved ClipMind contract, and the security/coding requirements discussed in this project channel.

When the standards are pasted or made available to the repo, this document should be reconciled line by line against that source.
