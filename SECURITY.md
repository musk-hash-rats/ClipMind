# Security Policy

ClipGuard is early-stage and not release-compliant yet.

## Supported Versions

No public versions are supported yet. Security review is required before alpha release.

## Reporting A Vulnerability

For now, report vulnerabilities through the project maintainers in the studio workspace. Do not post sensitive exploit details in public channels.

Include:

- Affected area.
- Steps to reproduce.
- Expected result.
- Actual result.
- Data exposure or user impact.
- Suggested fix, if known.

## Security Expectations

- Clipboard payloads must be treated as sensitive by default.
- Clip payloads must not be logged.
- Masked content must stay masked across previews, search, and export unless the user explicitly reveals it.
- Agent handoff must include only selected context.
- Panic wipe must affect ClipGuard-owned data only.
