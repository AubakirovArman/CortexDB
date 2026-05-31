# Implementation Tasks From `/mnt/hf_model_weights/arman/3bit/sites/pl.md`

Source plan reviewed: 2026-05-31.

This file normalizes the external `pl.md` audit into a current CortexDB action
list. The source plan is useful, but several findings are already closed in the
current repository. Treat this file as the executable follow-up list.

## Current Correction

Already present in the repo and not a fresh blocker:

- current architecture docs: `ARCHITECTURE.md`, `docs/ARCHITECTURE.md`;
- security docs: `docs/SECURITY_MODEL.md`, `docs/SECURITY_THREAT_MODEL.md`;
- backup/restore docs and gates: `docs/BACKUP_RESTORE.md`, `make backup-drill-check`;
- AQL v0.4 docs: `docs/AQL_V0_4.md`;
- Context Pack docs: `docs/CONTEXT_PACK.md`, `docs/CONTEXT_PACK_TECHNOLOGY.md`;
- API error taxonomy: `docs/API_ERROR_TAXONOMY.md`;
- OpenAPI and typed response gates: `make openapi-contract-check`;
- SDK release procedure and deprecation policy docs;
- metrics docs: `docs/METRICS.md`;
- upgrade/migration docs: `docs/UPGRADE_MIGRATION.md`;
- public-claim and beta-delta gates.

Still open as product/beta work:

- repeatable production-evidence runs on clean environments;
- long-running ANN/HNSW real-domain history and SLO calibration;
- public SDK registry release train;
- product-grade web UI beyond developer console;
- real distributed consensus rollout hardening;
- stronger operational security posture: RBAC, audit review, incident runbooks.

## P0 - Evidence Sweep And Current Reality Check

Goal: prove the current Core Alpha state with machine-readable local evidence.

What to do:

1. Run the broad evidence sweep:

   ```bash
   make production-evidence-sweep
   ```

2. Inspect:

   ```text
   target/production-evidence/report.json
   target/backup-drill/report.json
   target/single-node-performance/report.json
   target/tenant-recovery/report.json
   ```

3. If any gate fails, fix the concrete failing subsystem instead of changing
   the report.

Files likely involved:

- `Makefile`
- `scripts/*`
- `crates/cortex-engine`
- `crates/cortex-server`
- `docs/BETA_DELTA.md`
- `docs/REMAINING_EXECUTION_PLAN.md`

Done when:

- `make production-evidence-sweep` passes locally;
- report paths exist;
- `docs/BETA_DELTA.md` still matches the actual results.

## P0 - API And SDK Contract Proof

Goal: ensure clients can rely on stable HTTP and SDK contracts.

What to do:

1. Run:

   ```bash
   make openapi-contract-check
   make sdk-check
   ```

2. Confirm SDK docs and examples cover:

   - put/get;
   - search;
   - Context Pack;
   - Verify Fact;
   - tenant routing;
   - auth token usage.

3. If examples are missing, add them to:

   - `docs/SDK_QUICKSTART.md`
   - `sdk/README.md`
   - `sdk/python/README.md`
   - `sdk/typescript/README.md`

Done when:

- OpenAPI contract check passes;
- SDK checks pass;
- public registry publication remains documented as beta-stage unless actually
  released.

## P0 - Context Pack Quality Lock

Goal: keep Context Pack as the main product differentiator with reproducible
quality evidence.

What to do:

1. Run:

   ```bash
   make context-verify-quality-check
   ```

2. Review the fixture:

   ```text
   crates/cortex-engine/fixtures/context_verify_quality_v1.cells
   ```

3. Add or update cases only when they represent real agent behavior:

   - budget truncation;
   - missing citation;
   - redundant evidence;
   - numeric conflict preservation;
   - scope isolation;
   - checkpoint/restart stability.

Files likely involved:

- `crates/cortex-engine/src/context`
- `crates/cortex-engine/tests/context_pack.rs`
- `crates/cortex-engine/tests/context_verify_quality.rs`
- `docs/CONTEXT_PACK.md`
- `docs/CONTEXT_PACK_TECHNOLOGY.md`

Done when:

- gate passes;
- JSON contract stays stable;
- docs explain any changed anomaly or explain field.

## P0 - ANN/HNSW Guarded Production Evidence

Goal: keep ANN/HNSW guarded and measurable, not silently promoted.

What to do:

1. Run the local gates:

   ```bash
   make ann-fixture-check
   make ann-drift-check
   make ann-external-check
   make ann-metric-matrix-check
   make ann-real-embedding-readiness
   ```

2. For local real-domain runs, use environment variables only:

   ```text
   CORTEXDB_EMBEDDING_URL
   CORTEXDB_EMBEDDING_MODEL
   CORTEXDB_EMBEDDING_API_KEY
   ```

3. Do not re-add hosted provider secrets or scheduled real-embedding runs to
   GitHub Actions until beta.

Files likely involved:

