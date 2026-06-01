# CortexDB Next Development Plan From `pl.md`

Source: `/mnt/hf_model_weights/arman/3bit/sites/pl.md`
Reviewed: 2026-06-01.

This document extracts the useful work from the new external audit and turns it
into an ordered execution plan. It intentionally does not reopen already closed
Core Alpha evidence epics unless the new work requires stronger, repeated, or
clean-environment proof.

## Current Stage

CortexDB is beyond prototype and Core Alpha. The repo has a durable single-node
core, AQL, ContextPack, VERIFY FACT, search/ANN guardrails, CLI, HTTP API, SDK
surfaces, dashboard, release gates, and deployment docs.

The next work is not "add random features". The next work is:

```text
Core Alpha evidence -> Beta Foundation -> Beta RC -> Production Candidate
-> narrow local single-node Production v1.0
```

General production, managed cloud, and production distributed consensus are
explicitly future work.

## Execution Rules

1. Close epics in order.
2. Each epic has tasks, gates, and acceptance evidence.
3. A gate is evidence only if the command is actually run and the report exists.
4. Do not promote a capability from local evidence to beta/production without
   repeated runs or release artifacts where required.
5. Keep exact vector fallback for critical workloads while HNSW remains guarded.
6. Do not claim legal-grade verification, enterprise compliance, managed cloud,
   or production distributed consensus.

## Already Closed Locally

The following areas have local Core Alpha evidence and should not be treated as
fresh blockers unless a clean/repeated/release-level requirement is added:

- AQL v0.4 compatibility and golden behavior.
- ContextPack v1 contract and local quality gate.
- Verification quality fixture and local confusion-style report.
- HTTP contract, typed JSON, request IDs, auth/rate-limit/audit checks.
- CLI `doctor`, completions, golden output checks.
- SDK contract and local e2e release gate.
- Dashboard build/smoke/screenshots/product UI evidence gate.
- Security hardening evidence gate for Core Alpha scope.
- Observability docs, Prometheus, alerts, and Grafana evidence gate.
- Deployment/install/systemd/upgrade/rollback evidence gate.
- Binary release packaging and tag-gated GitHub release workflow wiring.

## Milestone 1 - Evidence-Backed Alpha Finalization

Goal: make the current Core Alpha release reproducible from a clean checkout.

### Epic 1.1 - Clean Release Evidence Reproduction

Tasks:

1. Run `make release-check` on a clean machine or clean container.
2. Record commit SHA, date, host profile, and reports in release evidence.
3. Confirm all referenced `target/.../report.json` artifacts exist.
4. Confirm `docs/RELEASE_EVIDENCE.md` matches the actual output.
5. Confirm README limitations match the release evidence.

Gates:

```bash
make release-check
make production-evidence-sweep
```

Acceptance:

- Release evidence proves the current repo state, not stale claims.
- `docs/RELEASE_EVIDENCE.md` lists generated artifact paths and status.

Risks:

- `release-check` is intentionally heavy and may not be suitable for every PR.

### Epic 1.2 - Public Release Tag And Artifact Audit

Tasks:

1. Confirm whether `v0.1.0-core-alpha` exists locally and on GitHub.
2. If publishing, create the tag only after clean release evidence passes.
3. Verify GitHub Release contains binary tarballs, SHA256 files, dashboard
   package, and ANN baseline package where applicable.
4. Verify release notes do not overclaim production/distributed readiness.

Gates:

```bash
git tag --list 'v*'
make binary-release-check
```

Acceptance:

- Public release assets are present and checksummed.
- Release notes link the right evidence and limitations.

## Milestone 2 - Beta Foundation

Goal: external developers can build real experiments with stable contracts and
measured quality.

### Epic 2.1 - SDK E2E Matrix Strengthening

Tasks:

1. Run live SDK smoke against a local `cortex-server`.
2. Cover Rust, Python, and TypeScript:
   - health;
   - put/get;
   - search;
   - AQL;
   - ContextPack;
   - VERIFY FACT;
   - tenant routing;
   - bearer auth;
   - structured errors.
