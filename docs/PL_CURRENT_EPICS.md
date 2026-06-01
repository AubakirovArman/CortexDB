# Current Epics Extracted From `pl.md`

Source: `/mnt/hf_model_weights/arman/3bit/sites/pl.md`
Reviewed: 2026-06-01

This file is the current epic extraction from the external `pl.md` audit plan.
It intentionally separates current local evidence from future/general
production claims.

## Status Legend

- `done-local`: implemented and covered by current local evidence or release
  gates.
- `partial`: a local gate exists, but the current epic still has unclosed local
  scope.
- `future`: explicitly outside the current local single-node boundary.

## Epic Summary

Total current epics extracted from `pl.md`: 18.

Current local completion:

- `done-local`: 18 / 18
- `partial`: 0 / 18
- `future`: 0 / 18 current epics, with explicit future/non-goals listed below

| # | Epic | Status | Main evidence / gate |
|---|---|---|---|
| 1 | Evidence-backed Alpha Finalization | done-local | `make release-check`, `target/production-evidence/report.json` |
| 2 | SDK E2E Evidence Matrix | done-local | `target/sdk-e2e-release/report.json`, `make sdk-contract-check` |
| 3 | ContextPack Quality Gate | done-local | `target/context-pack-quality/report.json` |
| 4 | Verification Quality Gate | done-local | `target/verification-quality/report.json` |
| 5 | Real-domain Embedding History / Retrieval Quality | done-local | `target/retrieval-quality/report.json`, `target/ann/real-embedding/runs/*/report.json` |
| 6 | Release Evidence Bundle | done-local | `target/production-evidence/report.json`, release-check output |
| 7 | Error Taxonomy And API Snapshot Evidence | done-local | `make openapi-contract-check`, server/API snapshot tests |
| 8 | Storage Compatibility And Soak History | done-local | `target/storage-compat/report.json`, `target/storage-soak/report.json` |
| 9 | Core Engine API Stability | done-local | `target/engine-api/report.json` |
| 10 | AQL Compatibility Pack | done-local | `target/aql-compat/report.json` |
| 11 | HTTP Server Contract And Operations | done-local | `target/http-contract-ops/report.json` |
| 12 | CLI Productization | done-local | `target/cli-product/report.json` |
| 13 | Security Hardening | done-local | `target/security-hardening/report.json`; enterprise RBAC/encrypted backups/tamper-evident audit remain future/non-goals |
| 14 | Observability And Operations Runbooks | done-local | `target/observability/report.json`, `docs/METRICS.md` |
| 15 | Deployment And Upgrade | done-local | `target/deployment-upgrade/report.json`, binary release checks |
| 16 | Beta Release Candidate | done-local | `target/beta-rc/report.json` |
| 17 | Production Candidate | done-local | `target/production-candidate/report.json` |
| 18 | Production v1.0 Local Single-node | done-local | `target/production-v1/report.json` |

## Epics And Task Pools

### Epic 1 - Evidence-backed Alpha Finalization

Goal: publish an Alpha release backed by machine-readable evidence.

Tasks:

1. Produce a release evidence bundle.
2. Run and preserve `make release-check` output.
3. Freeze API snapshots.
4. Publish the RAG-vs-CortexDB demo output.
5. Tag only after the release gates pass.

Acceptance:

- Release artifacts include docs, JSON reports, demo output, benchmark output,
  and clear limitations.

Current status:

- `done-local`.
- Verified by a full local `make release-check` run ending with
  `Release check passed`.

### Epic 2 - SDK E2E Evidence Matrix

Goal: prove Rust, Python, and TypeScript SDKs against a live local server.

Tasks:

1. Cover health and auth.
2. Cover put/get.
3. Cover search.
4. Cover ContextPack.
5. Cover Verify Fact.
6. Cover tenant routing and structured errors.

Acceptance:

- SDK users can build against the API without reading server source.

Current status:

- `done-local`.
- Evidence: `target/sdk-e2e-release/report.json`, `make sdk-check`,
  `make sdk-contract-check`, and `make sdk-smoke-test`.

### Epic 3 - ContextPack Quality Gate

Goal: prove ContextPack behavior with repeatable quality evidence.

Tasks:

1. Measure evidence coverage.
2. Measure token efficiency.
3. Measure citation coverage.
4. Measure duplicate suppression.
5. Compare RAG chunks vs ContextPack output on demo/domain data.

