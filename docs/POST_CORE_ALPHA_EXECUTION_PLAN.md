# Post-Core Alpha Execution Plan (CortexDB)

> Status note: this is a milestone planning document. For current public status,
> beta blockers, and what is stable now, use [`BETA_DELTA.md`](BETA_DELTA.md)
> and [`REMAINING_EXECUTION_PLAN.md`](REMAINING_EXECUTION_PLAN.md).

Current state: Core Alpha data+query path is stable and test-green. Next work should stay roadmap-first, not feature-first.

## Milestone X — Production-Grade ANN/HNSW

Goal: vector search can be used under production guardrails without silent recall regressions.

### In scope
1. `ANNPolicy` hardening
   - enforce graph freshness checks before serving ANN-only results
   - mandatory fallback on recall/health violations
   - bounded scan caps and latency budgets
2. `HNSW` guardrails
   - deterministic build/rebuild scheduling
   - recall regression thresholds with alerting
   - reject serving stale graph when policy says strict
3. Recall-diff + benchmark harness
   - nightly snapshot of `ann_metrics`
   - compare against exact baseline on fixed fixture set
4. API surface
   - expose recall/elapsed/scan stats in `ann_report`
   - add `search-vector` policy response flags in CLI/server
5. Failure modes
   - deterministic behavior for empty/corrupt graph (fallback exact)

### Exit criteria
- recall-lost tests + synthetic corpus benchmarks passing
- explicit fallback decision logged and observable
- no silent result shrink below policy threshold

## Milestone Y — Real Distributed Consensus

Goal: replace local single-node replication assumptions with durable replicated log semantics.

### In scope
1. Consensus core contract finalization
   - log/term/index invariants documented and tested
2. Log replication
   - leader election, append entries, commit index rules
   - snapshot transfer + follower catch-up
3. WAL integration
   - clear boundary between local WAL and replicated consensus log
   - idempotent replay from replication + local WAL
4. Topology management
   - node membership and stable node ids
   - observability for leader term/state
5. Failure handling
   - partition tests, stale leader handling, split-brain prevention

### Exit criteria
- integration tests for election, append-match, snapshot restore and replay idempotency
- documented consistency guarantees (read-your-writes, durability)

## Milestone Z — Standalone Web UI

Goal: replace static server HTML with a dedicated UI application.

### In scope
1. Scaffold frontend (independent SPA)
2. Screens: dashboard, cells, AQL, search/context, verification, ingest, health
3. Tenant-aware auth + scoped views
4. WAL/segment diagnostics views
5. E2E smoke tests for key flows

### Exit criteria
- UI can run against HTTP API without direct DB file access
- authenticated flows for create/search/validate/checkpoint/compact

## Milestone W — SDK/Contract Stabilization

Goal: publish stable Rust/TypeScript/Python SDK contracts.

### In scope
1. OpenAPI as contract source
2. Semantic versioning policy + deprecation windows
3. Typed request/response models per endpoint
4. CI contract test matrix across SDKs
5. Release packaging and changelog automation

### Exit criteria
- contract diff checks fail on unapproved changes
- publish pipeline for SDK artifacts + smoke tests

## Immediate implementation order
1. ANN guardrails + recall metrics (fastest/highest value)
2. API/SKD contract hardening around ANN fields
3. Consensus log split + membership + snapshot restore
4. Standalone web UI MVP

## Notes
- Core Alpha stability is a hard gate; all changes below must keep current `cargo test --workspace --all-features` green.
- Keep features behind explicit config flags during hardening.
