# CortexDB Beta Delta

This note separates the current Core Alpha guarantees from the remaining beta
work. It is intentionally conservative: do not promote a capability from
experimental to stable unless its release gate is repeatable and its
operational limits are documented.

## Stable Now

- Core Alpha single-node loop: WAL append, MVCC MemTable update, restart,
  replay, checkpoint, compact, validation, and repair evidence.
- AQL retrieval, ContextPack, and `VERIFY FACT` deterministic checks for the
  documented alpha contract.
- Typed HTTP API, OpenAPI contract checks, CLI flows, and Rust/Python/TypeScript
  SDK contract checks.
- Backup/restore drill evidence, offsite staging checks, crash/fault checks,
  and process-level chaos restart evidence.
- Production evidence sweep via `make production-evidence-sweep`, which records
  OpenAPI, backup, ANN release evidence, and replication partition evidence in
  `target/production-evidence/report.json`.

## Experimental Or Guarded

- ANN/HNSW is guarded by recall/latency reports, `production_safe` checks,
  release evidence packages, exact fallback, persisted multi-layer graph
  metadata, and `ef_construction` reporting. It is not yet promoted to
  unrestricted production search.
- Real distributed consensus has Raft-like primitives, replicated log recovery,
  partition tests, snapshot transfer, membership rotation, and repair workers.
  It still needs long-running failover and rejoin hardening before production
  rollout.
- The web UI is a developer dashboard with build, smoke, package, and
  screenshot gates. It is not yet a full product UI for incidents, permissions,
  or operational workflows.
- Search quality uses alpha-grade lexical/vector/hybrid foundations and
  guarded ANN reports. Critical recall-sensitive workloads should keep exact
  fallback enabled.

## Blocked Before Beta Promotion

- Real-domain ANN/HNSW baseline requires a domain corpus, query set, embedding
  endpoint, and model configuration. The readiness gate is
  `make ann-real-embedding-readiness`; required environment variables are
  `CORTEXDB_EMBEDDING_URL` and `CORTEXDB_EMBEDDING_MODEL`, with
  `CORTEXDB_EMBEDDING_API_KEY` required only when that deployment needs it.
- SDK publication needs a regular release train, package registry credentials,
  version lock-step, and changelog/deprecation policy enforcement on every
  public release.
- Product UI beta needs role-aware flows, stable error presentation, incident
  visibility, and broader browser/e2e regression evidence.
- Consensus beta needs sustained partition/failover/rejoin evidence with clear
  SLO thresholds for leader failover, replay, and repair completion.

## Required Gates

Run these before claiming a beta-ready delta:

```bash
cargo check --workspace
cargo test --workspace --all-features
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
make openapi-contract-check
make sdk-check
make production-evidence-sweep
make ann-real-embedding-readiness
make beta-delta-check
```

`make ann-real-embedding-readiness` may return a blocked readiness report when
external corpus or embedding endpoint prerequisites are absent. That is an
acceptable Core Alpha state, but not a beta promotion state.