3. Add missing SDK examples per language.
4. Keep public registry publishing blocked until credentials and ownership are
   explicit.

Gates:

```bash
make sdk-smoke-test
make sdk-contract-check
make sdk-e2e-release-check
```

Acceptance:

- All SDKs pass the same main flows against a live local server.
- SDK docs tell users how to run the examples.

### Epic 2.2 - ContextPack Quality v2

Tasks:

1. Compare ContextPack output against classic RAG chunks.
2. Expand quality metrics:
   - evidence coverage;
   - citation coverage;
   - token efficiency;
   - duplicate suppression;
   - anomaly coverage;
   - deterministic order.
3. Add at least one more fixture beyond `investment_projects`.
4. Keep metric definitions stable in docs.

Gates:

```bash
make context-pack-quality-check
make context-verify-quality-check
```

Acceptance:

- Quality report shows why ContextPack is better than raw chunk retrieval for
  the demo domain.

### Epic 2.3 - Verification Evaluation v2

Tasks:

1. Expand labeled VERIFY FACT dataset.
2. Track supported, contradicted, mixed, and insufficient cases.
3. Include numeric equality, numeric conflict, currency mismatch, missing
   citation, contradiction markers, ambiguity, and no evidence.
4. Report false positives and false negatives explicitly.

Gates:

```bash
make verification-quality-check
```

Acceptance:

- Verification quality report includes accuracy/confusion metrics and known
  limitations.

### Epic 2.4 - Error Taxonomy And API Snapshot Enforcement

Tasks:

1. Ensure every stable error code appears in:
   - `docs/API_ERROR_TAXONOMY.md`;
   - `docs/openapi.yaml`;
   - server response snapshots;
   - SDK decoders.
2. Add tests for invalid AQL, permission denied, busy, corruption, rate limit,
   tenant errors, and unknown route errors.
3. Require changelog/docs updates when adding or changing error codes.

Gates:

```bash
make openapi-contract-check
make http-contract-ops-check
make sdk-contract-check
```

Acceptance:

- New API or error response changes cannot silently bypass docs/OpenAPI/SDK
  snapshots.

### Epic 2.5 - Real-Domain Embedding Repeated History

Tasks:

1. Run at least 3 repeated `investment_projects` real-embedding benchmarks in a
   stable local environment.
2. Track recall@k, MRR, nDCG, exact fallback parity, p95/p99 latency, fallback
   rate, and drift.
3. Archive reports locally.
4. Keep API keys in `.env` only; do not restore scheduled hosted embedding
   runs in GitHub Actions before beta.

Gates:

```bash
make ann-real-embedding-readiness
make ann-real-embedding-benchmark-and-compare
make ann-real-embedding-history-regression-check
```

Acceptance:

- At least 3 runs exist with no regression beyond documented thresholds.
- The benchmark names model, corpus, query set, and ground truth.

### Epic 2.6 - Search Quality Report

Tasks:

1. Publish a retrieval quality report for real-domain corpus.
2. Track lexical, vector, hybrid, and guarded ANN behavior separately.
3. Document exact fallback parity and when ANN is allowed.
4. Add query-level quality output for review.

Gates:

```bash
make retrieval-quality-check
```

Acceptance:

- Release includes corpus, queries, ground truth, model, and metrics.

## Milestone 3 - Beta Release Candidate

Goal: stable external API/SDK plus operational docs for early external users.

### Epic 3.1 - Operations Runbook And Reader Path

Tasks:

1. Add a "first 10 minutes" reader path.
2. Consolidate operational runbooks:
   - install;
   - validate;
   - backup;
   - restore;
   - repair;
   - metrics;
   - upgrade;
   - rollback.
3. Add troubleshooting for stale locks, corrupt WAL, corrupt segment, busy
   actor queue, failed auth, and tenant errors.

