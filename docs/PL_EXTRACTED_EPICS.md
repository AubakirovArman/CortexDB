# Extracted Epics From `pl.md`

Source: `/mnt/hf_model_weights/arman/3bit/sites/pl.md`.
Reviewed: 2026-05-31.

This document extracts the epics, tasks, gates, and constraints from the
external audit plan. It is a backlog/reference document, not the active
execution queue. The active queue remains
[`EPIC_EXECUTION_ORDER.md`](EPIC_EXECUTION_ORDER.md).

## Status Boundary

- Current Core Alpha execution plan is closed in `EPIC_EXECUTION_ORDER.md`.
- This file captures the next backlog layers described by `pl.md`.
- Do not treat future production/beta items here as already implemented unless a
  current gate and evidence report proves it.

## Phase Epics

### Epic 1 - Evidence-Backed Core Alpha

Goal: release Core Alpha with machine-readable evidence.

Tasks:

1. Publish a release evidence bundle.
2. Confirm `make release-check` output.
3. Freeze API snapshots.
4. Publish a RAG-vs-CortexDB demo.
5. Tag the release only after gates pass.

Acceptance:

- GitHub release includes docs, JSON reports, demo output, and benchmark output.

### Epic 2 - Beta Foundation

Goal: make CortexDB usable by external developers for real experiments.

Status: focused local evidence gate added and passing.

Tasks:

1. SDK e2e matrix.
2. ContextPack quality gate.
3. Verification evaluation gate.
4. Search quality report.
5. Error taxonomy hardening.
6. Metrics documentation.

Release gates:

- `make beta-delta-check`
- `make beta-foundation-check`
- `make production-evidence-sweep`
- SDK e2e
- OpenAPI contract check

Evidence:

- Focused local gate: [`BETA_FOUNDATION_EVIDENCE.md`](BETA_FOUNDATION_EVIDENCE.md).

### Epic 3 - Beta Release Candidate

Goal: stabilize API/SDK and quality evidence.

Status: focused local evidence gate added; run `make beta-rc-check`.

Tasks:

1. Backup/restore beta evidence.
2. Operational docs.
3. Security model v1.
4. Ingestion jobs maturity.
5. Dashboard operational view.

Evidence:

- Focused local gate: [`BETA_RC_EVIDENCE.md`](BETA_RC_EVIDENCE.md).

### Epic 4 - Production Hardening

Goal: harden operational single-node reliability.

Status: focused local evidence gate added; run `make production-hardening-check`.

Tasks:

1. Load tests.
2. Crash/fault history.
3. Migration compatibility.
4. Audit log hardening.
5. Quotas and rate limits.
6. Encrypted backups design.

Evidence:

- Focused local gate: [`PRODUCTION_HARDENING_EVIDENCE.md`](PRODUCTION_HARDENING_EVIDENCE.md).
- Encrypted backup boundary: [`ENCRYPTED_BACKUPS_DESIGN.md`](ENCRYPTED_BACKUPS_DESIGN.md).

### Epic 5 - Production Candidate

Goal: prepare production-like single-node deployment.

Status: focused local evidence gate added; run `make production-candidate-check`.

Tasks:

1. RPO/RTO docs.
2. SDK/API compatibility evidence.
3. SLO docs.
4. Upgrade and rollback flow.

Evidence:

- Focused local gate: [`PRODUCTION_CANDIDATE_EVIDENCE.md`](PRODUCTION_CANDIDATE_EVIDENCE.md).
- RPO/RTO boundary: [`RPO_RTO.md`](RPO_RTO.md).
- Single-node SLO boundary: [`SINGLE_NODE_SLO.md`](SINGLE_NODE_SLO.md).

### Epic 6 - Production v1.0

Goal: stable Linux/macOS single-node production release.

Status: focused local evidence gate added; run `make production-v1-check`.

Tasks:

1. Single-node production claim only.
2. Stable API/SDK.
3. Supported backup/restore.
4. Complete operational docs.
5. Keep distributed production out of scope until separately proven.

Evidence:

- Boundary: [`PRODUCTION_V1.md`](PRODUCTION_V1.md).
- Focused local gate: [`PRODUCTION_V1_EVIDENCE.md`](PRODUCTION_V1_EVIDENCE.md).

## Workstream Epics

### Epic 7 - Storage Durability And Compatibility

Goal: turn the Core Alpha storage layer into a versioned, repeatable reliability
surface.

Tasks:

1. Add `docs/STORAGE_COMPATIBILITY.md`.
2. Clarify backup/restore status and RPO/RTO boundaries.
3. Add `make storage-compat-check`.
4. Publish `target/storage-compat/report.json`.
5. Test current-version backup restored by next-version code.
6. Test corruption of `.acs`, `.acb`, `.aci`, `.acv`, and `.ach`.
7. Test kill during checkpoint and compact.
8. Test repair dry-run vs repair apply.

