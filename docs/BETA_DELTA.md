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
  OpenAPI, backup, single-node performance, tenant recovery, ANN release
  evidence, real-embedding readiness, and replication partition evidence in
  `target/production-evidence/report.json`.
- Real-domain embedding evidence for `investment_projects` has an endpoint-
  backed `BAAI/bge-m3` run with a packaged local baseline and
  `production_safe=true`. The gate remains local-only until beta to avoid
  scheduled embedding spend and GitHub-hosted provider secrets.

## Experimental Or Guarded

- ANN/HNSW is guarded by recall/latency reports, `production_safe` checks,
  release evidence packages, exact fallback, persisted multi-layer graph
  metadata, and `ef_construction` reporting. It is not yet promoted to
  unrestricted production search.
- Real distributed consensus has Raft-like primitives, replicated log recovery,
  partition tests, snapshot transfer, membership rotation, repair workers,
  repeatable local hardening gates, and documented SLO targets. It still needs
  sustained multi-process failover/rejoin evidence before production rollout.
- The web UI is a developer dashboard with build, smoke, package, and
  screenshot gates. It is not yet a full product UI for incidents, permissions,
  or operational workflows.
- Search quality uses alpha-grade lexical/vector/hybrid foundations and
  guarded ANN reports. Critical recall-sensitive workloads should keep exact
  fallback enabled.

## Blocked Before Beta Promotion

- Real-domain ANN/HNSW baseline now has a local `investment_projects` corpus,
  query set, ground truth, endpoint-backed benchmark run, and packaged baseline.
  Beta promotion still requires repeated runs across stable environments and
  real traffic SLO history. The repeatable gate is local for Core Alpha; required
  environment variables are `CORTEXDB_EMBEDDING_URL`,
  `CORTEXDB_EMBEDDING_MODEL`, and `CORTEXDB_EMBEDDING_API_KEY` when the provider
  requires authentication. GitHub Actions promotion is deferred to beta.
- SDK publication needs a regular release train, package registry credentials,
  version lock-step, and changelog/deprecation policy enforcement on every
  public release.
- Product UI beta needs role-aware flows, stable error presentation, incident
  visibility, and broader browser/e2e regression evidence.
- Consensus beta needs sustained partition/failover/rejoin evidence that meets
  the SLO thresholds in `docs/CONSENSUS_SLO.md` for leader failover, replay,
  repair completion, snapshot handoff, and topology reload.

## Required Gates

Run these before claiming a beta-ready delta:

```bash
cargo check --workspace
cargo test --workspace --all-features
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
make openapi-contract-check
make beta-foundation-check
make beta-rc-check
make sdk-check
make production-evidence-sweep
make ann-real-embedding-readiness
make beta-delta-check
```

`make ann-real-embedding-readiness` may return a blocked readiness report when
embedding endpoint prerequisites are absent. That is an acceptable Core Alpha
state, but not a beta promotion state.

`make beta-foundation-check` is the focused local Epic 2 gate. It aggregates
SDK contract, OpenAPI contract, ContextPack/VERIFY quality fixtures, search
quality, error taxonomy, metrics contract, and beta boundary checks into
`target/beta-foundation/report.json`.

`make beta-rc-check` is the focused local Epic 3 gate. It aggregates the beta
foundation gate, backup/restore drills, offsite staging, security/auth tests,
ingestion job tests, dashboard release packaging, and operational docs checks
into `target/beta-rc/report.json`.