Gates:

```bash
make deployment-upgrade-check
make observability-check
make public-claims-check
```

Acceptance:

- A new user can install, run, validate, diagnose, backup, and rollback without
  reading source code.

### Epic 3.2 - Security Beta Baseline

Tasks:

1. Convert RBAC policy-store design into an implementation plan or first
   implementation.
2. Add per-token quota design or implementation.
3. Add tamper-evident audit-chain design or implementation.
4. Add encrypted backup design or implementation.
5. Add dashboard auth hardening plan.
6. Expand malicious ingestion and log redaction tests.

Gates:

```bash
make security-hardening-check
```

Acceptance:

- Docs clearly separate implemented controls from beta/enterprise non-goals.
- Release is blocked if auth, tenant, error, audit, or redaction tests fail.

### Epic 3.3 - Backup/Restore Beta Support

Tasks:

1. Keep backup/restore drills as release artifacts.
2. Add backup archive corruption tests.
3. Add restore drill trend across releases.
4. Decide whether encrypted backup is required for beta or remains production
   candidate.

Gates:

```bash
make backup-drill-check
make backup-offsite-check
make storage-compat-check
```

Acceptance:

- Backup/restore behavior has repeatable artifacts and documented RPO/RTO
  boundaries.

### Epic 3.4 - Ingestion Jobs Maturity

Tasks:

1. Harden ingestion jobs lifecycle:
   - progress;
   - retry;
   - cancel;
   - failure reason;
   - empty input behavior.
2. Clarify alpha limitations for PDF/OCR/document ingestion.
3. Add server and CLI tests for ingestion job flows.

Gates:

```bash
cargo test --workspace --all-features
```

Acceptance:

- Ingestion failures are visible and recoverable without corrupting the DB.

### Epic 3.5 - Dashboard Operational Views

Tasks:

1. Improve incident/status views.
2. Improve role-aware and permissions views.
3. Add metrics/audit panels.
4. Keep screenshots as release artifacts.

Gates:

```bash
make dashboard-product-check
make dashboard-screenshots
```

Acceptance:

- Dashboard is useful for operations, not only demo/debug.

## Milestone 4 - Production Candidate

Goal: controlled local single-node production candidate for Linux/macOS.

### Epic 4.1 - Long-Running Storage And Crash Soak

Tasks:

1. Add `make storage-soak-check`.
2. Run repeated write/checkpoint/compact/backup/restore loops.
3. Include kill during checkpoint, compact, restore, and WAL replay.
4. Track corruption, repair, and recovery outcomes over time.
5. Add versioned restore fixtures from previous release tags.

Gates:

```bash
make storage-compat-check
make crash-fault-check
make chaos-restart-check
make storage-soak-check
```

Acceptance:

- Soak reports prove durability across repeated cycles, not only one local
  release run.

### Epic 4.2 - Migration Compatibility Across Releases

Tasks:

1. Create historical fixtures for at least one previous release.
2. Validate old backups with current binary.
3. Validate old storage markers remain read-only compatible where promised.
4. Require migration notes for storage/API/SDK changes.

Gates:

```bash
make migration-policy-check
make migration-compatibility-check
make storage-compat-check
```

Acceptance:

- Breaking changes cannot land without migration docs, tests, and release
  notes.

### Epic 4.3 - Load And Performance Trend History

Tasks:

1. Keep single-node performance reports per release.
2. Add p95/p99 thresholds for write/read/search/context/verify flows.
3. Track actor queue saturation and `database_busy`.
4. Define workload classes and RPO/RTO expectations.

Gates:

```bash
make load-smoke-check
make single-node-performance-check
```

Acceptance:

- Performance regressions are visible before release.

### Epic 4.4 - Observability Runbooks And SLO Dashboards

Tasks:

1. Convert metric docs into operator playbooks.
2. Add alert thresholds for WAL growth, checkpoint lag, actor queue pressure,
   ANN fallback rate, and validation failures.