Acceptance:

- ContextPack improves evidence coverage or reduces irrelevant tokens on the
  demo corpus.

Current status:

- `done-local`.
- Evidence: `target/context-pack-quality/report.json`.

### Epic 4 - Verification Quality Gate

Goal: measure deterministic `VERIFY FACT` behavior.

Tasks:

1. Maintain labeled cases for supported, contradicted, mixed, and insufficient
   evidence.
2. Produce a confusion matrix.
3. Cover numeric conflicts.
4. Cover missing citations and contradiction markers.
5. Cover equal values, ambiguity, and no-evidence cases.

Acceptance:

- Verification report includes release-corpus accuracy metrics.

Current status:

- `done-local`.
- Evidence: `target/verification-quality/report.json`.

### Epic 5 - Real-domain Embedding History / Retrieval Quality

Goal: promote retrieval from feature presence to measured quality.

Tasks:

1. Keep repeated real-domain embedding runs.
2. Track recall@k, MRR, nDCG, exact fallback parity, p95/p99 latency, and drift.
3. Preserve corpus, queries, ground truth, model, and metrics.
4. Keep HNSW exact fallback available for critical workloads.

Acceptance:

- Release includes reproducible retrieval quality reports and no recall
  regression against the tracked baseline.

Current status:

- `done-local`.
- Evidence: `target/retrieval-quality/report.json` and real-embedding reports
  under `target/ann/real-embedding/runs/`.

### Epic 6 - Release Evidence Bundle

Goal: collect evidence reports into a release-ready artifact set.

Tasks:

1. Include production evidence.
2. Include SDK reports.
3. Include OpenAPI/API reports.
4. Include benchmarks and demo output.
5. Include backup, crash, tenant, security, and deployment evidence.

Acceptance:

- GitHub release can attach a coherent evidence bundle instead of scattered
  manual notes.

Current status:

- `done-local`.
- Evidence: `target/production-evidence/report.json` and release-check output.

### Epic 7 - Error Taxonomy And API Snapshot Evidence

Goal: keep API contracts and user-visible errors stable.

Tasks:

1. Enforce OpenAPI contract.
2. Enforce typed JSON schemas.
3. Cover invalid AQL.
4. Cover permission denied.
5. Cover tenant, corruption, rate-limit, and not-found errors.

Acceptance:

- SDK and HTTP callers can rely on stable error codes and response shapes.

Current status:

- `done-local`.
- Evidence: `make openapi-contract-check`, API snapshot tests, and SDK contract
  checks.

### Epic 8 - Storage Compatibility And Soak History

Goal: turn storage into a repeatable reliability surface.

Tasks:

1. Test storage compatibility.
2. Test previous-version restore fixtures.
3. Test corruption behavior for storage files.
4. Keep soak history.
5. Keep backup/restore and repair evidence.

Acceptance:

- Backup and crash/fault reports are release artifacts.
- Strict and best-effort recovery behavior is documented.

Current status:

- `done-local`.
- Evidence: `target/storage-compat/report.json`,
  `target/storage-soak/report.json`, backup reports, crash/fault reports, and
  migration compatibility reports.

### Epic 9 - Core Engine API Stability

Goal: separate stable embedded Rust API from internal engine APIs.

Tasks:

1. Document stable API boundaries.
2. Document module ownership.
3. Keep compile checks for examples and public API usage.

Acceptance:

- External Rust users know which APIs are stable and which are internal.

Current status:

- `done-local`.
- Evidence: `target/engine-api/report.json`.

### Epic 10 - AQL Compatibility Pack

Goal: freeze AQL v0.4 behavior for clients.

Tasks:

1. Keep AQL docs aligned with parser and binder behavior.
2. Maintain golden tests.
3. Cover `LIMIT`, `REQUIRE`, malformed AQL, forbidden scope, unknown fields,
   and explain snapshots.
4. Stabilize parse and bind error codes.

Acceptance:

- SDK callers can distinguish invalid syntax, permission denied, and
  unsupported query behavior.

Current status:

- `done-local`.
- Evidence: `target/aql-compat/report.json`.

### Epic 11 - HTTP Server Contract And Operations

Goal: make HTTP behavior reproducible and operator-friendly.

Tasks:

