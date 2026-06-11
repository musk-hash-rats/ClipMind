# Release Security Checklist

ClipMind is not release-ready until every required item is complete or explicitly waived by the project owner.

## Required Before Alpha

- [ ] `npm run check` passes.
- [ ] CI continuous checks pass on `main`.
- [ ] Dependency review passes for pull requests.
- [ ] CodeQL completes without high-severity findings.
- [ ] Native clipboard capture is opt-in and pauseable.
- [ ] Capture state is always visible.
- [ ] Clip payloads are encrypted at rest.
- [ ] Key management is implemented for macOS and Windows.
- [ ] Private metadata handling is documented and implemented.
- [ ] Panic wipe is scoped to ClipMind-owned data only.
- [ ] Agent export requires explicit selected clips/sessions.
- [ ] Agent export creates an audit event.
- [ ] Masked clips are not exported unless explicitly revealed or included.
- [ ] No logs include payloads, secrets, tokens, or private metadata.
- [ ] Accessibility pass covers keyboard navigation, focus states, labels, and contrast.

## Required Before Public Release

- [ ] macOS installer is signed and notarized.
- [ ] Windows installer is signed.
- [ ] Release artifacts include provenance or checksums.
- [ ] Vulnerability reporting path is published.
- [ ] Privacy policy reflects actual local storage/export behavior.
- [ ] Final standards-compliance review is complete.
