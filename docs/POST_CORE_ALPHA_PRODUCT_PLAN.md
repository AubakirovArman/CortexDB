# Post-Core Alpha Product Plan

Core Alpha is treated as closed and published. This plan starts after the
durable single-node core and focuses only on the next product layer.

## Current Stage

```text
Core Alpha:
AQL + WAL + MVCC + checkpoint/compact + ContextPack + CLI/server + SDK draft
```

The next layer is:

```text
1. Production-grade HNSW/ANN
2. Real distributed consensus
3. Full web UI
4. Stable published SDK packages
```

## Recommended Order

```text
1. API/SDK contract freeze
2. Published SDK packages
3. Production ANN/HNSW
4. Full web UI
5. Real distributed consensus
```

API and SDK stability should come before UI and distributed work. ANN can move
in parallel, but consensus should wait until the single-node API and storage
contracts are stable.

## Milestone 1 - API/SDK Contract Freeze

Goal: make the external contract stable enough that SDKs and UI can build on it.

Tasks:

1. Keep REST API versioned under `/v1`.
2. Freeze typed JSON response shapes for public endpoints.
3. Freeze error response codes and safe messages.
4. Treat `docs/openapi.yaml` as the source of truth.
5. Add schema compatibility tests for Rust, Python, and TypeScript SDKs.
6. Add release compatibility notes for breaking and non-breaking changes.
7. Add live server smoke examples for each SDK.
8. Add changelog rules for API and SDK changes.
9. Add CI checks that fail when SDK contract fixtures drift.
10. Document deprecated fields before removal.

Definition of done:

```text
OpenAPI is stable.
SDK contract tests are green.
Breaking changes require version bump and release notes.
```

## Milestone 2 - Published SDK Packages

Goal: make SDKs installable and repeatably publishable.

Tasks:

1. Prepare Rust SDK package metadata for crates.io.
2. Add Rust SDK examples and docs.rs-ready documentation.
3. Prepare Python SDK wheel metadata and typed response models.
4. Add PyPI dry-run and tag-gated publish workflow.
5. Prepare TypeScript SDK package metadata for npm.
6. Decide and document ESM/CJS support policy.
7. Add npm dry-run and tag-gated publish workflow.
8. Align server, OpenAPI, and SDK versions.
9. Add SDK integration tests against a running server.
10. Add SDK quickstarts to docs.

Definition of done:

```text
cargo add / pip install / npm install path is documented.
SDKs pass live server smoke tests.
Publishing is tag-driven and repeatable.
```

## Milestone 3 - Production-Grade ANN/HNSW

Goal: move vector search from experimental to reliable.

Tasks:

1. Add persistent vector collection metadata:
   - dimension
   - metric
   - normalization policy
2. Enforce dimension and metric compatibility on the write path.
3. Keep validation checks for persisted `.acv` and `.ach` bundles.
4. Add cosine and L2 policies if they are part of the public API.
5. Add deterministic HNSW build planner.
6. Add background rebuild hooks without introducing hidden write races.
7. Add HNSW persistence compatibility tests.
8. Add recall benchmark fixtures.
9. Keep exact fallback for invalid graph, stale graph, and low recall.
10. Add ANN metrics:
    - recall
    - latency
    - graph nodes
    - deleted vectors
    - rebuild count
11. Add `cortexdb ann validate`.
12. Document ANN limitations and tuning parameters.

Definition of done:

```text
ANN survives restart/checkpoint/compact.
Recall tests pass against golden fixtures.
Dimension and metric mismatch cannot happen silently.
Exact fallback preserves correctness.
```

## Milestone 4 - Full Web UI

Goal: turn the dashboard into a real product UI.

Tasks:

1. Choose the frontend stack and repository layout.
2. Build a separate frontend app instead of growing server raw strings.
3. Add Overview page.
4. Add Cells page.
5. Add AQL Console page.
6. Add Search page.
7. Add ContextPack page.
8. Add Verify page.
9. Add Ingestion Jobs page.
10. Add Storage Validation page.
11. Add ANN Metrics page.
12. Add Cluster Status page.
13. Add auth/token handling.
14. Add request history and JSON inspectors.
15. Add clear error states.
16. Add Playwright smoke tests.
17. Add static asset build and serving path.
18. Add UI screenshots to docs.

Definition of done:

```text
Core Alpha operations can be run without CLI.
UI has e2e smoke tests.
Errors are visible and actionable.
```

## Milestone 5 - Real Distributed Consensus

Goal: replace experimental replication with a real consensus layer.

Tasks:

1. Write the consensus design document.
2. Choose the consensus model and explicitly list non-goals.
3. Separate local WAL, replication log, and consensus metadata.
4. Add persistent node identity.
5. Add cluster membership model.
6. Implement leader election.
7. Implement log replication.
8. Persist commit index safely.
9. Implement follower recovery.
10. Implement snapshot transfer.
11. Add network partition tests.
12. Add crash/restart cluster tests.
13. Add split-brain prevention tests.
14. Add unsafe-state detector and admin diagnostics.

Definition of done:

```text
3-node cluster survives leader restart.
Committed writes are not lost.
Split brain is prevented.
Snapshot transfer works.
```

## Non-Goals For This Plan

Do not reopen Core Alpha as part of this plan.

Not included here:

```text
Core WAL rewrite
Core MVCC rewrite
AQL parser rewrite
Storage format churn without migration policy
LLM integration
Document/OCR production ingestion
```

## Release Path

1. Finish API/SDK contract freeze.
2. Publish SDK packages as alpha packages.
3. Promote ANN/HNSW from experimental to guarded production mode.
4. Ship full web UI as an optional management surface.
5. Start real distributed consensus as the next major engineering track.

