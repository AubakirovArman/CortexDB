# CortexDB Future Product Plan (Post-Core Alpha)

Status: Core Alpha is stabilized and published. The following are the next execution layers.

## Current Production Parity Position

Compared with a classic Redis-like product, CortexDB currently offers:

- durable single-node storage with WAL + MVCC,
- checkpoints/compaction,
- permission-aware retrieval (AQL + lexical/vector foundations),
- verification + ContextPack + typed CLI/server/API,
- initial SDK contracts.

What is still missing vs Redis at this stage:

- no distributed consensus cluster mode,
- search and ANN are foundation-grade, not production-tuned,
- no full web UI.
- backup/restore now has local restore drills, retention pruning, and a
  repeatable `make backup-drill-check` evidence artifact, but still needs
  external offsite target automation.
- crash/fault evidence now has `make crash-fault-check` plus CI artifact
  upload, and repeatable process-level kill/restart evidence now has
  `make chaos-restart-check`. Longer randomized soak campaigns remain future
  hardening.

## Milestone A — API/SDK Runtime Freeze (next)

Goal: make API and SDKs stable enough for external consumers.

- Freeze REST + typed response contracts.
- Add semantic versioned OpenAPI + changelog policy.
- Keep `openapi.yaml` as source of truth, run sync checks in CI.
- Expand SDK contract tests against live server.
- Publish all SDKs through repeatable release pipeline.

## Milestone B — Production-Grade ANN/HNSW

Goal: move vector search from experimental to guarded production mode.

- Finalize collection-level ANN metadata (`.ach` schema v2).
- Add deterministic, repeatable HNSW index rebuild policy and recall baselines.
- Add golden recall fixtures and regression dashboard.
- Add safety guard that blocks serving degraded ANN result sets when recall drops.
- Add observability metrics: recall, latency, graph rebuild count, stale graph handling.

## Milestone C — Full Web UI

Goal: provide a product surface for operations and debug.

- Standalone frontend app (not server-string HTML).
- Core pages: dashboard, cells, AQL console, search/context, verify, ingest, storage health.
- Auth + tenant scope handling.
- Error and recovery views with WAL/segment diagnostics.
- E2E smoke tests with Playwright.

## Milestone D — Real Distributed Consensus

Goal: move from local consensus model to durable production-grade replication.

- Define and document consensus model and failure assumptions.
- Split local WAL from consensus log and commit metadata.
- Implement leader election + log replication + snapshot install.
- Add partition/failover tests and split-brain prevention checks.
- Add node membership, recovery, and safety invariants docs.

## Execution Sequence

1) API/SDK freeze
2) ANN/HNSW hardening
3) Web UI
4) Real distributed consensus

## Definition of Done for milestone handoff

- Green alpha-check.
- Contract checks for API + SDK.
- Replay/restore proof via restart + corruption matrix tests.
- CI + smoke tests for each milestone objective.
