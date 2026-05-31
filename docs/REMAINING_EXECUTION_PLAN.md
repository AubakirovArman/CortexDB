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
- **Открыто:**
  - финальный baseline на доменном реальном embedding
  - долгоживущие observability и SLA-регрессионные пороги для production traffic

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
4. Tighten recovery evidence for production restore drills and tenant isolation incidents.

## Immediate next actions (1–2 недели)

- Run one production evidence sweep:
  - `make openapi-contract-check`
  - `make backup-drill-check`
  - `make ann-release-evidence-check`
  - `make replication-partition-check`
- Publish a short beta delta note:
  - what is stable
  - what is still experimental
  - what is blocked
- Lock the external-facing statement in README/API docs to avoid overclaiming.

## Gate definition

Current gate for moving to next cycle:
- `cargo check --workspace`
- `cargo test --workspace --all-features`
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `make openapi-contract-check`
- no contradictory claims in architecture/API docs
