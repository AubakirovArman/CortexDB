# CortexDB Epic Execution Order

This document is the active execution queue. Treat every numbered item below as
an epic. Each epic may contain smaller tasks, but the project should close the
epics in order instead of jumping into unrelated deeper work.

## Working Rule

1. Work on the first epic that is not `done` or explicitly `blocked`.
2. Do not start a later epic just because a smaller task looks interesting.
3. If an epic is too broad, split it into tasks inside that epic, not into a new
   top-level roadmap.
4. Every implementation update should report:
   - current epic;
   - tasks completed;
   - tasks remaining;
   - next task;
   - risks or blockers.
5. A later epic can start only when the current epic is `done` or when the
   blocker is external and documented.

## Epic 1 - Production Evidence Sweep And Reality Check

Status: mostly done locally.

Goal: prove the current Core Alpha state with machine-readable evidence.

Tasks:

1. Run `make production-evidence-sweep`.
2. Confirm `target/production-evidence/report.json` exists.
3. Confirm the sweep includes:
   - OpenAPI contract;
   - backup drill;
   - single-node performance;
   - tenant recovery;
   - ANN release evidence;
   - real-embedding readiness;
   - replication partition evidence.
4. Record the latest passing evidence in the execution plan.
5. Repeat the sweep on a clean environment before beta/release claims.

Done when:

- `make production-evidence-sweep` passes.
- `docs/BETA_DELTA.md` and public docs match the report.
- The latest evidence commit/date is recorded.

Current evidence:

- Passed locally on `2026-05-31` at
  `36f70e8cf1d88293254d1f7c9133793dc057f313`.

## Epic 2 - API And SDK Contract Proof

Status: partial.

Goal: make HTTP and SDK contracts reliable for external consumers.

Tasks:

1. Run `make openapi-contract-check`.
2. Run `make sdk-check`.
3. Verify SDK docs cover:
   - put/get;
   - search;
   - Context Pack;
   - Verify Fact;
   - tenant routing;
   - bearer token auth.
4. Add live SDK smoke tests against a local `cortex-server` for Rust, Python,
   and TypeScript.
5. Keep public registry publishing beta-stage until credentials and release
   ownership are explicit.

Done when:

- OpenAPI contract check passes.
- SDK package/dry-run checks pass.
- Live SDK smoke covers the main API flows.
- Version lock-step and changelog policy are enforced.

## Epic 3 - Context Pack Quality Lock

Status: partial.

Goal: keep Context Pack as the main product differentiator with repeatable
quality evidence.

Tasks:

1. Run `make context-verify-quality-check`.
2. Review the quality fixture in
   `crates/cortex-engine/fixtures/context_verify_quality_v1.cells`.
3. Add real-agent behavior cases for:
   - budget truncation;
   - missing citations;
   - redundant evidence;
   - numeric conflicts;
   - scope isolation;
   - checkpoint/restart stability.
4. Stabilize anomaly codes and explain fields.
5. Document any changed Context Pack field in `docs/CONTEXT_PACK.md`.

Done when:

- Quality gate passes.
- JSON contract stays stable.
- Context Pack behavior is evaluated on a real-domain corpus, not only trivial
  synthetic cases.

## Epic 4 - ANN/HNSW Guarded Production Evidence

Status: partial, guarded by exact fallback.

Goal: keep ANN/HNSW measurable and safe from silent recall regressions.

Tasks:

1. Run local gates:
   - `make ann-fixture-check`;
   - `make ann-drift-check`;
   - `make ann-external-check`;
   - `make ann-metric-matrix-check`;
   - `make ann-real-embedding-readiness`.
2. Keep real embedding keys only in local environment files.
3. Do not re-add hosted provider secrets or scheduled embedding runs to GitHub
   Actions before beta.
4. Archive repeated real-domain runs locally.
5. Add long-running SLO history for recall, latency, fallback rate, and graph
   freshness.

Done when:

- Local reports show `production_safe=true`.
- Exact fallback stays enabled for critical workloads.
- Real-domain baseline history is repeatable across stable environments.

## Epic 5 - Security And Operations Baseline

Status: partial.

Goal: make the local/Core Alpha deployment safer without claiming production
security.

Tasks:

1. Run:
   - `make tenant-recovery-check`;
   - `make backup-drill-check`;
   - `make crash-fault-check`;
   - `make chaos-restart-check`.
2. Review route auth coverage.
3. Review tenant path validation.
4. Review audit log behavior.
5. Review rate-limit behavior.
6. Add audit review tooling:
   - CLI audit viewer;
   - filters by route/status/action/tenant;
   - summary counts;
   - redaction check.
7. Harden RBAC:
   - keep static `admin`/`data` roles;
   - add dynamic policy-store design;
   - add AgentView-backed role/scope management;
   - document non-goals until implemented.

Done when:

- Security/ops gates pass.
- Audit logs can be reviewed without hand-parsing JSONL.
- RBAC limitations and next steps are explicit.

## Epic 6 - Product UI And Release Surface Hardening

Status: partial.

Goal: harden the user-facing release surface before deeper production claims.

Tasks:

1. Run:
   - `make dashboard-check`;
   - `make dashboard-release-check`;
   - `make dashboard-smoke`;
   - `make dashboard-screenshots`.
2. Keep dashboard documented as a developer console until product UX is ready.
3. Improve only product-critical flows:
   - auth/session UX;
   - tenant selection;
   - error surfaces;
   - validation/repair views;
   - Context Pack explain;
   - ANN evaluation report.
4. Add binary release artifacts:
   - Linux/macOS tarballs for `cortexdb` and `cortex-server`;
   - SHA-256 checksums;
   - install docs;
   - release workflow validation.
5. Add migration compatibility proof:
   - storage/API/SDK cross-version fixtures;
   - old-format sample files;
   - upgrade/downgrade matrix;
   - migration notes for breaking changes.

Done when:

- Dashboard release gates pass.
- Binary artifacts are generated and checksummed.
- Migration compatibility tests exist for storage/API/SDK boundaries.

## Epic 7 - Consensus Hardening

Status: partial, not production-ready.

Goal: continue from Raft-like primitives toward operational consensus.

Tasks:

1. Keep existing replication gates green.
2. Add long-running split-brain/rejoin tests.
3. Add follower lag repair soak tests.
4. Add snapshot handoff under restart.
5. Add membership rotation resume scenarios.
6. Add operator topology reload lifecycle.
7. Define failover/replay/repair SLOs before beta claims.
8. Keep docs clear that consensus is experimental until these SLOs are proven.

Done when:

- Failure-mode tests are repeatable.
- Failover and repair SLOs are documented.
- Operational lifecycle is documented and test-covered.

## Current Next Epic

Epic 2 is next if Epic 1 remains accepted as locally done. If the next work is
implementation-heavy, start with API/SDK live smoke and version lock-step checks
before moving to Context Pack, ANN, security, UI, or consensus work.

## Minimum Verification

For doc-only updates to this file:

```bash
make beta-delta-check
make public-claims-check
cargo fmt --check
cargo check --workspace
```

For implementation updates:

```bash
cargo check --workspace
cargo test --workspace --all-features
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
make openapi-contract-check
```