3. Keep Grafana/Prometheus examples release-ready.

Gates:

```bash
make observability-check
```

Acceptance:

- Operators know what to do when an alert fires.

### Epic 4.5 - Security Production-Candidate Controls

Tasks:

1. Decide implementation boundary for:
   - dynamic RBAC;
   - per-token quotas;
   - tamper-evident audit;
   - encrypted backup;
   - remote object-store backup.
2. Implement or explicitly defer each with a release-blocking decision.
3. Update threat model and release checklist.

Gates:

```bash
make security-hardening-check
make production-candidate-check
```

Acceptance:

- Production candidate does not rely on undocumented security assumptions.

## Milestone 5 - Production v1.0 Local Single-Node

Goal: stable local single-node release only.

### Epic 5.1 - Stable API/SDK Release Train

Tasks:

1. Publish or dry-run every SDK package with version lock-step.
2. Enforce changelog and deprecation policy.
3. Keep OpenAPI as source of truth.
4. Add SDK examples to release artifacts.

Gates:

```bash
make sdk-check
make sdk-contract-check
make sdk-e2e-release-check
```

Acceptance:

- API and SDK compatibility can survive repeated releases.

### Epic 5.2 - Binary Platform Matrix

Tasks:

1. Keep Linux and macOS binary release artifacts.
2. Document Windows as unsupported until implemented.
3. Add macOS launchd example only if macOS is a serious target.
4. Validate clean install -> server -> fixture -> query -> backup/restore.

Gates:

```bash
make binary-release-check
make deployment-upgrade-check
```

Acceptance:

- Supported platform matrix is clear and tested.

### Epic 5.3 - Public Claims Freeze

Tasks:

1. Audit README and docs for overclaims.
2. Keep "local single-node only" production boundary explicit.
3. Keep distributed, managed cloud, enterprise compliance, legal-grade
   verification, and production HNSW claims out of public wording.

Gates:

```bash
make public-claims-check
```

Acceptance:

- A user cannot reasonably misread local single-node evidence as general
  distributed/cloud production readiness.

## Future Product Layers

These are not blockers for local single-node production. They require separate
plans and evidence.

### Future Epic F1 - Real Distributed Consensus

Tasks:

1. Define actual replicated log semantics.
2. Define leader election and failover SLO.
3. Add long-running split-brain/rejoin tests.
4. Add operational lifecycle: add/remove node, snapshot transfer, lag repair,
   rolling restart.
5. Add production transport/security model.

Acceptance:

- Distributed mode proves no split-brain writes under partition/rejoin and has
  documented operator procedures.

### Future Epic F2 - Managed Cloud

Tasks:

1. Define cloud deployment model.
2. Define tenant isolation beyond local path realms.
3. Define auth/identity integration.
4. Define monitoring, backup, billing, upgrades, and support lifecycle.

Acceptance:

- Managed cloud has its own threat model and operational evidence.

### Future Epic F3 - Product UI v2

Tasks:

1. Move beyond static developer console.
2. Add role-aware workflows.
3. Add incident response workflows.
4. Add admin UX for policies, backups, validation, repair, metrics, and
   ingestion jobs.

Acceptance:

- UI supports real operator workflows, not only demos.

## Immediate Ordered Queue

1. Clean release evidence reproduction.
2. Public release tag/artifact audit.
3. SDK E2E matrix strengthening.
4. ContextPack Quality v2.
5. Verification Evaluation v2.
6. Real-domain embedding repeated history.
7. Operations runbook and reader path.
8. Security beta baseline decisions.
9. Storage/crash soak.
10. Migration compatibility across releases.

## Do Not Do Now

- Do not market production distributed consensus.
- Do not remove exact vector fallback for critical workloads.
- Do not build managed cloud before SDK/API/security/ops evidence is stable.
- Do not embed LLM calls into the core engine.
- Do not change storage formats without migration compatibility tests.
- Do not claim legal-grade verification.
