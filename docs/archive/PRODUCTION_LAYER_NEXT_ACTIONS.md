# Production Layer Execution Plan (post-Core Alpha)

> Status note: this is an execution snapshot, not the canonical public status.
> Use [`BETA_DELTA.md`](BETA_DELTA.md) for stable/experimental/beta-blocked
> classification and [`REMAINING_EXECUTION_PLAN.md`](REMAINING_EXECUTION_PLAN.md)
> for the active cycle.

## Цель
Сделать следующий слой системы production-oriented и при этом не ломать уже закрытый Core Alpha.

## Состояние на 2026-05-31

### 1) ANN/HNSW production guardrails
**Статус: ✅ Закрыто (ядро guardrails)**

- Стабильные recall/latency gates в CI:
  - `make ann-fixture-check`
  - `make ann-drift-check`
  - `make ann-external-check`
  - `make ann-metric-matrix-check`
- `production_safe=true` в текущих baseline/реальных run-ах.
- deterministic `ann_repeatable_report_json` + multi-layer `.ach` graph.
- Observability через ANN report (`ann_queries`, `fallback`, `slo_violations`, `query_profile`).

### 2) Real distributed consensus
**Статус: ⚠️ Частично закрыто (многие примитивы есть, но не «product-ready» как цель слоя)**

- Набор модулей для raft/log/snapshot/repair уже есть.
- Прогон matrix/смоки частично закрыт, однако осталось перейти к продуктовой стабильности:
  - операционный lifecycle узла (install/upgrade/rollback/health diagnostics)
  - устойчивость к длительным split-brain сценариям и recovery race
  - строгий production SLO для failover и client latency under churn

### 3) Full web UI (не embedded demo)
**Статус: ⚠️ Частично закрыто (developer console готов, продуктовый UI нет)**

- Имеется многостраничный статический dashboard (`web/dashboard` + assets).
- Осталось:
  - product-like flow (permissions UX, role-based screens, error surfaces)
  - полноценный routing/state model
  - более полные визуальные регрессионные тесты
  - release-grade polish/аудит usability

### 4) Stable published SDK packages
**Статус: ✅ Закрыто на контрактном уровне**

- Rust/Python/TS checks проходят.
- OpenAPI ↔ SDK contract check включен в `make`.
- Базовая политика выпуска и deprecation-check внедрены.
- Осталось: полноценный release pipeline в публичном реестре (если ещё не прогнан для release)
  и валидация версионного совместимого выпуска.

## Непосредственный Sprint (next 2 недели)

1. Закрыть оставшиеся product gaps в UI (по шагам из `docs/POST_CORE_ALPHA_PRODUCT_PLAN.md`):
   - финальный роутинг и state handling
   - error UX + admin/tenant flows
   - расширенный e2e/visual regression
2. Повторять local-only real-embedding ANN baseline runs, фиксировать history,
   and keep GitHub-hosted promotion deferred until beta.
3. Ускорить distributed consensus hardening:
   - долгоживущие partition/rejoin сценарии
   - операторские diagnostics и restart-safe observability.
4. Подготовить и зафиксировать релизный чек-лист для следующего слоя (`v0.1.x`), если решено публиковать.

## Что считать «production-safe=true» в ANN отчёте

- `production_safe=true` означает, что run не нарушил важные guardrails:
  - recall выше порога
  - граф не деградировал по уровню слоёв/структуре
  - fallback не был активирован
  - latency в заданных SLO пределах
- Если хотя бы один guardrail сработал, report может быть `production_safe=false`:
  - это **не ошибка** системы как такой,
  - но сигнал для routing/monitoring, что результаты не нужно считать надежным production default.