Acceptance:

- Backup and crash/fault reports are versioned release artifacts.
- Strict and best-effort recovery behavior is documented.

### Epic 8 - Core Engine API Stability

Goal: separate stable embedded Rust API from internal engine APIs.

Tasks:

1. Add `docs/ENGINE_API.md`.
2. Add `docs/MODULE_OWNERSHIP.md`.
3. Add public API compile checks or snapshots for `cortex-engine`.
4. Compile docs examples.
5. Document stable vs internal APIs.

Acceptance:

- External Rust users know which APIs are stable.

### Epic 9 - AQL Query Compatibility

Goal: freeze AQL v0.4 behavior for clients.

Tasks:

1. Confirm AQL v0.4 docs match parser behavior.
2. Add or finish `EXPLAIN RETRIEVE CONTEXT`.
3. Add AQL golden test pack.
4. Add AQL changelog policy.
5. Stabilize structured parse and bind error codes.

Tests:

- malformed AQL;
- forbidden scope;
- unknown field;
- `LIMIT` and `REQUIRE`;
- explain snapshots.

Acceptance:

- SDK callers can distinguish invalid syntax, permission denied, and unsupported
  query behavior.

### Epic 10 - Retrieval Quality And ANN History

Goal: promote retrieval from feature presence to measured quality.

Tasks:

1. Promote `investment_projects` real-domain embedding baseline into repeated
   benchmark history.
2. Add or keep `make ann-real-embedding-history-regression-check`.
3. Publish retrieval quality docs and reports.
4. Track recall@k, MRR, nDCG, exact fallback parity, p95/p99 latency, and drift.

Acceptance:

- Release includes corpus, queries, ground truth, model, and metrics.

### Epic 11 - ContextPack Quality

Goal: prove ContextPack is better than raw RAG chunks for the demo domain.

Tasks:

1. Add `context_pack_quality.jsonl`.
2. Add `make context-pack-quality-check`.
3. Measure evidence coverage, token efficiency, citation coverage, and
   redundancy reduction.
4. Add RAG chunks vs ContextPack demo report.
5. Cover budget truncation, required citations, source refs, duplicate
   suppression, deterministic ordering, and answer prompt fixtures.

Acceptance:

- ContextPack improves evidence coverage or reduces irrelevant tokens on the
  demo corpus.

### Epic 12 - Verification Evaluation

Goal: measure deterministic `VERIFY FACT` behavior.

Tasks:

1. Add `examples/eval/verification_cases.jsonl`.
2. Label cases as supported, contradicted, mixed, or insufficient.
3. Produce a confusion matrix report.
4. Cover numeric conflict, missing citation, contradiction markers, equal
   values, ambiguity, and no-evidence cases.

Acceptance:

- Verification report includes accuracy metrics for the release corpus.

### Epic 13 - HTTP Server Contract And Operations

Goal: make HTTP behavior reproducible and operator-friendly.

Tasks:

1. Verify route-level error-code coverage.
2. Add request ID propagation.
3. Add structured logs with redaction.
4. Add status-code matrix tests.
5. Add or keep `make security-check`.
6. Test admin vs data token, data-token scope restrictions, rate limit, CORS,
   and audit redaction.

Acceptance:

- HTTP security behavior is reproducible from a single gate.

### Epic 14 - CLI Productization

Goal: make local diagnosis and operations easier.

Tasks:

1. Add `cortexdb doctor`.
2. Add `cortexdb completions`.
3. Add CLI golden snapshots.
4. Document CLI-to-HTTP schema parity.

Acceptance:

- New users can diagnose install, data, and server problems from CLI.

### Epic 15 - SDK E2E And Release Train

Goal: prove Rust/Python/TypeScript SDKs against a live local server.

Tasks:

1. Add or keep SDK e2e for put/get.
2. Add search, Context Pack, Verify Fact, tenant routing, and auth examples.
3. Maintain SDK compatibility matrix.
4. Generate examples from OpenAPI where practical.
5. Keep public registry publishing blocked until release ownership and
   credentials are explicit.

Acceptance:

- Python, TypeScript, and Rust SDKs pass the same main flows against a local
  server.

### Epic 16 - Dashboard Product UI

Goal: move from developer console to product UI.

Tasks:

1. Add dashboard read-only mode.
2. Add incident/status panels.
3. Add permissions view.
4. Keep screenshots in release artifacts.

Acceptance:

- Dashboard smoke tests and screenshots are included in release artifacts.

### Epic 17 - Security Hardening

Goal: move from Core Alpha token controls toward beta security posture.

Tasks:

1. Persisted auth policy store.
2. Per-token quotas.
3. Tamper-evident audit chain.
4. Encrypted backup support.
5. Remote object-store backup design.
6. Secret rotation docs.
7. Dashboard auth hardening.
8. Malicious ingestion tests.
9. Log redaction tests.
10. Security release checklist.

