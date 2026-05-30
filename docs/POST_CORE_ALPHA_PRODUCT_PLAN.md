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

## Milestone 3 - Guarded ANN/HNSW Alpha

Goal: move vector search from experimental to guarded alpha with exact-fallback guardrails.

Status: **Guarded production controls landed; large-scale tuning remains open**.

What landed:

1. ✅ Persistent vector collection metadata stored in `.ach` files (dimension, metric).
2. ✅ Dimension enforcement on the write path (`HnswIndex::add_vector` returns `EngineError` on mismatch).
3. ✅ Validation checks for persisted `.acv` and `.ach` bundles.
4. ✅ Cosine and L2 distance policies (`DistanceMetric` enum with `DotProduct`, `Cosine`, `L2`) — all fixed-point, no `f64`.
5. ✅ Deterministic HNSW build planner (`hnsw_graph_for_cells` processes segments in commit order).
6. ✅ Background rebuild via `compact()` — graphs are rebuilt atomically during compaction without write races.
7. ✅ HNSW persistence compatibility tests (backward-compatible `.ach` decode with optional metadata trailer).
8. ✅ Recall benchmark fixtures in `core_baseline.rs` (`ann_recall_q16_1k`, `ann_eval_latency_1k`).
9. ✅ Exact fallback for invalid graph, stale graph, and low recall (`MIN_ANN_RECALL_Q16` = 75%).
10. ✅ Extended ANN metrics:
    - `deleted_vectors` — computed from live segment tombstones
    - `rebuild_count` — tracked on `HnswIndex`
    - `graph_nodes` / `total_edges` / `persisted_segments` — existing
11. ✅ `cortexdb ann validate` CLI command.
12. ✅ ANN limitations and tuning parameters documented in `SEARCH.md`.
13. ✅ `require_slo`, `production_safe`, `fallback_performed`, and `slo_violations` are exposed in engine, CLI, HTTP, OpenAPI, and SDK contracts.
14. ✅ Fast recall fixture gates assert checkpointed ANN evaluation meets `MIN_ANN_RECALL_Q16`.
15. ✅ Deterministic multi-layer HNSW graph links persist in `.ach` optional upper-layer trailers.
16. ✅ Repeatable `ann_repeatable_report_json` benchmark output records recall, latency, graph edges, and upper-layer counts.
17. ✅ Release-mode ANN fixture gate compares observed recall/latency and multi-layer graph shape against `ann_fixture_baseline_v1.json`.
18. ✅ CI uploads `ann_fixture_report.json` so ANN recall/latency drift can be inspected between commits.
19. ✅ ANN drift baseline gate fails on recall loss, graph-shape loss, or latency regression beyond budget.
20. ✅ External ANN JSONL fixture gate verifies non-generated vectors and named queries.
21. ✅ ANN metric matrix gate verifies dot-product, cosine, and L2 against exact top-k on the same fixture.
22. ✅ `ann_corpus_check` can evaluate larger external vectors/queries/ground-truth JSONL suites.
23. ✅ ANN helper scripts generate exact ground truth and compare corpus reports without extra dependencies.
24. ✅ `run_external_corpus.sh` orchestrates ground-truth generation, ANN evaluation, optional baseline comparison, and run artifact archival.
25. ✅ `ANN_PRODUCTION_TUNING.md` defines corpus readiness levels, thresholds, fallback policy, and release blockers.
26. ✅ `summarize_history.py` writes corpus-run history with adjacent recall, latency, graph-shape, and production-safety regression summaries.

What remains before broad production tuning:

- Collection-level metadata (not just per-segment `.ach` trailer).
- Checked-in or archived sift/glove-style golden reports generated through `ann_corpus_check`.
- Benchmark history tracking across commits.
- Tuned `ef_construction` and larger external-corpus parameter sweeps.

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
