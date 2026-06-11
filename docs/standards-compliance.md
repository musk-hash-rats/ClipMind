# Standards Compliance Pass

This document tracks ClipMind against the studio security and coding standards gate. It should be updated whenever the standard changes or a release boundary moves.

## Current Verdict

Status: not release-compliant yet.

ClipMind has an approved desktop direction, security planning docs, shared TypeScript contracts, and passing frontend build checks. It does not yet have the native security implementation needed for release.

## Standards Source

Standards were pulled from Discord channel `<#1512929541897846815>` on 2026-06-11.

Studio mission:

- Create memorable games, software, machinima, and digital experiences that combine creativity, quality, security, and innovation.
- Build products the studio would proudly use and confidently share with players, customers, and partners.

Core goals applied to ClipMind:

- Create a unique, consumer-friendly clipboard working-memory experience.
- Ship finished desktop software instead of stopping at prototypes.
- Build secure software from the start.
- Maintain professional engineering standards.
- Learn from each project pass and preserve decisions in docs.
- Build user trust through privacy, security, and reliable release practices.
- Keep the work creative and enjoyable while still treating release quality seriously.

Applicable frameworks and rules:

- NIST CSF, NIST SSDF, NIST SP 800-53, NIST SP 800-218.
- OWASP Top 10, API Security Top 10, ASVS, SAMM, MASVS, Mobile Top 10.
- Privacy by Design, data minimization, least privilege, encryption for sensitive data at rest and in transit.
- ISO/IEC 12207, 25010, 29119, 27034, 27001, 27002, 27017, 27018, 27701.
- ISO 9241, WCAG 2.2, W3C HTML/CSS/accessibility/semantic markup.
- Apple Security Guidelines.
- Google Android Security Best Practices.
- Local engineering rules: security by default, privacy by default, finish before expanding, maintainable code, decision docs, important testing, quality over quantity, no ego development, user trust, and leave it better than found.

Studio motto: Creative by nature. Secure by design. Built to last.

Engineering rules:

1. Security by default: every feature must assume it may be attacked.
2. Privacy by default: collect the minimum user data required.
3. Finish before expanding: complete core functionality before adding extras.
4. Maintainable code: another developer should understand it six months later.
5. Document important decisions: architecture, workflows, and design choices need a trail.
6. Test everything important: critical systems require automated or documented testing.
7. Quality over quantity: a few polished products beat many unfinished ones.
8. No ego development: the best solution wins, regardless of who proposed it.
9. Respect the user: never knowingly ship software that harms security, privacy, or trust.
10. Leave it better than you found it: every commit should improve the product, codebase, or documentation.

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
- Implemented: dependency review and CodeQL workflow.
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

## Framework Mapping

NIST SSDF / SP 800-218:

- Plan and document security requirements: started in `docs/security-storage.md`, `docs/threat-model.md`, and this file.
- Protect source and build pipeline: started with CI checks, dependency review, and CodeQL.
- Produce well-secured software: pending native encryption, key management, permission checks, and audit persistence.
- Respond to vulnerabilities: started with `SECURITY.md`; release process still needs triage ownership and SLA approval.

OWASP / ASVS / SAMM:

- Access control: pending app lock, permission boundary, and explicit agent export confirmation.
- Cryptography: pending encrypted store and platform key management.
- Logging: planned audit events; must avoid payload/secrets in logs.
- Supply chain: started with `npm audit`, dependency review, and lockfile.

Privacy by Design / ISO 27701:

- Data minimization: product docs require local-only MVP and selected exports.
- Purpose limitation: sessions and agent handoff are explicit.
- User control: capture pause, masking, burn-after-use, expiry, and panic wipe are required.

ISO 25010 / ISO 29119:

- Quality attributes covered in planning: security, usability, maintainability, reliability.
- Testing is currently minimal; must expand from typecheck/build into unit, integration, native clipboard, storage, and destructive-action tests.

WCAG 2.2 / ISO 9241:

- Current UI uses semantic regions and visible states.
- Pending: keyboard navigation audit, focus states, contrast audit, reduced-motion handling, screen-reader pass.