1. Cover route-level status and error codes.
2. Keep request ID propagation.
3. Keep structured logs and redaction.
4. Cover admin/data token behavior, scope restrictions, rate limit, CORS, and
   audit redaction.

Acceptance:

- HTTP security and operations behavior is reproducible from a single gate.

Current status:

- `done-local`.
- Evidence: `target/http-contract-ops/report.json`.

### Epic 12 - CLI Productization

Goal: make local diagnosis and operations easier.

Tasks:

1. Keep `cortexdb doctor`.
2. Keep completions.
3. Keep CLI golden snapshots.
4. Document CLI-to-HTTP schema parity.

Acceptance:

- New users can diagnose install, data, and server problems from CLI.

Current status:

- `done-local`.
- Evidence: `target/cli-product/report.json`.

### Epic 13 - Security Hardening

Goal: move from Core Alpha token controls toward beta/production security.

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

Current status:

- `done-local`.
- Evidence: `target/security-hardening/report.json` proves the current local
  security gate.
- This epic is closed for the local single-node boundary by proving the current
  controls and documenting release-blocking boundaries. Broader
  enterprise-grade items remain explicit future/non-goals: dynamic RBAC,
  production encrypted backups, tamper-evident audit, remote backup, and
  external identity providers.

### Epic 14 - Observability And Operations Runbooks

Goal: make runtime behavior visible to operators.

Tasks:

1. Document health, request, actor, WAL, search, and storage metrics.
2. Add Prometheus scrape examples.
3. Add Grafana dashboard examples.
4. Add alert examples.
5. Keep operations runbooks aligned with current endpoints.

Acceptance:

- Operators can inspect runtime signals without source-code reading.

Current status:

- `done-local`.
- Evidence: `target/observability/report.json`.

### Epic 15 - Deployment And Upgrade

Goal: make single-node install and upgrade repeatable.

Tasks:

1. Keep install docs.
2. Keep systemd docs.
3. Keep Linux/macOS binary install guide.
4. Attach binary release assets to GitHub releases.
5. Keep upgrade and rollback guide.

Acceptance:

- A user can install, run, upgrade, rollback, and validate a single-node
  CortexDB instance from docs.

Current status:

- `done-local`.
- Evidence: `target/deployment-upgrade/report.json` and binary release checks.

### Epic 16 - Beta Release Candidate

Goal: stabilize external beta readiness.

Tasks:

1. Backup/restore beta evidence.
2. Operational docs.
3. Security model v1.
4. Ingestion jobs maturity.
5. Dashboard operational view.

Acceptance:

- Beta can be evaluated by external developers without ad hoc setup.

Current status:

- `done-local`.
- Evidence: `target/beta-rc/report.json`.

### Epic 17 - Production Candidate

Goal: prepare production-like local single-node deployment.

Tasks:

1. Load tests.
2. Crash/fault history.
3. Migration compatibility.
4. Audit log hardening.
5. Quotas and rate limits.
6. Encrypted backups design.
7. RPO/RTO and SLO docs.

Acceptance:

- Production-candidate claims stay inside the local single-node boundary.

Current status:

- `done-local`.
- Evidence: `target/production-candidate/report.json`.

### Epic 18 - Production v1.0 Local Single-node

Goal: stable local single-node Linux/macOS release.

Tasks:

1. Keep single-node production claim only.
2. Keep stable API and SDK contracts.
3. Keep supported backup/restore.
4. Keep complete operational docs.
5. Keep distributed production out of scope until separately proven.

Acceptance:

- Local single-node production boundary is documented and evidence-backed.

Current status:

- `done-local`.
- Evidence: `target/production-v1/report.json`.

## Explicit Non-goals From `pl.md`

These are not current epics and should not be treated as closed by the current
local single-node evidence:

1. Production distributed consensus.
2. Managed cloud.
3. Enterprise RBAC and compliance.
4. Full production HNSW without fallback.
5. Built-in LLM inference.
6. External identity providers.
7. Legal-grade verification.

## Immediate Operating Rule

The current plan is locally evidence-backed, but release claims should remain
bounded:

1. Before any public release: rerun `make release-check`.
2. Before stronger production claims: rerun `make production-v1-check` in a
   clean environment.
3. Before beta claims: preserve SDK, ContextPack, Verify, retrieval, storage,
   security, observability, and deployment reports as release artifacts.
