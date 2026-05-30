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
- [ ] Нормализация/оценка качества для разных метрик/источников (dot/cosine/L2) на фиксированной матрице.
- [x] Наблюдаемость (словарь метрик / отчёт по деградации графа) через расширенный `AnnSearchReport`.

### 2) Real distributed consensus
- [ ] Отделить репликационный consensus-log и local WAL по строгим durability guarantees.
- [ ] Идемпотентный реплей с корректной синхронизацией term/index across crash/restart.
- [ ] Полный snapshot transfer + peer resync + membership lifecycle (join/leave/rotation).
- [ ] Split-brain + partition matrix тесты в CI (не только unit).
- [x] Базовая документированная модель Raft-like и внутренние модули протокола уже есть.
- [x] Есть первичные тесты `election/append/log/transport`.

### 3) Full web UI (не embedded HTML only)
- [ ] Вынести dashboard в отдельный frontend-продукт (SPA/static build), независимый от ручной разметки.
- [ ] Наборы страниц: dashboard, cells, AQL, search/context, verify, ingest, health/validation.
- [ ] Tenant-aware auth в UI и понятный путь ошибок.
- [ ] Playwright/CI smoke путь по критическим flows.
- [x] Есть минимальный dashboard (embedded HTML) и accessibility smoke tests.

### 4) Stable published SDK packages
- [ ] Полный release-процедурный lock-step по версиям `server <-> OpenAPI <-> SDK`.
- [ ] Автономные CI-пайплайны для публичных публикаций в crates.io / npm / PyPI (tag-gated).
- [ ] Deprecation policy и changelog для breaking changes в SDK contract.
- [x] Подготовлены release workflows и базовые contract checks (локально, в repo).

## Непосредственный следующий 2-недельный sprint

1. ANN/HNSW: расширить fixed corpus для dot/cosine/L2 и накопить benchmark history.
2. ANN/HNSW: добавить долгий latency history gate вне быстрых unit тестов.
3. Consensus: добавить failure-injection harness для partition/restart recovery в integration tests.
4. UI: вынести текущее dashboard html в versioned frontend asset в отдельной папке.
5. SDK: проверить публикационные версии в `sdk/python`, `sdk/typescript`, `rust` как единый release gate.

## Критерий перехода к следующему слою

- `cargo check --workspace`
- `cargo test --workspace --all-features`
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- Все contract snapshots не меняются без `API/SDK` change-log заметки.
