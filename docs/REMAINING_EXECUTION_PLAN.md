# CortexDB Execution Plan — Current Cycle

## Current Stage (по факту репозитория)

Core Alpha is operational:
- durable WAL + MVCC + restart/replay
- checkpoint/compact + validation/recovery
- AQL retrieval + ContextPack + verify
- ANN/HNSW evaluation pipeline and recall/latency guardrails
- backup/restore + actor-based server + CLI + SDK contract checks

## Big Next Milestones (выполняем в очереди)

### 1) Production-grade ANN / HNSW hardening
- **Status:** частично закрыто, в production-safe режиме.
- **Сделано:**
  - ANN recall/latency gates, history tracking, drift checks, external corpus checks, exact fallback and graph trailer persistence
  - baseline publishing + history regression gates
  - machine-readable real-embedding readiness report for missing corpus/env/archive prerequisites
  - local `investment_projects` real-domain corpus with documents, chunks,
    40 queries, ground truth, validators, and readiness wiring
  - endpoint-backed `investment-projects-v1` benchmark using `BAAI/bge-m3`
    embeddings, with packaged local baseline and `production_safe=true`
- **Открыто:**
  - долгоживущие observability и SLA-регрессионные пороги для production traffic
  - beta-stage GitHub Actions promotion for real embedding benchmark history

### 2) Real distributed consensus readiness
- **Status:** активно развивается (не production-ready).
- **Сделано:** raft-like primitives, replication log, snapshots, partition/failure матрицы, repair worker/runtime
- **Открыто:**
  - операционный lifecycle узла и стабильный production rollout
  - долгий failure-mode hardening (повторные split-brain/rejoin сценарии)
  - строгие SLO для failover/replay

### 3) Full web UI
- **Status:** developer-консоль готова, продуктовый UX частичный.
- **Сделано:** многовью статический dashboard, build/smoke/screenshots, standalone dist
- **Открыто:**
  - production UX для прав доступа, ошибок, инцидентного состояния
  - расширенный визуальный и e2e регрессионный охват

### 4) Stable SDK publication lifecycle
- **Status:** контракт и публикационные процессы в основном закрыты.
- **Сделано:** версия/контракт checks, docs/чеклисты, Rust/Python/TS metadata и pipelines
- **Открыто:**
  - регулярный release train с версионированием на всех каналах API/SDK
  - интеграционные smoke на каждом релизе + changelog policy enforcement

## Технический долг (выполнить в первую очередь)

1. Finalize production-style error taxonomy and security model docs tied to API/CLI behavior.
2. Close remaining self-inconsistent docs (`pl.md` и roadmap mirrors) на уровне статуса: done/partial/blocked.
3. Finish load/performance matrix for single-node (before consensus expansion).
   - current gate: `make single-node-performance-check`
   - report: `target/single-node-performance/report.json`
4. Tighten recovery evidence for production restore drills and tenant isolation incidents.
   - current gate: `make tenant-recovery-check`
   - report: `target/tenant-recovery/report.json`

## Immediate next actions (1–2 недели)

- Run one production evidence sweep:
  - `make production-evidence-sweep`
  - writes `target/production-evidence/report.json`
  - includes OpenAPI contract, backup drill, single-node performance,
    tenant recovery, ANN release evidence, real-embedding readiness, and
    replication partition evidence logs
- Latest local evidence: passed on `2026-05-31` at
  `36f70e8cf1d88293254d1f7c9133793dc057f313`.
  Covered steps: OpenAPI contract, backup drill, single-node performance,
  tenant recovery, ANN release evidence, real-embedding readiness, and
  replication partition evidence.
- Publish a short beta delta note:
  - what is stable
  - what is still experimental
  - what is blocked
  - current note: `docs/BETA_DELTA.md`
  - consistency gate: `make beta-delta-check`
- Run the focused Beta Foundation evidence gate:
  - `make beta-foundation-check`
  - writes `target/beta-foundation/report.json`
  - covers SDK e2e, OpenAPI contract, ContextPack/VERIFY quality, search
    quality, error taxonomy, metrics contract, and beta boundary docs
- Run the focused Beta Release Candidate evidence gate:
  - `make beta-rc-check`
  - writes `target/beta-rc/report.json`
  - covers backup/restore evidence, offsite staging, security/auth tests,
    ingestion jobs, dashboard release packaging, and operational docs checks
