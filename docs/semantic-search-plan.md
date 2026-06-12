# Semantic Search Plan

Semantic search now has a first local implementation. It uses a deterministic local hash-vector index stored encrypted in the ClipGuard store. This is not a neural embedding model yet, but it provides the local encrypted index, rebuild flow, exact/semantic UI split, and locked redaction boundary needed for the product path.

## Privacy Model

- Semantic indexes stay local by default.
- Local-only clips must not be sent to remote embedding providers.
- Masked clips may be indexed only while unlocked and only into the local encrypted index.
- File paths, sender identity, URLs, and window titles are private metadata and must be redacted or encrypted consistently with clip payload policy.

## Index Shape

- Store one vector record per clip payload revision.
- Keep embedding records keyed by `clipId`, `payloadId`, `updatedAt`, and embedding model/version.
- Persist the index encrypted with the same wrapped data-key boundary as clip payloads.
- Do not index burn-after-use clips after they are consumed.

## Rebuild Flow

- Rebuild is explicit from settings or automatic after embedding model/version changes.
- Rebuild requires the app to be unlocked.
- Rebuild progress should be visible and cancelable.
- Failed rebuilds must leave the previous index intact until a full replacement is ready.

## Search UX

- Keep exact search as the default trusted mode.
- Add a segmented control for `Exact` and `Semantic` after semantic search exists.
- Label semantic results with the matching mode and score/rank only when the score is real.
- Locked mode must never expose semantic snippets, private metadata, or payload-derived terms.

## Open Decisions

- Choose whether to replace `clipguard-local-hash-v1` with a local neural embedding runtime.
- Decide whether remote embeddings are ever allowed for non-local-only clips.
- Define max indexed payload size for large files and screenshots.
- Add integration tests for index rebuild, locked redaction, local-only policy, and model-version migration.