- `docs/ANN_PRODUCTION_TUNING.md`
- `docs/ANN_PUBLIC_CORPUS_RUNS.md`
- `docs/BENCHMARKS.md`
- `examples/real_domains/investment_projects`
- `crates/cortex-engine/src/search`

Done when:

- local reports show `production_safe=true`;
- exact fallback remains documented for critical workloads;
- repeated real-domain runs are archived locally before beta promotion.

## P0 - Security And Operations Baseline

Goal: keep the project safe to expose locally without pretending it is
production-hardened.

What to do:

1. Run:

   ```bash
   make tenant-recovery-check
   make backup-drill-check
   make crash-fault-check
   make chaos-restart-check
   ```

2. Review:

   - route auth coverage;
   - tenant path validation;
   - audit log behavior;
   - rate limit behavior;
   - backup/restore evidence;
   - repair behavior with stale locks and partial WAL tails.

Files likely involved:

- `crates/cortex-server/src/auth.rs`
- `crates/cortex-server/src/audit.rs`
- `crates/cortex-server/src/router.rs`
- `crates/cortex-engine/src/backup*`
- `crates/cortex-engine/src/repair*`
- `docs/AUTH.md`
- `docs/OPERATIONS.md`
- `docs/SECURITY_MODEL.md`

Done when:

- local gates pass;
- docs state the current security model and non-goals;
- no secrets are committed.

## P1 - Product UI Hardening

Goal: move from developer dashboard to product-grade UI.

What to do:

1. Run current dashboard gates:

   ```bash
   make dashboard-check
   make dashboard-release-check
   ```

2. Improve only product-critical flows first:

   - auth/session UX;
   - tenant selection;
   - error surfaces;
   - validation/repair views;
   - Context Pack explain view;
   - ANN evaluation report view.

Files likely involved:

- `web/dashboard/src`
- `crates/cortex-server/assets/dashboard/v1`
- `docs/DASHBOARD_UI.md`

Done when:

- route-level smoke passes;
- screenshot/visual artifacts are stable enough for review;
- UI docs clearly say developer console vs product UI.

## P1 - SDK Release Train

Goal: make SDK releases repeatable without surprise contract drift.

What to do:

1. Keep version lock-step:

   ```text
   server version
   OpenAPI version
   Rust crate version
   Python package version
   TypeScript package version
   changelog entry
   ```

2. Run:

   ```bash
   make sdk-check
   ```

3. Public registry publishing is a beta-stage action. Until then, package
   locally and document dry-runs.

Files likely involved:

- `sdk/release-manifest.json`
- `docs/SDK_RELEASE.md`
- `docs/SDK_DEPRECATION_POLICY.md`
- `.github/workflows/*sdk*`
- `crates/cortex-sdk`
- `sdk/python`
- `sdk/typescript`

Done when:

- package dry-runs pass;
- changelog/deprecation policy is enforced;
- release ownership and credentials are explicitly available before publish.

## P1 - Consensus Hardening

Goal: continue from Raft-like primitives toward real operational consensus.

What to do:

1. Keep existing replication gates green.
2. Add evidence around long-running scenarios:

   - repeated split-brain/rejoin;
   - follower lag repair;
   - snapshot handoff under restart;
   - membership rotation resume;
   - operator topology reload.

Files likely involved:

- `crates/cortex-engine/src/replication*`
- `crates/cortex-engine/tests/replication_*`
- `docs/CONSENSUS_DESIGN.md`
- `docs/REPLICATION.md`

Done when:

- consensus docs clearly separate experimental model from production rollout;
- failure-mode tests are repeatable;
- SLO targets for failover/repair are documented before beta claims.

## P2 - Production Candidate Work

Goal: prepare a single-node production candidate after beta evidence is stable.

What to do later:

1. Add longer load/soak runs.
2. Add migration compatibility tests for storage/API/SDK.
3. Add audit-log review tooling.
4. Add RBAC/AgentView persistence for server authz.
5. Add binary install docs for Linux/macOS.
6. Add release artifacts and checksums.

Do not start this before P0/P1 gates are stable.

## Do Not Do Now

- Do not claim CortexDB is production-ready.
- Do not re-enable real embedding GitHub secrets or scheduled provider spend.
- Do not publish SDKs publicly without a release owner, credentials, and
  version lock-step evidence.
- Do not remove exact vector fallback from critical ANN paths.
- Do not make storage format changes without migration docs and compatibility
  tests.
- Do not turn Context Pack into an LLM-calling subsystem inside the DB core.

## Minimum Verification For This Task List

For doc-only changes to this plan:

```bash
make beta-delta-check
make public-claims-check
cargo fmt --check
cargo check --workspace
```

For implementation changes:

```bash
cargo check --workspace
cargo test --workspace --all-features
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
make openapi-contract-check
```

For release readiness:

```bash
make production-evidence-sweep
make sdk-check
make dashboard-release-check
```