Acceptance:

- Security model clearly separates implemented controls from enterprise
  non-goals.

### Epic 18 - Observability

Goal: make runtime behavior visible to operators.

Tasks:

1. Ensure `docs/METRICS.md` covers all fields and examples.
2. Add Prometheus scrape example.
3. Add Grafana dashboard JSON.
4. Add alert examples.

Acceptance:

- Operators can inspect health, request, actor, WAL, search, and storage
  signals without source-code reading.

### Epic 19 - Deployment And Upgrade

Goal: make single-node install/upgrade repeatable.

Tasks:

1. Add `docs/INSTALL.md`.
2. Add `docs/SYSTEMD.md`.
3. Promote Linux/macOS binary install guide.
4. Attach binary release assets to GitHub releases.
5. Add upgrade and rollback guide.

Acceptance:

- A user can install, run, upgrade, rollback, and validate a single-node
  CortexDB instance from docs.

## First Five Actions From `pl.md`

1. Release evidence bundle:
   - collect reports from `target/production-evidence`, benchmarks, demo, SDK,
     and OpenAPI;
   - add `docs/RELEASE_EVIDENCE.md`;
   - acceptance: release artifact contains JSON reports and summary.
2. SDK e2e proof:
   - local server to Python/TypeScript/Rust put/search/context/verify;
   - acceptance: `make sdk-smoke-test` proves live compatibility.
3. ContextPack quality gate:
   - queries, ground truth, evidence coverage, token efficiency, citation
     coverage;
   - acceptance: report JSON in release evidence.
4. Verification evaluation dataset:
   - labeled supported/contradicted/mixed/insufficient cases;
   - acceptance: confusion matrix report.
5. Error taxonomy hardening:
   - stable error codes for invalid AQL, permission denied, busy, corruption,
     rate limit, and tenant errors;
   - acceptance: OpenAPI and snapshot tests.

## GitHub Issue Backlog From `pl.md`

### P0 - Beta Blockers

1. SDK e2e release evidence.
2. ContextPack quality gate.
3. Verify quality gate.
4. Error code catalog.
5. API snapshots published.
6. Real-domain embedding repeated-run history.
7. Security release checklist.
8. Backup/restore RPO/RTO docs.
9. Operations runbook.
10. Release evidence bundle.

### P1

1. `cortexdb doctor`.
2. SDK examples.
3. Dashboard incident view.
4. Metrics docs.
5. Search quality report.
6. CLI golden outputs.
7. Tenant authz docs.
8. Migration compatibility matrix.
9. ContextPack benchmark docs.
10. Verify dataset expansion.

### P2

1. Dynamic RBAC policy store.
2. Per-token quotas.
3. Tamper-evident audit.
4. Encrypted backups.
5. Remote backup adapter.
6. Load testing.
7. Migration automation.
8. Binary installer.
9. Systemd service.
10. SLO dashboards.

### P3

1. Real distributed consensus.
2. Managed cloud.
3. Advanced graph traversal.
4. Built-in LLM ingestion.
5. Product UI v2.

## ADR Backlog

1. Why CortexDB exists.
2. ContextPack design.
3. Storage model.
4. WAL/MVCC assumptions.
5. AQL design.
6. Metadata source of truth.
7. Verification model.
8. HNSW guarded mode.
9. Single-node before distributed.
10. SDK compatibility.
11. API versioning.
12. Security model.
13. Production readiness policy.
14. Real-domain embedding promotion.
15. Release evidence model.

## Testing And Evaluation Gates

Keep these test classes visible when converting backlog items into active work:

- unit, property, crash/recovery, WAL, compaction, MVCC;
- query policy, AQL parser/binder, ContextPack, verification;
- retrieval quality, HNSW recall/latency, HTTP contract, SDK, CLI;
- fixture e2e, performance, security input/auth, compatibility, release gates.

Alpha gate:

```bash
make alpha-check
```

Beta gate:

```bash
make production-evidence-sweep
make sdk-smoke-test
make ann-real-embedding-readiness
make beta-delta-check
make context-pack-quality-check
make verify-quality-check
```

Production candidate gate:

- crash/fault history;
- backup/restore RPO/RTO;
- security release checklist;
- load tests;
- migration compatibility;
- SDK compatibility matrix.

## Do Not Do Now

1. Do not claim CortexDB is production-ready.
2. Do not re-enable real embedding GitHub secrets or scheduled provider spend.
3. Do not publish SDKs publicly without a release owner, credentials, and
   version lock-step evidence.
4. Do not remove exact vector fallback from critical ANN paths.
5. Do not make storage format changes without migration docs and compatibility
   tests.
6. Do not turn ContextPack into an LLM-calling subsystem inside the DB core.
7. Do not prioritize managed cloud or production distributed consensus before
   single-node evidence is stable.