- Run the focused Deployment And Upgrade evidence gate:
  - `make deployment-upgrade-check`
  - writes `target/deployment-upgrade/report.json`
  - covers install docs, systemd service docs, offline upgrade/rollback docs,
    binary packaging docs, and tag-gated GitHub release asset upload wiring
- Run the focused Production Hardening evidence gate:
  - `make production-hardening-check`
  - writes `target/production-hardening/report.json`
  - covers load smoke, crash/fault evidence, migration compatibility, audit
    hardening, rate-limit behavior, CLI audit tooling, and encrypted-backup
    design boundary
- Run the focused Production Candidate evidence gate:
  - `make production-candidate-check`
  - writes `target/production-candidate/report.json`
  - covers production hardening, backup/RPO-RTO drill, single-node SLO evidence,
    OpenAPI and SDK compatibility, migration policy/compatibility, and binary
    release packaging
- Run the focused Production v1.0 evidence gate:
  - `make production-v1-check`
  - writes `target/production-v1/report.json`
  - covers production candidate evidence, full release check, OpenAPI contract,
    SDK lifecycle checks, backup/restore drills, and public-claims guard
- Run the focused Storage Compatibility evidence gate:
  - `make storage-compat-check`
  - writes `target/storage-compat/report.json`
  - covers migration compatibility, backup drills, crash/fault/corruption
    matrix, chaos restart, and repair dry-run/apply behavior
- Run the focused Engine API evidence gate:
  - `make engine-api-check`
  - writes `target/engine-api/report.json`
  - covers public `cortex-engine` compile checks, doctests, rustdoc build, and
    stable-vs-internal docs
- Run the focused AQL Compatibility evidence gate:
  - `make aql-compat-check`
  - writes `target/aql-compat/report.json`
  - covers v0.4 golden parser/binder behavior, explain, `LIMIT`, `REQUIRE`,
    malformed AQL, permission denial, unknown field, and HTTP error classes
- Run the focused Retrieval Quality evidence gate:
  - `make retrieval-quality-check`
  - writes `target/retrieval-quality/report.json`
  - covers real-domain corpus validity, repeated ANN embedding history,
    recall, MRR, nDCG, exact parity, latency, and production-safety status
- Run the focused ContextPack Quality evidence gate:
  - `make context-pack-quality-check`
  - writes `target/context-pack-quality/report.json`
  - covers budget truncation, required citations, source refs, duplicate
    suppression, deterministic ordering, evidence coverage, and token reduction
- Run the focused Verification Quality evidence gate:
  - `make verification-quality-check`
  - writes `target/verification-quality/report.json`
  - covers labelled supported, contradicted, mixed, insufficient, numeric
    conflict, missing citation, ambiguity, and no-evidence cases
- Run the focused HTTP Server Contract evidence gate:
  - `make http-contract-ops-check`
  - writes `target/http-contract-ops/report.json`
  - covers auth roles, typed errors, OpenAPI contract, request IDs, rate limit,
    CORS, and audit redaction
- Run the focused CLI Product evidence gate:
  - `make cli-product-check`
  - writes `target/cli-product/report.json`
  - covers help/version, doctor, completions, common command docs, and CLI
    golden output markers
- Run the focused SDK E2E Release evidence gate:
  - `make sdk-e2e-release-check`
  - writes `target/sdk-e2e-release/report.json`
  - covers Rust/Python/TypeScript live SDK compatibility, release metadata,
    deprecation policy, and quickstart/release docs
- Run the focused Dashboard Product UI evidence gate:
  - `make dashboard-product-check`
  - writes `target/dashboard/product-ui-report.json`
  - covers read-only mode, operational status, permissions view, standalone
    packaging, and screenshot artifact wiring
- Run the focused Security Hardening evidence gate:
  - `make security-hardening-check`
  - writes `target/security-hardening/report.json`
  - covers current auth, rate limit, audit redaction, malicious ingestion
    denial, encrypted/remote backup boundaries, and security release checklist
- Run the focused Observability evidence gate:
  - `make observability-check`
  - writes `target/observability/report.json`
  - covers metrics field docs, Prometheus scrape config, alert examples, and
    Grafana dashboard JSON
- Lock the external-facing statement in README/API docs to avoid overclaiming.
  - current policy: `docs/PUBLIC_CLAIMS_POLICY.md`
  - consistency gate: `make public-claims-check`

## Gate definition

Current gate for moving to next cycle:
- `cargo check --workspace`
- `cargo test --workspace --all-features`
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `make openapi-contract-check`
- no contradictory claims in architecture/API docs
