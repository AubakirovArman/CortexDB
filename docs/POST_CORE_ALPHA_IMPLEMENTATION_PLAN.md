# Post-Core Alpha Implementation Plan — Current Status

Дата фиксации: 2026-05-29

Цель: не просто добавить фичи, а стабильно довести четыре крупных слоя после Core Alpha.

## Что уже закрыто (в production-ready sense)

- Core Alpha pipeline: WAL + MVCC + checkpoint/compact + restart + validation.
- AQL compiler и Runtime retrieval (bitmap + persisted индексы).
- ContextPack v1.
- Search foundation: keyword + vector exact + ANN/HNSW fallback.
- REST API (v1), OpenAPI, typed response contracts.
- CLI + SDK (Rust/Python/TypeScript) scaffolds + контракт-тесты.
- HTTP safety controls: static and file-backed bearer token policies,
  per-token AgentView binding, bounded actor backpressure, rate limit, and audit
  sink.
- Replication + cluster primitives присутствуют на уровне модулей (эпические тесты для лидера/логов/репликации уже покрывают базовую механику).

## Что осталось по крупным эпикам (большие TODO)

### 1) Production-grade ANN/HNSW (высокий приоритет)
- [x] Конфигурируемый recall-policy guard для production: `require_slo`, exact fallback policy, `production_safe`, `slo_violations`.
- [x] Добавлены полиси-поля ANN (fbackoff, fallback + limits), visited-count и budget-guard (`max_visited_candidates`).
- [x] Порог `MIN_ANN_RECALL_Q16` и fallback в exact.
- [x] Golden-фикстуры для recall guard и baseline benchmark hooks.
- [x] Repeatable recall/latency JSON report (`ann_repeatable_report_json`) for benchmark archival.
- [x] Release-mode ANN fixture gate (`make ann-fixture-check`) comparing synthetic recall, graph shape, upper-layer shape, latency ceilings, and production safety against a checked-in baseline.
- [x] ANN fixture report artifact (`make ann-fixture-report`, `target/ann/ann_fixture_report.json`) uploaded by CI for commit-to-commit drift inspection.
- [x] ANN drift check (`make ann-drift-check`) compares current synthetic recall, graph shape, upper-layer shape, and latency against `ann_drift_baseline_v1.json`.
- [x] External ANN JSONL fixture gate (`make ann-external-check`) validates explicit vectors and named queries against exact top-k.
- [x] Deterministic multi-layer graph persisted in `.ach` upper-layer trailer.
- [x] Metric matrix gate (`make ann-metric-matrix-check`) evaluates dot/cosine/L2 against exact top-k on the fixed JSONL fixture.
- [x] External-corpus harness (`ann_corpus_check`) accepts vectors, queries, and ground-truth JSONL files for larger recall suites.
- [x] Наблюдаемость (словарь метрик / отчёт по деградации графа) через расширенный `AnnSearchReport`.
- [x] Profile-aware durable HNSW construction through `DatabaseOptions::hnsw_build_config` for checkpoint/compact `.ach` graphs.

### 2) Real distributed consensus
- [ ] Отделить репликационный consensus-log и local WAL по строгим durability guarantees.
- [ ] Идемпотентный реплей с корректной синхронизацией term/index across crash/restart.
- [ ] Полный snapshot transfer + peer resync + membership lifecycle (join/leave/rotation).
- [ ] Split-brain + partition matrix тесты в CI (не только unit).
- [x] Базовая документированная модель Raft-like и внутренние модули протокола уже есть.
- [x] Есть первичные тесты `election/append/log/transport`.
- [x] Начальный failure-injection integration harness покрывает minority
  partition, stale leader rejection after healed majority, and idempotent
  replication-log replay after restart.
- [x] Chunked snapshot resync теперь проверяет contiguous chunks, consistent
  leader/term metadata, durable follower install, and stale-state replacement.
- [x] Membership lifecycle получил начальный reconfiguration API and tests for
  join/leave majority counting and invalid local-node removal.
- [x] In-memory transport получил partition-aware links and a five-node
  partition matrix covering minority writes, healed quorum, majority election,
  and stale minority leader rejection.
- [x] TCP snapshot transport smoke path now streams multi-chunk
  `SnapshotSegment` payloads and rejects a non-zero first chunk.
- [x] Durable peer snapshot install: a `ReplicationPeerServer` can receive TCP
  snapshot chunks and install the decoded `SnapshotSegment` into follower
  database storage, replacing stale local state across restart.
- [x] Membership rotation can now be encoded as replicated log entries and
  recovered from `replication.aclog` using only committed config entries.
- [x] Joint-consensus membership safety primitive: joint config entries encode
  old/new voter sets and commit only after majorities from both sets.
- [x] Crash/restart partition matrix seed coverage: partitioned leader restart
  preserves uncommitted entries without committing them, and committed leader
  restart catches up healed followers with the recovered commit index.

### 3) Full web UI (не embedded HTML only)
- [x] Вынести dashboard из Rust string modules в versioned static assets under `crates/cortex-server/assets/dashboard/v1`.
- [x] Завести frontend source-of-truth under `web/dashboard/src` plus `make dashboard-build` / `make dashboard-check`.
- [x] Завести standalone static build artifact under `web/dashboard/dist`, independent from the server crate asset copy.
- [ ] Превратить standalone static build в полноценный frontend-продукт с route-level pages и выбранным stack/release pipeline.
- [x] Наборы views в текущем static dashboard: overview, cells, search/explain, ANN, AQL, context, verify, ingest, storage health, cluster status.
- [x] Базовый tenant/token control в UI для scoped API calls; полноценный auth UX остаётся будущей standalone UI задачей.
- [x] Playwright/CI smoke путь для текущего `/dashboard`: asset loading, tabs, cell put/get, keyword search.
- [x] Playwright smoke дополнен search explain, storage validation, and cluster status.
- [x] Playwright screenshot artifacts for desktop/mobile dashboard review in CI.
- [x] Есть минимальный dashboard, static asset routes, and accessibility smoke tests.

### 4) Stable published SDK packages
- [x] Полный release-процедурный lock-step по версиям `server <-> OpenAPI <-> SDK`.
- [x] Автономные CI-пайплайны для публичных публикаций в crates.io / npm / PyPI (tag-gated, protected environment).
- [x] Deprecation policy и changelog для breaking changes в SDK contract.
- [x] Подготовлены release workflows и базовые contract checks (локально, в repo).

## Непосредственный следующий 2-недельный sprint

1. ANN/HNSW: опубликовать real-embedding baseline bundle для доменного корпуса.
2. ANN/HNSW: добавить долгий latency history gate вне быстрых unit тестов.
3. Consensus: expand the crash/restart partition matrix beyond the seed cases
   and automate membership rotation on top of the joint-consensus primitive.
4. UI: начать multi-page standalone app после текущих dashboard screenshot artifacts.
5. SDK: перейти к следующему продуктному слою после закрытия release/deprecation gates.

## Критерий перехода к следующему слою

- `cargo check --workspace`
- `cargo test --workspace --all-features`
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- Все contract snapshots не меняются без `API/SDK` change-log заметки.
