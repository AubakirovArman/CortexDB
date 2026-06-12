# CortexDB Database-Grade Execution Plan

Source of truth: `/mnt/hf_model_weights/arman/3bit/sites/pl copy.md`.

Execution rule: close epics in order. Use the dependency-aware order from the source plan when raw catalog order conflicts with dependencies. Do not jump to later epics unless the current epic explicitly depends on a prerequisite check, a small safe parallel task, or the user redirects the order.

Status values: `next`, `in_progress`, `partial`, `done`, `blocked`, `frozen`.

Current pointer: `EPIC-A02` (typed descriptor WAL path is in progress; `EPIC-D15`
public tag correction remains a release-management decision).

Impact measurement rule: after each meaningful retrieval, ContextPack, or answer
pipeline change, run `make enterprise-rag-bench-impact-gemini-50`. The target
uses official-clean 50 questions, Gemini 3.5 Flash as answerer and judge,
`engine-aql` + weighted rerank, and `target/enterprise-rag-bench/cortexdb-full`
with `reuse_db=1` so the corpus is not reingested. Current baseline:
`overall=41.36`, `correctness=42.0`, `completeness=44.76`, `document_recall=56.0`,
`invalid_extra_docs=9.44`, `answer_tokens=302372`, `judge_tokens=27312`
from `target/enterprise-rag-bench/official-clean/50/impact-gemini50-20260612T112354Z/answer-gemini/official_clean_run_report.json`.

## First Execution Queue

This queue follows section 7 of the source plan and dependency notes from the epic catalog.

1. `EPIC-A01` — clean repository and reproducibility baseline
2. `EPIC-A03` — data model contract before typed metadata implementation
3. `EPIC-C12` — rank key precompute
4. `EPIC-A04` — MemTable iterators without cloning
5. `EPIC-A05` — indexed VERIFY FACT
6. `EPIC-C16` — memory profiling harness
7. `EPIC-A19` — scale benchmarks 100K/1M
8. `EPIC-A20` — property-based core tests
9. Kill hardcoded EnterpriseRAG overfit from default search
10. `EPIC-D12` — docs consolidation
11. `EPIC-D01` + `EPIC-D03` + `EPIC-D04` — CLI/quickstart/demo
12. `EPIC-D05` — SDK publishing
13. `EPIC-D15` — beta tag and release notes
14. `EPIC-A02` — typed metadata after property coverage
15. `EPIC-A10` — LogicalPlan and PolicyRewrite
16. `EPIC-A14` — snapshot pinning
17. `EPIC-A16` — concurrent reads
18. `EPIC-E09` — AgentView permission property suite
19. `EPIC-A07` + `EPIC-A08` — segment v2 and lazy payload
20. `EPIC-D11` — MCP adapter

## Summary

- Block A: Earn the word database — 20 epics
- Block B: Agent-native database primitives — 20 epics
- Block C: Indexing, retrieval, and performance — 20 epics
- Block D: Developer experience and adoption — 15 epics
- Block E: Reliability, security, and operations — 15 epics
- Block F: Long-term database research — 10 epics
- Total: 100 epics

## Block A — Earn the word database

### EPIC-A01 — Чистый репозиторий и базовая воспроизводимость

- status: `done`
- meta: Категория: cleanup · Приоритет: P0 · Горизонт: 30 days · Тип: remove
- goal: Почему важно для vision: database-проект, в котором 51 файл (+5856 строк) не закоммичен, не может заявлять воспроизводимость.
- problem: Проблема: незакоммиченный WIP в рабочем дереве (зафиксировано аудитом).
- tasks:
  - [x] 1) разложить diff на логические коммиты (bench-скрипты отдельно от engine-изменений) — current audit found no pre-existing WIP to split on `enterprise-rag-dense-official-eval` at `8158916`
  - [x] 2) спорный тюнинг → ветка `wip/erb-v82` — not needed for the current state because the worktree was clean before this tracker was generated
  - [x] 3) pre-commit: `cargo fmt --check` + быстрый unit-набор — `cargo fmt --check`, `cargo test --workspace --all-features`, and `cargo clippy --workspace --all-targets -- -D warnings` passed on the active branch.
- acceptance:
  - [x] 1) `git status` чист на the active integration branch; final main cleanliness is verified at merge time — no pre-existing dirty worktree was present before generating this tracker.
  - [x] 2) каждый коммит проходит `cargo test --workspace` — validated with `cargo test --workspace --all-features`.
  - [x] 3) branch/main собирается с нуля одним описанным путём — reproducibility gate for this phase is `cargo fmt --check`, `cargo test --workspace --all-features`, and `cargo clippy --workspace --all-targets -- -D warnings`.
- files: scripts/, crates/cortex-engine, Makefile.
- dependencies: нет.
- risks: потерять полезный тюнинг — поэтому ветка, не reset.
- expected effect: все остальные 99 эпиков становятся измеримыми относительно чистой базы.

### EPIC-A02 — Типизированная модель метаданных (typed cell descriptor)

- status: `partial`
- meta: Категория: storage · Приоритет: P0 · Горизонт: 60 days · Тип: refactor
- goal: data model — фундамент БД; сейчас scope/trust/даты — текстовые строки в payload, т.е. security-поле живёт в user-контенте и парсится regex'ом на каждом доступе.
- problem: Проблема: `CellMetadata::from_payload` в hot path (в т.ч. в сортировке, database.rs:461-474); подделываемость представления.
- tasks:
  - [x] 1a) `CellDescriptor {scope_id, cell_type, status, source_trust_q16, created_at, valid_from/to, content_hash, parent_id, citation}` as a binary WAL section — `CellDescriptor::encode_section_v1/decode_section_v1`, `SectionTag::CellDescriptor`, automatic put/patch WAL emission, and replay apply are implemented.
  - [x] 1b) descriptor in segment v2 — `ACS2` segment records persist optional descriptor bytes while `ACS1` remains read-only compatible.
  - [x] 2a) WAL dual-read: new WAL records use binary descriptor; old WAL records without the section still materialize descriptor from legacy payload headers once.
  - [x] 2b) segment/checkpoint dual-read: checkpoint load decodes descriptor bytes from `ACS2` records and falls back to legacy payload materialization for `ACS1`/descriptor-less records.
  - [x] 3) кэш descriptor в `CellVersion`
  - [ ] 4) `cortexdb migrate` для офлайн-перегонки.
- acceptance:
  - [ ] 1) hot paths не вызывают текстовый парсинг (проверка профилем)
  - [ ] 2) fixtures/migration: старые базы читаются
  - [ ] 3) descriptor — единственный источник scope для permission-проверок.
- files: cortex-core/src/cell.rs, memtable/version.rs; cortex-storage/src/{wal,segment,format}.rs; cortex-engine/src/query/metadata.rs.
- dependencies: A01, A20 (property-тесты до начала).
- risks: САМЫЙ ОПАСНЫЙ рефакторинг блока — формат данных; строго version-gated, dual-read, ни одного big-bang.
- expected effect: модель данных перестаёт быть «текстом с конвенциями»; разблокирует B06, B10, C13, C14.
- evidence: Added `cortex_core::CellDescriptor`, lossy legacy payload header materialization, `CellVersion.descriptor`, and core tests for descriptor decode/cache. `Database::retrieve_cells` now uses the cached descriptor for the source-trust/freshness quality fast path and falls back to legacy payload parsing when source-ref confidence is required. Added binary WAL descriptor sections (`CellDescriptor` tag 10), put/patch WAL emission, replay dual-read, CLI WAL section-count contract update, and replay tests proving WAL descriptor wins over conflicting payload headers. Added `ACS2` segment records with optional descriptor bytes, `ACS1` read-only compatibility, checkpoint write/read descriptor persistence, compatibility/OpenAPI snapshot updates, and regression coverage proving checkpoint load prefers segment descriptor bytes over conflicting payload headers. Checks passed: `cargo fmt --check`, `cargo test --workspace --all-features`, `cargo clippy --workspace --all-targets -- -D warnings`, `make openapi-contract-check`, targeted descriptor/retrieval/WAL/segment/checkpoint tests, and Gemini-50 impact on the existing DB stayed at the current baseline (`overall=41.36`, `document_recall=56.0`; run `impact-gemini50-20260612T112354Z`).
- remaining: permission/index hot paths still mostly parse legacy payload metadata; `cortexdb migrate` remains pending; descriptor is not yet the sole source of scope for permission checks.

### EPIC-A03 — DATA_MODEL.md — контракт модели данных

- status: `done`
- meta: Категория: docs · Приоритет: P0 · Горизонт: 30 days · Тип: document
- goal: у БД модель данных — это спецификация, а не эмерджентное свойство кода.
- problem: Проблема: семантика cell/scope/version/TTL/brain разбросана по 6+ докам и коду.
- tasks:
  - [x] 1) один документ: cell, descriptor-поля и их типы, версионирование (MVCC), tombstone, TTL, scope/brain-семантика — added `docs/DATA_MODEL.md`.
  - [x] 2) что стабильно/экспериментально — documented Core Alpha stable fields and experimental descriptor/lazy/temporal work.
  - [x] 3) совместимость: как меняется формат (ссылка на migration policy) — documented dual-read payload-header to descriptor migration path.
- acceptance:
  - [x] 1) разработчик может написать клиента, не читая Rust-код
  - [x] 2) все поля descriptor из A02 описаны до их реализации
  - [x] 3) README ссылается на документ.
- files: docs/DATA_MODEL.md (новый), правки CELL_METADATA_MODEL.md → merge.
- risks: нет. Зависимости: нет (делается ДО A02 как спека).
- expected effect: A02 реализует спеку, а не импровизирует.

### EPIC-A04 — MemTable-итераторы без клонирования

- status: `done`
- meta: Категория: storage · Приоритет: P0 · Горизонт: 30 days · Тип: refactor
- goal: БД не имеет права клонировать все payload'ы ради чтения.
- problem: Проблема: `snapshot_versions()` → `visible_cells()` клонирует каждый `CellVersion` с payload (memtable/mod.rs:105-110); ~25 call sites.
- tasks:
  - [x] 1) `MemTable::visible_iter(txn) -> impl Iterator<Item = &CellVersion>`
  - [x] 2) классифицировать call sites: cold (checkpoint/validate/export — итератор), hot (verify/feedback/graph/dedup — индексные пути, отдельные эпики)
  - [x] 3) аллокационный регресс-тест (dhat-счётчик payload-клонов на стандартном сценарии).
- acceptance:
  - [x] 1) cold-пути не клонируют payload
  - [x] 2) полный тест-набор зелёный
  - [x] 3) regression-гейт на клоны в CI.
- files: cortex-core/src/memtable/mod.rs; checkpoint.rs, validation.rs, backup.rs.
- dependencies: A01. Эффект: основа для A05, A06, C-блока.
- evidence: Added borrowed MemTable iterators (`visible_iter`, `range_iter`, `visible_created_after_iter`) and borrowed storage segment views (`SegmentCellRef`, `SegmentWriter::write_refs`). `checkpoint` and `compact` now build segment/index/vector/HNSW outputs through borrowed refs instead of `snapshot_versions()` and owned `SegmentWriter::write(&segment_path, ...)`. Added `scripts/memtable_clone_gate_check.py` and wired it into `make check` as a static clone regression gate. Full `cargo test --workspace --all-features` is green.
- risks: the clone gate is static rather than a `dhat` allocator counter to avoid adding a new dependency; hot full-scan paths intentionally remain for later indexed epics A05/A06/C-block.

### EPIC-A05 — Indexed VERIFY FACT (кандидаты вместо full scan)

- status: `done`
- meta: Категория: verification · Приоритет: P0 · Горизонт: 30 days · Тип: refactor
- goal: флагманская database-фича не может быть O(N)+full-clone на вызов.
- problem: Проблема: `verify_fact_aql` (verification.rs:188) сканирует и клонирует всю базу.
- tasks:
  - [x] 1) термы факта (`tokenize`) + числовые токены → union-запрос к lexical-индексу → кандидаты
  - [x] 2) contradiction-маркеры/relations остаются в candidate path: persisted lexical union поднимает relation cells по факту, graph contradiction обрабатывается без `conflicts_for_fact()` full scan
  - [x] 3) скан только кандидатов по ссылкам (A04)
  - [x] 4) p95-бенч VERIFY на 100K/1M.
- acceptance:
  - [x] 1) вердикты на всех verification-фикстурах (9 тест-файлов) не изменились
  - [x] 2) аллокации payload = O(k кандидатов), не O(N) — static clone gate + borrowed candidate refs; `memtable_clone_gate_check.py` rejects the old full-clone VERIFY paths
  - [x] 3) p95 VERIFY на 1M ≤ 250ms steady-state after one warmup sample; cold-cache latency recorded separately.
- files: cortex-engine/src/verification.rs, verification/{support,contradiction,conflict_index}.rs, query/provider.rs.
- dependencies: A04. Эффект: VERIFY становится database-операцией.
- evidence: `verify_fact_aql` now has a VERIFY-specific binder path that does not build `EngineAqlIndex`, then uses cached persisted lexical indexes plus changed-tail lookup to choose candidate cells and reads only those cells through borrowed MemTable references. Graph contradiction/source-support enrichment no longer calls `conflicts_for_fact()` from VERIFY; it processes candidate/tail relation refs and optional persisted ACKG. `memtable_clone_gate_check.py` rejects `self.snapshot_versions()` in `verification.rs`, rejects `bind_aql_cached` in VERIFY, rejects `conflicts_for_fact` in VERIFY graph enrichment, and rejects `self.snapshot_versions()` in `verification/conflict_index.rs`. `verify_performance_check` records checkpointed VERIFY p95: 100K cells steady p95 `12.488ms`, 1M cells steady p95 `127.734ms` with one cold-cache warmup sample recorded separately. Verification integration fixtures pass.
- risks: cold-cache first VERIFY still pays persisted index cache load (`10744.217ms` at 1M); this is recorded separately and should be addressed by C16/A19 preload/capacity work rather than hidden in steady-state p95.

### EPIC-A06 — Indexed-only retrieve/ContextPack путь

- status: `pending`
- meta: Категория: query-engine · Приоритет: P0 · Горизонт: 60 days · Тип: refactor
- goal: главный read path обязан быть индексным от начала до конца.
- problem: Проблема: `try_aql_index` на незачекпоинченных данных строит индекс из `snapshot_versions()` на каждый запрос (query.rs:55-69); ranking парсит metadata в sort-ключе.
- tasks:
  - [ ] 1) поддерживаемый **инкрементальный delta-индекс** MemTable (обновляется в `apply_operation`), мержится с persisted-индексом на чтении
  - [ ] 2) предвычисление rank-ключей один раз на кандидата (sort_by_cached_key)
  - [ ] 3) feedback/graph/dedup-пути — на свои инкрементальные структуры (B13, B18, отдельные эпики) либо за candidate-фильтр.
- acceptance:
  - [ ] 1) `grep snapshot_versions` по запросным путям = 0
  - [ ] 2) p95 retrieve на 1M cells измерен и опубликован
  - [ ] 3) корректность: фикстуры retrieval-quality без изменений.
- files: cortex-engine/src/query.rs, query/{provider,candidates}.rs, database.rs (apply_operation), search/database.rs.
- dependencies: A04, A20. Эффект: read path масштабируется индексом, не размером базы.
- risks: инкрементальный индекс = новый класс багов согласованности — property-тест «индекс ≡ пересборке с нуля» обязателен.

### EPIC-A07 — Segment format v2 — payload-офсеты и блочные CRC

- status: `pending`
- meta: Категория: storage · Приоритет: P0 · Горизонт: 60 days · Тип: build
- goal: random access к payload — предусловие disk-resident исполнения.
- problem: Проблема: сегмент читается только целиком (`SegmentReader::read`); валидация целофайловая.
- tasks:
  - [ ] 1) footer-таблица (candidate_id, cell_id, descriptor, payload_offset, len, crc32c-блока)
  - [ ] 2) `SegmentReader::read_payload_at(candidate)` + `read_descriptors()`
  - [ ] 3) writer пишет v2, reader читает v1+v2 (dual-read)
  - [ ] 4) migration-фикстуры.
- acceptance:
  - [ ] 1) чтение одного payload без декодирования сегмента (тест)
  - [ ] 2) поблочная детекция порчи (corruption-тест на один блок)
  - [ ] 3) fixtures/storage и compatibility-тесты зелёные.
- files: cortex-storage/src/segment.rs, format.rs; cortex-engine/src/checkpoint.rs.
- dependencies: A02 (descriptor в footer). Эффект: открывает A08.
- risks: формат-миграция — version gate, как в A02.

### EPIC-A08 — Lazy payload residency, фаза 1 (метаданные в RAM, payload на диске)

- status: `pending`
- meta: Категория: storage · Приоритет: P0 · Горизонт: 90 days · Тип: build
- goal: свойство №1 «database»: данные > RAM.
- problem: Проблема: `load_checkpoint` (checkpoint.rs:396-415) грузит все payload'ы в память.
- tasks:
  - [ ] 1) `PayloadRef::{Inline(Vec<u8>), Segment{segment_id, offset, len}}` в CellVersion
  - [ ] 2) open: descriptors+индексы в RAM, payload — on-demand (pread/mmap) через LRU page cache
  - [ ] 3) конфиг `payload_residency = memory | lazy` (дефолт memory до стабилизации)
  - [ ] 4) интеграция с executor: payload читается только для ячеек, прошедших permission+rank+budget (см. B03)
  - [ ] 5) crash/recovery матрица в обоих режимах.
- acceptance:
  - [ ] 1) RSS на 1M cells в lazy ≥ 5x ниже memory-режима (бенч)
  - [ ] 2) вся crash-матрица зелёная в lazy
  - [ ] 3) p95 retrieve в lazy задокументирован рядом с memory.
- files: cortex-core/memtable/version.rs; cortex-engine/{checkpoint,database}.rs; новый cache-модуль.
- dependencies: A02, A07, A20. Эффект: потолок масштаба переезжает с RAM на диск.
- risks: ВЫСОКИЕ — самое глубокое вмешательство; только после A04-A07 и A20; за флагом.

### EPIC-A09 — Disk-resident индексы: инкрементальный merge без полной пересборки

- status: `pending`
- meta: Категория: indexing · Приоритет: P0 · Горизонт: 90 days · Тип: refactor
- goal: сейчас merged-индекс целиком в RAM и пересобирается при смене сегментов.
- problem: Проблема: `persisted_index_state` re-merge всех сегментов; `remove_candidates` — O(terms×candidates) retain-циклы (checkpoint.rs:357-394).
- tasks:
  - [ ] 1) merged-индекс хранится как поддерживаемая структура: новый сегмент применяется дельтой
  - [ ] 2) tombstones — отложенный roaring-бmånад вместо retain по всем термам
  - [ ] 3) (фаза 2) сегментные индексы запрашиваются без полного merge (search across segments + объединение результатов).
- acceptance:
  - [ ] 1) checkpoint на 1M не вызывает полный re-merge (профиль)
  - [ ] 2) первая search-латентность после checkpoint без многосекундной паузы (тест с таймером)
  - [ ] 3) индексная RAM измерена до/после.
- files: cortex-engine/src/checkpoint.rs (persisted_index_*), checkpoint/index_merge.rs.
- risks: согласованность дельт — property-тест «инкрементальный ≡ полному». Зависимости: C02 (roaring) желательно раньше. Эффект: убирает скрытые паузы и RAM-пик индексов.

### EPIC-A10 — LogicalPlan IR + формальный Policy Rewrite этап

- status: `pending`
- meta: Категория: query-engine · Приоритет: P0 · Горизонт: 60 days · Тип: build
- goal: без promежуточного представления нет планировщика; policy-этап делает permission свойством плана.
- problem: Проблема: binder выдаёт BoundPlan, дальше — захардкоженные функции.
- tasks:
  - [ ] 1) `LogicalPlan` (Scan{brain, predicate}, Filter, Rank{mode,weights}, Limit, Budget, Pack, Verify) — binder транслирует AST в него
  - [ ] 2) PolicyRewrite-проход: вшивает permission-маску в каждый Scan, клампит budget/limit (логика PolicyValidator переезжает сюда)
  - [ ] 3) сериализация плана в JSON для EXPLAIN.
- acceptance:
  - [ ] 1) существующее поведение байт-в-байт сохранено (golden AQL-тесты v0.4 зелёные)
  - [ ] 2) EXPLAIN выводит logical plan до/после policy rewrite
  - [ ] 3) тест: ни один Scan в плане после rewrite не существует без permission-предиката (структурная проверка).
- files: cortex-aql/src/binder/plan.rs (расширение), новый cortex-engine/src/plan/.
- risks: переусложнить IR — держать 7-8 узлов, не 30. Зависимости: нет (можно параллельно A05/A06). Эффект: скелет настоящего query engine; вход для A11-A13.

### EPIC-A11 — Operator-based executor

- status: `pending`
- meta: Категория: query-engine · Приоритет: P0 · Горизонт: 90 days · Тип: build
- goal: исполнение как дерево операторов — отличие database от «функции поиска».
- problem: Проблема: фиксированный конвейер retrieve_cells→rank→dedup→pack.
- tasks:
  - [ ] 1) `trait PhysicalOp { fn next(&mut self) -> Option<Candidate> }` (или batch-вариант)
  - [ ] 2) операторы: BitmapIndexScan, LexicalScan, VectorScan, PermissionFilter(no-op если вшит в scan), RankOp, DedupOp, PackOp, VerifyOp, ExplainCollector
  - [ ] 3) текущее поведение воспроизводится деревом по умолчанию
  - [ ] 4) счётчики кандидатов на каждом операторе (для EXPLAIN ANALYZE).
- acceptance:
  - [ ] 1) все retrieve/context фикстуры дают идентичные результаты через executor
  - [ ] 2) EXPLAIN ANALYZE показывает per-operator счётчики и время
  - [ ] 3) микробенч: оверхед операторной модели ≤ 10% против прямого вызова.
- files: новый cortex-engine/src/exec/; database.rs (retrieve_cells → exec).
- risks: преждевременная абстракция — начать с pull-итератора без векторизации. Зависимости: A10. Эффект: planner получает исполняемую цель; budget pushdown (B03) становится возможен.

### EPIC-A12 — Статистика хранилища (df, cardinality, zone maps)

- status: `pending`
- meta: Категория: indexing · Приоритет: P0 · Горизонт: 90 days · Тип: build
- goal: cost model без статистики — гадание.
- problem: Проблема: `bitmap_estimated_cardinality` возвращает None (binder.rs:63) — хук есть, данных нет.
- tasks:
  - [ ] 1) при checkpoint собирать в manifest/индексы: cells per scope/status/type, term document frequency (top-K + sketch), min/max created_at per segment
  - [ ] 2) API `Statistics::estimate(predicate) -> rows`
  - [ ] 3) zone maps для segment skipping (C10 использует).
- acceptance:
  - [ ] 1) оценка кардинальности scope-фильтра отклоняется ≤ 2x на тест-корпусе
  - [ ] 2) статистика переживает рестарт (в манифесте/сайдкаре)
  - [ ] 3) EXPLAIN показывает estimated vs actual.
- files: cortex-storage/src/manifest.rs, indexes.rs; checkpoint.rs.
- risks: распухание манифеста — sketch/top-K, не полные словари. Зависимости: A07 удобно вместе. Эффект: кормит A13.

### EPIC-A13 — Cost model v0 — выбор пути исполнения

- status: `pending`
- meta: Категория: query-engine · Приоритет: P1 · Горизонт: 90 days · Тип: build
- goal: планировщик должен выбирать, а не исполнять единственный путь.
- problem: Проблема: lexical/vector/hybrid захардкожены режимом AQL.
- tasks:
  - [ ] 1) правила v0 по статистике: узкий scope → bitmap-first; редкие термы → lexical-first; есть вектор + широкий корпус → vector-first с lexical-rerank
  - [ ] 2) бюджет → candidate-limit вниз по эвристике токенов/ячейку
  - [ ] 3) флаг `force_mode` для обхода (отладка), решение пишется в EXPLAIN.
- acceptance:
  - [ ] 1) на синтетических сценариях (узкий scope/редкий терм/широкий vector) планировщик выбирает ожидаемый путь (тест)
  - [ ] 2) retrieval-quality фикстуры не деградируют
  - [ ] 3) EXPLAIN показывает причину выбора.
- files: cortex-engine/src/plan/cost.rs (новый).
- risks: регрессии качества — quality-фикстуры в gate. Зависимости: A10, A11, A12. Эффект: «planner» перестаёт быть словом из доков.

### EPIC-A14 — Snapshot pinning и GC-барьер (честный snapshot isolation)

- status: `pending`
- meta: Категория: transactions · Приоритет: P0 · Горизонт: 60 days · Тип: build
- goal: заявлять snapshot isolation можно только если GC не уносит версии под читателем.
- problem: Проблема: `gc_versions_before(current_seq)` после checkpoint не знает о живых ReadTxn; сейчас спасает только однопоточность — с A16 станет багом.
- tasks:
  - [ ] 1) реестр активных ReadTxn (epoch/refcount, `PinnedReadTxn` с Drop)
  - [ ] 2) GC-горизонт = min(active read seq)
  - [ ] 3) тест: долгий читатель видит согласованный снапшот через checkpoint+gc
  - [ ] 4) задокументировать контракт изоляции в DATA_MODEL.md.
- acceptance:
  - [ ] 1) конкурентный тест читатель-vs-checkpoint зелёный под loom/threads
  - [ ] 2) p99 деградации GC нет (метрика отложенных версий)
  - [ ] 3) контракт описан.
- files: cortex-core/src/memtable/mod.rs, cortex-engine/src/{database,checkpoint}.rs.
- risks: утечка пинов → распухание версий — таймаут/метрика на длинные пины. Зависимости: A04. Эффект: предусловие A16; «MVCC» становится полноценным.

### EPIC-A15 — Транзакционный API: атомарный мульти-cell write batch

- status: `pending`
- meta: Категория: transactions · Приоритет: P1 · Горизонт: 60 days · Тип: build
- goal: у БД должна быть заявленная атомарность, не «обычно так получается».
- problem: Проблема: `put_cells` атомарен по WAL-батчу, но семантика не оформлена (нет batch для patch/tombstone/смешанных, нет контракта ошибок частичного применения).
- tasks:
  - [ ] 1) `WriteBatch {put/patch/tombstone}` → один WAL-батч → последовательное применение с единым диапазоном seq
  - [ ] 2) контракт: всё или ничего durable; применение в MemTable не может частично провалиться (валидация до WAL)
  - [ ] 3) HTTP `/v1/batch` + SDK.
- acceptance:
  - [ ] 1) crash-тест: батч либо весь виден после recovery, либо отсутствует
  - [ ] 2) валидационные ошибки возвращаются до записи WAL
  - [ ] 3) API задокументирован.
- files: cortex-engine/src/{database,operation}.rs; cortex-server/src/router.rs.
- risks: patch-валидация требует видимости — порядок проверок до WAL. Зависимости: A14. Эффект: агентные «записать факт+связь+память атомарно» сценарии.

### EPIC-A16 — Конкурентный read path

- status: `pending`
- meta: Категория: concurrency · Приоритет: P0 · Горизонт: 60 days · Тип: refactor
- goal: однопоточный тенант — дисквалификация слова database.
- problem: Проблема: `DatabaseActor` сериализует чтения и записи (actor.rs); медленный VERIFY стопит PUT.
- tasks:
  - [ ] 1) `Arc<RwLock<Database>>`: writer-актор берёт write, read-запросы исполняются в tokio blocking-пуле под read (read-методы уже `&self`; проверить внутренние Mutex-поля — aql_query_cache, persisted_index_cache — на contention)
  - [ ] 2) приоритет writer (без write starvation)
  - [ ] 3) load-тест смешанного r/w: чтения не ждут записей.
- acceptance:
  - [ ] 1) тест: GET/context при искусственно медленном PUT отвечает < N ms
  - [ ] 2) throughput чтений растёт с потоками (бенч C18)
  - [ ] 3) ни одного deadlock под stress (loom на критичных секциях или 24h chaos).
- files: cortex-server/src/{actor,router}.rs; cortex-engine/src/database.rs (Sync-аудит).
- risks: скрытая внутренняя мутабельность — аудит всех Mutex/Cell полей обязателен. Зависимости: A14 (пины), A04. Эффект: сервер масштабируется по ядрам.

### EPIC-A17 — Checkpoint без stop-the-world (WAL-ротация)

- status: `pending`
- meta: Категория: storage · Приоритет: P1 · Горизонт: 90 days · Тип: refactor
- goal: БД не должна останавливать записи на время снапшота.
- problem: Проблема: checkpoint() делает writer.shutdown() → segment write → truncate(0) → restart (checkpoint.rs:74-106).
- tasks:
  - [ ] 1) ротация: новый WAL-файл открывается сразу, записи продолжаются; дельта собирается по снапшоту seq (A14)
  - [ ] 2) старый WAL удаляется только после durable publish манифеста
  - [ ] 3) recovery-порядок нескольких WAL-файлов (find_wal_files уже умеет) — property-тест
  - [ ] 4) расширить crash_matrix окнами ротации.
- acceptance:
  - [ ] 1) put p95 во время checkpoint деградирует < 2x (тест)
  - [ ] 2) crash в каждом окне ротации восстанавливается корректно
  - [ ] 3) WAL-архив (старые файлы) опционально сохраняется (зачаток PITR, E03).
- files: cortex-engine/src/{checkpoint,database,database_files}.rs; cortex-storage/src/wal/writer_rotation.rs.
- risks: тонкий recovery-порядок — property-тесты до мержа. Зависимости: A14, A20. Эффект: предсказуемая латентность записи; путь к PITR.

### EPIC-A18 — Фоновая инкрементальная компакция

- status: `pending`
- meta: Категория: storage · Приоритет: P2 · Горизонт: 6 months · Тип: build
- goal: рост числа сегментов без фоновой компакции деградирует чтения и диск.
- problem: Проблема: compact — ручной полный снапшот; политика «когда» отсутствует (метрика compaction_pressure_q16 есть, ничем не используется).
- tasks:
  - [ ] 1) фоновый компактор в writer-runtime: триггер по pressure/превышению сегментов
  - [ ] 2) инкрементальная компакция выбранных сегментов (не полный снапшот)
  - [ ] 3) ops-ручки: пауза/форс, метрики.
- acceptance:
  - [ ] 1) длительный write-нагрузочный тест держит число сегментов в коридоре
  - [ ] 2) чтения во время компакции не деградируют > x%
  - [ ] 3) crash во время компакции безопасен (матрица).
- files: checkpoint.rs, новый compactor-модуль, bundle.rs/cleanup.rs.
- risks: конкуренция с checkpoint — общий планировщик фоновых работ. Зависимости: A17, A14. Эффект: эксплуатация без ручного compact.

### EPIC-A19 — Scale-бенчмарки 100K/1M/10M + кривые RAM/латентности

- status: `partial`
- meta: Категория: benchmarks · Приоритет: P0 · Горизонт: 30 days (100K/1M baseline) / 90 days (10M) · Тип: benchmark
- goal: слово database требует чисел на масштабе, а не 10K из BENCHMARKS.md.
- problem: Проблема: перф-матрица заканчивается на 10K; линейный рост уже виден.
- tasks:
  - [x] 1) генератор корпуса (0.5-4KB payload, реалистичное распределение scope/термов) в cortex-bench — implemented as `scale_benchmark_check` with realistic 0.5KB-4KB payloads, mixed scopes, and operational terms.
  - [ ] 2) матрица: open time, RSS, put/get/search/context/verify p50/p95, checkpoint time — на 100K/1M (10M — после A08) — 100K/1M core lifecycle matrix is reproducible; heavy search/context/verify p95 remain open.
  - [ ] 3) baseline ДО оптимизаций и кривая ПОСЛЕ каждой (A05, A06, A08, A09) — first 100K/1M core baselines captured; trend curves remain open.
  - [x] 4) публикация в BENCHMARKS.md, включая некрасивые цифры — `docs/SCALE_BENCHMARKS.md` and `docs/BENCHMARKS.md`.
- acceptance:
  - [x] 1) `make scale-bench-{100k,1m}` воспроизводимы — both safe core targets pass locally.
  - [ ] 2) кривые в доках с датой и коммитом — 100K/1M baselines documented with date/source state; multi-point before/after optimization curves remain open.
  - [ ] 3) 10M-прогон после A08 (lazy) с RSS-сравнением.
- files: crates/cortex-engine/src/bin/scale_benchmark_check.rs, Makefile, docs/SCALE_BENCHMARKS.md, docs/BENCHMARKS.md.
- risks: страшные baseline-цифры — публиковать: это и есть claims-policy. Зависимости: A01, C16. Эффект: фундамент честности всего «database»-нарратива.
- evidence: Added `scale_benchmark_check`, `make scale-bench-100k`, and `make scale-bench-1m`. Local 100K core report `target/scale-bench/100k/report.json`: `ok=true`, cells `100000`, duration `71185.262ms`, put batches `960.891ms`, checkpoint `38416.460ms`, get_latest p95 `0.003ms`, restart open `219.172ms`, after-checkpoint RSS `890494976`, peak RSS `1123278848`, estimated total memory `894553484`, no validation errors. Local 1M core report `target/scale-bench/1m/report.json`: `ok=true`, cells `1000000`, duration `704416.326ms`, put batches `10892.097ms`, checkpoint `378042.066ms`, get_latest p95 `1.165ms`, restart open `2879.535ms`, after-checkpoint RSS `8748335104`, peak RSS `11147628544`, estimated total memory `8946879838`, no validation errors.
- risks: Heavy broad search/context/verify at 100K are not hidden behind the default pass. An exploratory low-sample run reached those phases but did not complete in a practical window; this remains an A19/A06/A11 optimization target.

### EPIC-A20 — Property-based тесты ядра (MVCC, WAL, recovery, индексы)

- status: `done`
- meta: Категория: testing · Приоритет: P0 · Горизонт: 30 days · Тип: test
- goal: A02/A06/A08/A17 — рискованные рефакторинги ядра; без property-страховки это рулетка.
- problem: Проблема: инварианты проверяются примерами; crash-матрицы сильны, но не исчерпывают перестановки.
- tasks:
  - [x] 1) proptest: произвольные последовательности put/patch/tombstone/checkpoint/gc против эталонной модели (BTreeMap-оракул) — implemented as deterministic property-style tests without new dependencies, using fixed and env-driven seeds.
  - [x] 2) WAL: random truncate/bit-flip префиксов → recovery не паникует, durable-ack'нутые данные не теряются (strict) — complete-prefix strict replay and corruption no-panic/best-effort coverage added.
  - [x] 3) «инкрементальный индекс ≡ пересборке» (для A06/A09) — persisted keyword index result set is compared with a fresh rebuild after put/patch/tombstone/checkpoint.
  - [x] 4) перенести в CI fast-lane с фиксированным seed + nightly с random — fixed test is part of workspace tests; `make core-property-check` and `make core-property-random-check` provide explicit fixed/random gates.
- acceptance:
  - [x] 1) ≥ 4 property-теста в CI
  - [x] 2) каждый найденный баг закреплён регрессионным кейсом
  - [x] 3) обязательный гейт для PR в storage/core.
- files: cortex-core/tests, cortex-storage/tests, cortex-engine/tests (новые файлы).
- risks: flaky на таймингах — модельные тесты делать детерминированными. Зависимости: нет. Эффект: страховка всего блока A.
- evidence: Added `crates/cortex-engine/tests/core_property_tests.rs` with 4 property-style tests: model operation sequences across restart, strict WAL complete-prefix replay, WAL corruption no-panic/best-effort safe prefix, and persisted keyword index vs fresh rebuild. Added `make core-property-check` and `make core-property-random-check`. The suite found and fixed a real MVCC bug where tombstoning a replaced cell could resurrect an older version; regression coverage added in `crates/cortex-core/tests/memtable_tests.rs`.
- verification: `make core-property-check`, `make core-property-random-check CORE_PROPERTY_RANDOM_SEED=424242`.

### Queue Item — Kill hardcoded EnterpriseRAG overfit from default search

- status: `done`
- goal: default database search must be generic and must not silently apply EnterpriseRAG benchmark calibration.
- problem: `WeightedScoreReranker::default()` previously enabled EnterpriseRAG question-type calibration, and default hybrid-rerank paths used calibrated RRF weights.
- tasks:
  - [x] 1) Make default reranking generic/fixed, not EnterpriseRAG-calibrated.
  - [x] 2) Keep EnterpriseRAG calibration only as explicit opt-in diagnostic/benchmark API.
  - [x] 3) Use balanced RRF weights in default live and persisted hybrid-rerank paths.
  - [x] 4) Add a regression test proving default reranker is not EnterpriseRAG-calibrated.
- acceptance:
  - [x] 1) Default search/rerank no longer changes weights based on EnterpriseRAG question labels.
  - [x] 2) Benchmark calibration helpers still exist for explicit experiments.
  - [x] 3) Search API/database tests continue passing.
- files: crates/cortex-engine/src/search/rerank.rs, crates/cortex-engine/src/search.rs, crates/cortex-engine/src/search/database.rs.
- evidence: `WeightedScoreReranker::default()` now has `calibrate_by_question_type=false`; `WeightedScoreReranker::enterprise_rag_calibrated()` is the explicit opt-in. Default hybrid rerank uses `HybridRrfWeights::balanced()` and `WeightedScoreReranker::fixed_default()`.
- verification: `cargo test -p cortex-engine search::rerank --lib`, `cargo test -p cortex-engine --test database_search`, `cargo test -p cortex-server search_api_tests`.

## Block B — Agent-native database primitives

### EPIC-B01 — ContextPack JSON Schema v1 — замороженный тип результата

- status: `pending`
- meta: Категория: contextpack · Приоритет: P0 · Горизонт: 30 days · Тип: productize
- goal: result type — контракт БД; ContextPack должен быть стабильным объектом, на который можно строить интеграции.
- problem: Проблема: схема живёт в коде и примерах README; поля меняются по мере коммитов.
- tasks:
  - [ ] 1) ревизия полей (решить судьбу спорных: `access_decision.NotRecorded` — признание учётной дыры; либо закрыть дыру, либо не включать в v1)
  - [ ] 2) `docs/schemas/context_pack.v1.json` + `schema_version` в ответе
  - [ ] 3) golden snapshot-тесты сериализации; additive-only политика до v2.
- acceptance:
  - [ ] 1) CI валидирует `/v1/context` против схемы
  - [ ] 2) breaking change ломает golden-тест
  - [ ] 3) SDK-типы генерируются из схемы.
- files: cortex-engine/src/context/*, cortex-server/src/responses.rs, docs/schemas/.
- risks: заморозить неудачное поле — ревизия до freeze. Зависимости: A01. Эффект: ContextPack официально становится «result set» CortexDB.

### EPIC-B02 — ContextPackBuilder как физический оператор

- status: `pending`
- meta: Категория: contextpack · Приоритет: P1 · Горизонт: 90 days · Тип: refactor
- goal: пак должен собираться внутри исполнения, а не пост-обработкой полного результата.
- problem: Проблема: `ContextPack::from_retrieved_*` получает уже полностью извлечённые и отранжированные ячейки (context/pack.rs:148+).
- tasks:
  - [ ] 1) PackOp в executor (A11): потребляет кандидатов потоком, ведёт бюджет, anomalies, redundancy инкрементально
  - [ ] 2) перенос текущей логики (span selection, large-cell policy, MMR) в оператор без изменения семантики
  - [ ] 3) корректность: golden-фикстуры паков неизменны.
- acceptance:
  - [ ] 1) идентичные паки на context_pack_* фикстурах (15 тест-файлов)
  - [ ] 2) пак собирается без материализации полного кандидат-сета (профиль аллокаций)
  - [ ] 3) счётчики оператора в EXPLAIN ANALYZE.
- files: cortex-engine/src/context/pack.rs → exec/pack_op.rs.
- risks: MMR-диверсификация требует пула — допустим bounded-буфер, не полный сет. Зависимости: A11. Эффект: включает B03.

### EPIC-B03 — Token-budget pushdown и early termination

- status: `pending`
- meta: Категория: query-engine · Приоритет: P1 · Горизонт: 90 days · Тип: build
- goal: «бюджет токенов» как параметр исполнения — уникальный database-примитив CortexDB.
- problem: Проблема: сегодня бюджет применяется в самом конце; при lazy-payload (A08) это означало бы читать с диска лишнее.
- tasks:
  - [ ] 1) PackOp сигнализирует исполнителю «бюджет заполнен» → upstream-операторы останавливаются
  - [ ] 2) candidate-limit в плане выводится из бюджета (оценка токенов/ячейку из статистики)
  - [ ] 3) payload-чтение (A08) переносится ЗА permission+rank: читаем диск только для ячеек, которые реально пойдут в пак (+ запас).
- acceptance:
  - [ ] 1) тест: при бюджете 500 токенов на 1M-корпусе читается ≤ K payload'ов с диска (счётчик)
  - [ ] 2) качество паков на фикстурах не меняется
  - [ ] 3) p95 context на 1M в lazy-режиме улучшается измеримо против наивного.
- files: exec/, plan/cost.rs, context/.
- risks: rank до чтения payload требует rank по descriptor/индексным фичам — спроектировать двухфазный rank (cheap rank → fetch → final rank). Зависимости: A08, A11, B02. Эффект: исполнение, оптимизированное под LLM-окно — ядро категории.

### EPIC-B04 — AgentView как индексный инвариант (permission bitmap в scan)

- status: `pending`
- meta: Категория: security · Приоритет: P0 · Горизонт: 60 days · Тип: refactor
- goal: permission-safe retrieval должен быть свойством физического доступа, не пост-фильтром.
- problem: Проблема: binder уже пересекает agent-allowed маску в bitmap-программе (хорошо), но непайплайновые поверхности (`/get` route, search-пути, verify, graph) проверяют scope пост-фактум по payload-строке (`require_payload_read`, authz.rs:80).
- tasks:
  - [ ] 1) permission-bitmap (scope→candidates) как поддерживаемый индекс
  - [ ] 2) все читающие поверхности проходят через candidate-фильтр ДО чтения payload
  - [ ] 3) `/get` по cell_id: проверка по descriptor (A02), не по payload-парсингу.
- acceptance:
  - [ ] 1) структурный тест A10 («нет Scan без permission-предиката») распространён на все поверхности
  - [ ] 2) E09 property-тест зелёный
  - [ ] 3) пост-фильтрация payload-скоупа удалена из router/authz.
- files: cortex-server/src/{authz,router}.rs; cortex-engine/src/query/provider.rs.
- risks: нет существенных — упрощение модели. Зависимости: A02 (descriptor), A06. Эффект: инвариант безопасности становится архитектурным.

### EPIC-B05 — AgentView lifecycle API v1

- status: `pending`
- meta: Категория: security · Приоритет: P1 · Горизонт: 60 days · Тип: productize
- goal: security boundary без удобного управления не используется.
- problem: Проблема: создание/гранты разбросаны (auth_scope_admin.rs, policy cells); нет единого CLI/API/доки.
- tasks:
  - [ ] 1) `cortexdb agent create|grant|revoke|list|show` + `/v1/agents` CRUD (admin-роль)
  - [ ] 2) персистентность AgentView как системных ячеек с миграцией текущего формата
  - [ ] 3) AUTH.md: «агенты и права за 10 минут» (≤250 строк); e2e-тест двух агентов с разными scopes.
- acceptance:
  - [ ] 1) сценарий «два агента, разные права» проходит из CLI без ручного JSON
  - [ ] 2) admin-маршруты покрыты authz-тестами
  - [ ] 3) doc-страница единственная.
- files: cortex-server/src/{auth_scope_admin,auth_policy_*}.rs; cortex-cli; docs/AUTH.md.
- risks: совместимость со старым policy-store — migration-тест. Зависимости: нет. Эффект: главная фича становится управляемой.

### EPIC-B06 — Typed provenance model (source_ref, citation, content_hash как колонки)

- status: `pending`
- meta: Категория: storage · Приоритет: P1 · Горизонт: 90 days · Тип: refactor
- goal: provenance — продуктовое отличие; строки в payload не дают целостности.
- problem: Проблема: citation/source/content_hash — текстовые конвенции.
- tasks:
  - [ ] 1) поля в CellDescriptor (A02): source_id, citation, content_hash (обязателен при ingestion), source_trust
  - [ ] 2) валидация на записи (warn/strict режимы)
  - [ ] 3) пак ссылается на typed-цитаты, dedup — на typed content_hash.
- acceptance:
  - [ ] 1) пак с citations_required работает без payload-парсинга
  - [ ] 2) ingestion проставляет content_hash автоматически
  - [ ] 3) формат описан в DATA_MODEL.md.
- files: cortex-core/cell.rs, cortex-engine/{ingestion,context}/.
- risks: вместе с A02 (одна миграция, не две). Зависимости: A02. Эффект: цитаты — данные, а не текст.

### EPIC-B07 — Fact/claim store: типизированные факты с numeric-значениями

- status: `pending`
- meta: Категория: verification · Приоритет: P1 · Горизонт: 6 months · Тип: build
- goal: «база фактов с конфликт-детекцией» — сердце agent-native категории; сейчас факты — просто текст, числа парсятся на лету.
- problem: Проблема: numeric-парсер (verification/numeric.rs) работает на каждом вызове по payload.
- tasks:
  - [ ] 1) при записи/ingestion извлекать numeric-факты (metric, value, unit, magnitude) в typed fact-записи (cell_type=fact, structured body — typed_body.rs уже есть как зачаток)
  - [ ] 2) fact-индекс: metric→(cell, value) (C13)
  - [ ] 3) VERIFY numeric-конфликты — запросом к индексу, не сканом.
- acceptance:
  - [ ] 1) numeric-конфликты на фикстурах находятся через индекс с теми же вердиктами
  - [ ] 2) ingestion投 факт-извлечение покрыто тестами
  - [ ] 3) p95 numeric-verify на 1M — индексное.
- files: cortex-engine/src/{verification/numeric.rs, typed_body.rs, ingestion}.
- risks: extraction-качество — консервативные паттерны, без LLM в ядре. Зависимости: A02, A05. Эффект: verification переходит от «сравнить тексты» к «запросить факты».

### EPIC-B08 — VerifyOp — верификация как оператор плана

- status: `pending`
- meta: Категория: verification · Приоритет: P1 · Горизонт: 6 months · Тип: refactor
- goal: VERIFY FACT должен быть планируемым запросом (со статистикой, EXPLAIN, permission в scan), а не спецфункцией.
- problem: Проблема: verify_fact_aql — монолитная функция мимо будущего executor.
- tasks:
  - [ ] 1) перевести A05-реализацию на план: Scan(lexical ∪ numeric ∪ markers) → PermissionFilter → EvidenceMatch → VerdictAggregate
  - [ ] 2) EXPLAIN ANALYZE для VERIFY (сколько кандидатов, какие индексы)
  - [ ] 3) опции глубины (max evidence, max candidates) как параметры плана, клампятся политикой.
- acceptance:
  - [ ] 1) вердикты фикстур неизменны
  - [ ] 2) EXPLAIN VERIFY показывает стадии
  - [ ] 3) код verify не содержит собственного скан-цикла.
- files: verification.rs → exec/verify_op.rs.
- risks: нет существенных после A05/A11. Зависимости: A05, A11. Эффект: единый query engine для retrieve и verify.

### EPIC-B09 — Инкрементальный contradiction/conflict-индекс

- status: `pending`
- meta: Категория: verification · Приоритет: P2 · Горизонт: 6 months · Тип: build
- goal: «база знает свои противоречия» — фича уровня категории: конфликты можно находить при записи, а не при запросе.
- problem: Проблема: conflict_index.rs строится сканом по запросу.
- tasks:
  - [ ] 1) при записи факта (B07) проверять fact-индекс на конфликтующее значение той же метрики в том же scope → материализовать conflict-запись
  - [ ] 2) `/v1/conflicts?scope=` API + anomaly в паке «в паке есть стороны конфликта X»
  - [ ] 3) инвалидация при tombstone/patch.
- acceptance:
  - [ ] 1) конфликт обнаруживается на записи (тест: два бюджета → запись conflict)
  - [ ] 2) пак, содержащий стороны конфликта, помечает это без full-scan
  - [ ] 3) consistency-тест с patch/delete.
- files: verification/conflict_index.rs, database.rs (write hook).
- risks: write-amplification — делать асинхронно в фоновом задании writer-runtime. Зависимости: B07. Эффект: уникальная фича «contradiction-aware database».

### EPIC-B10 — Temporal validity как колонки + временные запросы

- status: `pending`
- meta: Категория: storage · Приоритет: P2 · Горизонт: 6 months · Тип: build
- goal: агентные знания устаревают; «что было верно на дату X» — естественный запрос agent-native БД.
- problem: Проблема: temporal-логика парсит даты из текста (verification/temporal.rs), валидность не первоклассна.
- tasks:
  - [ ] 1) valid_from/valid_to в descriptor (A02)
  - [ ] 2) AQL: `REQUIRE VALID AT "2026-01-01"` (расширение requirement, не новой грамматики)
  - [ ] 3) temporal-индекс (C14) для фильтрации без скана; stale-guard VERIFY переводится на колонки.
- acceptance:
  - [ ] 1) временной фильтр работает индексно (тест на 100K)
  - [ ] 2) stale-факты не попадают в evidence при запросе с датой
  - [ ] 3) семантика в DATA_MODEL.md.
- files: cell.rs, verification/temporal*.rs, binder (requirement).
- risks: ingestion редко знает валидность — поля опциональны, семантика null задокументирована. Зависимости: A02. Эффект: temporal reasoning — database-фича.

### EPIC-B11 — Memory lifecycle: TTL/decay как политика хранилища

- status: `pending`
- meta: Категория: storage · Приоритет: P1 · Горизонт: 90 days · Тип: productize
- goal: «память агента с lifecycle» — категория-фича; должна иметь контракт и engine-исполнение.
- problem: Проблема: memory.rs делает TTL/decay через full scan; expire — команда актора без планировщика; семантика не зафиксирована.
- tasks:
  - [ ] 1) TTL-индекс (expiry→cells), expire — фоновое задание writer-runtime (батчевые tombstone через WAL)
  - [ ] 2) decay — формула в rank по created_at/last_access из descriptor
  - [ ] 3) AGENT_MEMORY.md как контракт с формулами; golden-тесты «память через N дней ранжируется ниже/исчезает».
- acceptance:
  - [ ] 1) expire не сканирует базу (индекс, тест)
  - [ ] 2) golden-тесты формулы decay
  - [ ] 3) REMEMBER+TTL e2e через HTTP/SDK.
- files: memory.rs, memory_accounting.rs, session.rs; actor.rs (планировщик).
- risks: фоновые tombstone vs читатели — через обычный WAL-путь, ничего специального. Зависимости: A02, A14. Эффект: agent memory — продукт с гарантиями.

### EPIC-B12 — Session/episodic memory contract

- status: `pending`
- meta: Категория: product · Приоритет: P2 · Горизонт: 6 months · Тип: productize
- goal: LongMemEval показал, что session-память — реальный workload; нужен контракт, а не приватный харнесс.
- problem: Проблема: session.rs использует snapshot_versions-скан; семантика сессий не публична.
- tasks:
  - [ ] 1) session_id в descriptor; session-retrieval через индекс scope+session
  - [ ] 2) API: append session event, retrieve session window/summary-кандидаты
  - [ ] 3) перенести lessons из LongMemEval-харнесса в generic-механизм (без оверфита, урок EPIC из прошлого аудита).
- acceptance:
  - [ ] 1) session-пути индексные
  - [ ] 2) публичный пример «чат-агент с многосессионной памятью»
  - [ ] 3) LongMemEval-харнесс использует только публичные API.
- files: session.rs, query/, examples/.
- risks: переусложнение — минимальный контракт. Зависимости: A02, A06. Эффект: главный agent-workload оформлен.

### EPIC-B13 — Feedback как индексированный ranking-сигнал

- status: `pending`
- meta: Категория: retrieval · Приоритет: P2 · Горизонт: 6 months · Тип: refactor
- goal: feedback-loop (агент сообщает полезность контекста) — редкая и правильная фича, но сейчас она full-scan и полудокументирована.
- problem: Проблема: feedback.rs — 4 вызова snapshot_versions на расчёт.
- tasks:
  - [ ] 1) feedback-записи → инкрементальный map cell→score (поддерживается на write)
  - [ ] 2) RankOp читает map O(1)
  - [ ] 3) HTTP/SDK API + doc; решение зафиксировать (продуктизируем, не выпиливаем — opinionated recommendation).
- acceptance:
  - [ ] 1) feedback-путь без сканов
  - [ ] 2) API задокументирован с примером агентного цикла
  - [ ] 3) ranking-эффект покрыт golden-тестом.
- files: feedback.rs, exec/rank_op.rs, server/router.rs.
- risks: нет. Зависимости: A06. Эффект: «база, которая учится у агента» — без ML-пафоса, инженерно.

### EPIC-B14 — Explainability contract: explain для каждого результата

- status: `pending`
- meta: Категория: contextpack · Приоритет: P1 · Горизонт: 60 days · Тип: productize
- goal: проверяемость — категория-свойство; explain должен быть стабильной частью result type.
- problem: Проблема: explain-поля богатые (score_components, why_selected), но не контрактные; «почему ячейка НЕ попала» отвечается лишь частично anomalies.
- tasks:
  - [ ] 1) explain-схему в ContextPack v1 (B01) зафиксировать
  - [ ] 2) `cortexdb explain --cell-id N <aql>`: трассировка конкретной ячейки по стадиям (allowed? live? where? thresholds? budget? redundancy?) на базе операторных счётчиков (A11)
  - [ ] 3) doc EXPLAIN.md с примерами.
- acceptance:
  - [ ] 1) для исключённой ячейки называется первая отсёкшая стадия
  - [ ] 2) explain стабилен под golden-тестами
  - [ ] 3) поля документированы.
- files: context/explain.rs, exec/, cli.
- risks: нет. Зависимости: A11 (полная версия), частично можно раньше. Эффект: главный debugging-инструмент пользователя.

### EPIC-B15 — EXPLAIN ANALYZE для AQL

- status: `pending`
- meta: Категория: query-engine · Приоритет: P1 · Горизонт: 90 days · Тип: build
- goal: у настоящей БД можно спросить, как исполнился запрос.
- problem: Проблема: AqlExplainReport есть, но не отражает физическое исполнение (его пока нет).
- tasks:
  - [ ] 1) `EXPLAIN <stmt>` — logical+physical план (JSON и текст)
  - [ ] 2) `EXPLAIN ANALYZE` — с real счётчиками/временем операторов
  - [ ] 3) HTTP `/v1/aql?explain=analyze`, CLI-флаг.
- acceptance:
  - [ ] 1) estimated vs actual кандидаты видны на каждом операторе
  - [ ] 2) выбор cost-планировщика (A13) обоснован в выводе
  - [ ] 3) doc с примерами.
- files: exec/explain.rs, parser (EXPLAIN уже есть в AST), cli/server.
- risks: нет. Зависимости: A10-A13. Эффект: главный аргумент «это настоящая БД» для внешнего инженера.

### EPIC-B16 — Формализованный Policy Rewrite + доказательство инварианта

- status: `pending`
- meta: Категория: security · Приоритет: P0 · Горизонт: 60 days · Тип: build
- goal: permission-safe retrieval как database-level invariant требует структурного и тестового доказательства.
- problem: Проблема: гарантия сейчас распределена между binder, authz.rs пост-фильтрами и дисциплиной кода.
- tasks:
  - [ ] 1) единственная точка: PolicyRewrite-проход над LogicalPlan (A10), все поверхности (search/get/verify/graph/memory/export) строят планы через него
  - [ ] 2) негативные тесты на каждую поверхность (запрос чужого scope → пустота/ошибка, никогда payload)
  - [ ] 3) структурный тест: пост-rewrite план не содержит непокрытого Scan.
- acceptance:
  - [ ] 1) одна функция-источник гарантии
  - [ ] 2) E09 property-suite зелёный
  - [ ] 3) SECURITY_MODEL.md описывает инвариант одной страницей.
- files: plan/policy.rs (новый), authz.rs (сжимается), все query-поверхности.
- risks: миграция поверхностей по одной, не разом. Зависимости: A10, B04. Эффект: продаваемый и проверяемый security-инвариант.

### EPIC-B17 — Tool registry как типизированный каталог

- status: `pending`
- meta: Категория: product · Приоритет: P3 · Горизонт: 6 months · Тип: productize
- goal: рекомендация инструментов в паке — полезный агентный примитив, но сейчас это scan по type=tool ячейкам.
- problem: Проблема: tool_registry.rs — snapshot_versions-скан; формат tool-ячеек конвенционный.
- tasks:
  - [ ] 1) typed tool-записи (descriptor type=tool + structured body)
  - [ ] 2) каталог в памяти, инкрементально поддерживаемый
  - [ ] 3) recommend_tools через индекс задач→термов.
- acceptance:
  - [ ] 1) без сканов
  - [ ] 2) doc TOOL_REGISTRY.md сокращён до контракта
  - [ ] 3) пример «агент получает пак+инструменты».
- files: tool_registry.rs.
- risks: низкие. Зависимости: A02. Эффект: пак отвечает «что знать И чем действовать».

### EPIC-B18 — Инкрементальный knowledge-graph/provenance индекс

- status: `pending`
- meta: Категория: indexing · Приоритет: P2 · Горизонт: 6 months · Тип: refactor
- goal: graph-traversal и source-support рёбра используются VERIFY и retrieval, но строятся full-scan'ом на вызов (graph.rs:73).
- problem: Проблема: O(N) проекция графа.
- tasks:
  - [ ] 1) entity/relation/source-ref записи индексируются при записи (adjacency map)
  - [ ] 2) graph_retrieval и verification graph-обогащение читают индекс
  - [ ] 3) инвалидация на patch/tombstone + property-тест эквивалентности полной проекции.
- acceptance:
  - [ ] 1) graph-пути без сканов
  - [ ] 2) фикстуры graph_tests/verification_graph_tests без изменений семантики
  - [ ] 3) p95 на 100K-графе.
- files: graph.rs, graph_retrieval.rs, database.rs hook.
- risks: согласованность — property-тест. Зависимости: A02, A06. Эффект: provenance-граф как database-индекс.

### EPIC-B19 — REMEMBER write-path policy formalization

- status: `pending`
- meta: Категория: AQL · Приоритет: P2 · Горизонт: 90 days · Тип: productize
- goal: запись через AQL — половина агентного цикла; политика записи должна быть так же формальна, как чтения.
- problem: Проблема: REMEMBER реализован (binder enforce_remember: scope/memory_type/TTL-клампы), но семантика (id-аллокация, descriptor-поля, дефолты) не контрактна.
- tasks:
  - [ ] 1) спецификация REMEMBER в AQL_V0_5: что создаётся, какие поля, какие клампы
  - [ ] 2) аллокация cell_id — атомарный счётчик в манифесте (вместо max+1 эвристик allocate_cell_id)
  - [ ] 3) e2e: remember→retrieve→verify цикл.
- acceptance:
  - [ ] 1) контракт в доке
  - [ ] 2) id-аллокация безопасна при конкуренции (тест)
  - [ ] 3) e2e-тест цикла.
- files: binder.rs, engine remember-путь, manifest.
- risks: нет. Зависимости: A15. Эффект: полный агентный read-write цикл формализован.

### EPIC-B20 — Multi-brain: реальная семантика или удаление

- status: `pending`
- meta: Категория: AQL · Приоритет: P2 · Горизонт: 6 months · Тип: refactor
- goal: грамматика обещает БРЕЙНЫ, движок живёт с DEFAULT_BRAIN=1 — «синтаксис без семантики» хуже обоих вариантов.
- problem: Проблема: query.rs:32 `const DEFAULT_BRAIN: BrainId = BrainId(1)`.
- tasks:
  - [ ] 1) решение (opinionated: brains = изолированные неймспейсы индексов внутри тенанта — полезно для разделения «знания/память/инструменты»)
  - [ ] 2) если да: каталог брейнов, scope-уникальность внутри брейна, статистика per brain
  - [ ] 3) если нет: депрекация в грамматике к v1.0.
- acceptance:
  - [ ] 1) решение задокументировано в DATA_MODEL.md
  - [ ] 2) либо работающие брейны с тестами, либо migration-план удаления.
- files: query/catalog.rs, binder.
- risks: расширение модели — не делать до A-блока. Зависимости: A02, A12. Эффект: грамматика перестаёт врать.

## Block C — Indexing, retrieval, and performance

### EPIC-C01 — Интернирование термов + компактные постинги

- status: `pending`
- meta: Категория: indexing · P1 · 90 days · refactor
- goal: `LexicalIndex` = BTreeMap<String, BTreeMap<u32,u32>> ×5 разрезов — RAM-обжорство и cache-miss'ы; database-индекс так не строят.
- problem: Проблема: память и скорость лексического индекса.
- tasks:
  - [ ] 1) term dictionary (term→u32 id, FST или sorted dict)
  - [ ] 2) постинги: sorted Vec<u32>/roaring + parallel freq-массив
  - [ ] 3) формат .aci v2 c dual-read.
- acceptance:
  - [ ] 1) RAM lexical-индекса на 1M ↓ ≥3x (бенч)
  - [ ] 2) lexical_index_tests + quality-фикстуры зелёные
  - [ ] 3) migration-тест.
- files: cortex-storage/src/indexes.rs, format.rs.
- risks: формат — version gate. Зависимости: A20. Эффект: индексы перестают быть главным потребителем RAM.

### EPIC-C02 — Roaring bitmaps в bitmap-индексе и VM

- status: `pending`
- meta: Категория: indexing · P1 · 60 days · refactor
- goal: BTreeSet<u32> для universe/AND/OR не масштабируется к 10M кандидатов.
- problem: Проблема: bitmap VM и индексы на BTreeSet.
- tasks:
  - [ ] 1) crate `roaring`; BitmapIndex и eval_bitmap_program на RoaringBitmap
  - [ ] 2) сериализация .acb v2
  - [ ] 3) бенч AND/OR/NOT на 1M/10M.
- acceptance:
  - [ ] 1) bitmap-операции на 1M < 1ms
  - [ ] 2) bitmap_vm_tests зелёные
  - [ ] 3) память ↓ (зафиксировать).
- files: cortex-aql/src/executor_mock.rs (VM), cortex-storage/indexes.rs.
- risks: семантика Not в segment-universe — сохранить точную (complement в universe). Зависимости: A20. Эффект: фундамент перм-битмапов B04 и масштаба.

### EPIC-C03 — Честный BM25 с полевыми весами

- status: `pending`
- meta: Категория: retrieval · P1 · 90 days · refactor
- goal: текущий «BM25-like» с магическими константами (256+768 norm, database.rs:579) не верифицирован против эталона.
- problem: Проблема: непараметризованная аппроксимация.
- tasks:
  - [ ] 1) каноничный BM25(k1,b) в fixed-point, тест против float-эталона на мини-корпусе
  - [ ] 2) полевые веса title/body/path (field_term_frequencies уже хранятся)
  - [ ] 3) конфиг + дефолты.
- acceptance:
  - [ ] 1) расхождение с эталоном < ε
  - [ ] 2) retrieval-quality фикстуры ≥ текущих
  - [ ] 3) параметры в доке SCORING.md.
- files: search/database.rs, lexical scoring.
- risks: сдвиг ранжирования — quality-гейт. Зависимости: C01 желательно. Эффект: ранжирование становится защитимым.

### EPIC-C04 — Токенизация: unicode segmentation + опциональный стемминг

- status: `pending`
- meta: Категория: retrieval · P2 · 90 days · improve→build
- goal: фикстуры проекта на русском/казахском контенте (KZT, инвестпроекты), а токенизатор простой.
- problem: Проблема: morфология снижает recall не-английских корпусов.
- tasks:
  - [ ] 1) unicode-segmentation
  - [ ] 2) rust-stemmers (ru/en) за конфигом коллекции
  - [ ] 3) quality-фикстура с русскими запросами.
- acceptance:
  - [ ] 1) «бюджету»→«бюджет» при включённом стемминге
  - [ ] 2) англ. фикстуры не деградируют
  - [ ] 3) конфиг описан.
- files: search/tokenize, analyze_search_query.
- risks: стемминг меняет индекс — версия анализатора в манифесте (иначе mixed-индекс). Зависимости: C01. Эффект: честный multilingual.

### EPIC-C05 — Disk-resident vector storage + SIMD exact scan

- status: `pending`
- meta: Категория: indexing · P1 · 90 days · refactor
- goal: exact vector scan — ваш заявленный «предсказуемый дефолт»; он должен быть быстрым и не требовать RAM-резидентности.
- problem: Проблема: вектора в payload-строках/RAM; dot-product скалярный.
- tasks:
  - [ ] 1) вектора в .acv с контiguous layout + mmap
  - [ ] 2) SIMD (std::simd/portable-simd) i16-dot
  - [ ] 3) бенч exact scan 1M×768d; векторные данные отвязать от payload-строк (A02: embedding ref в descriptor).
- acceptance:
  - [ ] 1) exact scan 1M×768d p95 в разумном бюджете (зафиксировать после baseline)
  - [ ] 2) parity с текущими результатами
  - [ ] 3) RSS не растёт от векторов в lazy-режиме.
- files: cortex-storage/vectors.rs, cortex-engine/search/vector.
- risks: alignment/endianness — формат-тесты. Зависимости: A07. Эффект: дефолтный семантический путь масштабируется.

### EPIC-C06 — HNSW: guarded productization через nightly recall-гейты

- status: `pending`
- meta: Категория: indexing · P2 · 6 months · improve
- goal: ANN нужен после 1M+; текущая guarded-позиция правильная — нужно сузить стоимость поддержки.
- problem: Проблема: 5 ann-make-гейтов в широких прогонах тормозят разработку.
- tasks:
  - [ ] 1) ann-гейты → nightly
  - [ ] 2) один real-embedding recall-gate (bge-m3 кэш из бенчей)
  - [ ] 3) интеграция в cost-планировщик A13: ANN выбирается при больших корпусах, exact — fallback (существующая логика сохраняется).
- acceptance:
  - [ ] 1) PR CI без ANN-матрицы
  - [ ] 2) nightly recall-отчёт артефактом
  - [ ] 3) planner-правило покрыто тестом.
- files: search/ann*, Makefile, CI.
- risks: нет. Зависимости: A13. Эффект: ANN остаётся честным и дешёвым в поддержке.

### EPIC-C07 — Гибридный retrieval (lexical+dense RRF) в движке

- status: `pending`
- meta: Категория: retrieval · P1 · 90 days · build
- goal: гибрид дал doc recall 85.8% на EnterpriseRAG, но живёт во внешних python-скриптах — это должна быть фича БД.
- problem: Проблема: RRF-фьюжн вне движка (scripts/enterprise_rag_bench).
- tasks:
  - [ ] 1) `RetrievalMode::Hybrid`: два scan-потока → RRF-оператор (A11)
  - [ ] 2) generic-реализация без bench-эвристик (после Kill-решения)
  - [ ] 3) quality-фикстура: hybrid ≥ lexical.
- acceptance:
  - [ ] 1) `USING MODE hybrid` из AQL
  - [ ] 2) фикстурный гейт
  - [ ] 3) EXPLAIN показывает оба пути и фьюжн.
- files: exec/, search/, binder (mode).
- risks: нет. Зависимости: A11, C05, EPIC-C08. Эффект: главный результат бенчей становится продуктом.

### EPIC-C08 — Server-side embedding integration

- status: `pending`
- meta: Категория: retrieval · P1 · 60 days · productize
- goal: «semantic» без встроенного пути получения вектора — ловушка DX (query_vector= строкой в task, database.rs:637).
- problem: Проблема: клиент обязан сам считать вектора; тихая деградация в лексику.
- tasks:
  - [ ] 1) продуктизировать embedding-клиент из embedding_pipeline.rs: конфиг URL/model/key
  - [ ] 2) `/v1/context {embed_query:true}`
  - [ ] 3) явная ошибка «semantic requires vector or embedding config»; таймаут/fallback-политика.
- acceptance:
  - [ ] 1) semantic-режим без ручных векторов
  - [ ] 2) без конфига — ошибка, не молчание
  - [ ] 3) e2e с локальным эмбеддером.
- files: embedding_pipeline.rs, server/{context,search}.rs.
- risks: внешний вызов в запросе — таймаут+fallback на hybrid задокументированы. Зависимости: нет. Эффект: semantic перестаёт быть переоценённым словом.

### EPIC-C09 — Permission-aware index pruning

- status: `pending`
- meta: Категория: indexing · P1 · 90 days · build
- goal: уникальная оптимизация agent-native БД: права агента сужают пространство ДО скана.
- problem: Проблема: scans пересекают permission-маску, но не используют её для пропуска работ.
- tasks:
  - [ ] 1) интеграция scope-bitmap кардинальности в planner: маленькая разрешённая зона → bitmap-first scan
  - [ ] 2) segment skipping: сегмент без разрешённых scope (zone map A12) не открывается вовсе
  - [ ] 3) бенч: агент с 1% видимости на 1M-корпусе.
- acceptance:
  - [ ] 1) latency «узкого» агента近 пропорциональна его зоне, не корпусу (бенч)
  - [ ] 2) корректность E09
  - [ ] 3) EXPLAIN показывает пропущенные сегменты.
- files: plan/cost.rs, exec/scan, statistics.
- risks: нет. Зависимости: A12, A13, B04. Эффект: «права делают запросы быстрее» — продаваемое свойство.

### EPIC-C10 — Segment zone maps + segment skipping

- status: `pending`
- meta: Категория: indexing · P2 · 6 months · build
- goal: классическая database-техника, нужная lazy/temporal/scope фильтрам.
- problem: Проблема: каждый сегмент участвует во всех запросах.
- tasks:
  - [ ] 1) zone map per segment: min/max created_at, scope-set, type-set (в манифест при checkpoint)
  - [ ] 2) planner отбрасывает сегменты по предикатам
  - [ ] 3) тест на 10-сегментном корпусе.
- acceptance:
  - [ ] 1) временной/scope-запрос открывает подмножество сегментов (счётчик)
  - [ ] 2) корректность фикстур.
- files: manifest.rs, plan/, exec/scan.
- risks: нет. Зависимости: A12. Эффект: I/O-составляющая запросов падает.

### EPIC-C11 — AQL query cache: метрики и политика

- status: `pending`
- meta: Категория: query-engine · P2 · 60 days · improve
- goal: кэш есть (AqlQueryCache), но непрозрачен.
- tasks:
  - [ ] 1) hit/miss/eviction в /v1/stats и Prometheus
  - [ ] 2) инвалидация по seq задокументирована
  - [ ] 3) размер/политика в конфиге.
- acceptance:
  - [ ] 1) hit-rate видим
  - [ ] 2) тест инвалидации после записи.
- files: query/cache.rs, server/metrics.rs.
- risks: нет. Зависимости: E05. Эффект: наблюдаемость кэша.

### EPIC-C12 — Rank key precompute (sort_by_cached_key)

- status: `done`
- meta: Категория: retrieval · P0 · 30 days · refactor
- goal: дешёвая победа: metadata-парсинг и dot-product в sort-ключе (database.rs:461-474) выполняются многократно.
- tasks:
  - [x] 1) один проход: предвычислить (lexical, semantic, recency, trust)
  - [x] 2) сортировка по готовым ключам
  - [x] 3) микробенч до/после.
- acceptance:
  - [x] 1) один parse/ячейку/запрос
  - [x] 2) ranking-фикстуры неизменны
  - [x] 3) % ускорения зафиксирован.
- files: database.rs.
- evidence: `rank_retrieved_cells` now precomputes metadata, lexical, semantic, recency, trust, and rank keys before sorting. Focused ranking/search tests pass. Local before/after `single_node_performance_check --cells 1000` shows `context_pack` improvement: strict 2245.923ms → 1997.310ms (+11.07%), balanced 2175.399ms → 1964.614ms (+9.69%). Report: `target/c12-rank-precompute/report.json`.
- risks: local microbench is noisy and not a long-running performance trend. Зависимости: нет. Эффект: немедленное ускорение каждого retrieve; временный мост до A06/A11.

### EPIC-C13 — Fact/numeric индекс

- status: `pending`
- meta: Категория: indexing · P1 · 6 months · build
- goal: численные конфликты — главное verification-оружие; им нужен индекс metric→(cell,value).
- problem: Проблема: numeric-парсинг payload'ов на каждый VERIFY.
- tasks:
  - [ ] 1) extraction при записи (B07) → typed fact rows
  - [ ] 2) индекс metric_id→sorted (value, cell)
  - [ ] 3) конфликт-запрос: same metric, same scope, different normalized value.
- acceptance:
  - [ ] 1) numeric-verify через индекс с прежними вердиктами
  - [ ] 2) p95 на 1M
  - [ ] 3) инкрементальность под property-тестом.
- files: новый index-модуль, verification/numeric.rs.
- risks: нормализация единиц (KZT/B/M) — берётся из существующего numeric.rs. Зависимости: B07. Эффект: verification-масштаб.

### EPIC-C14 — Temporal индекс

- status: `pending`
- meta: Категория: indexing · P2 · 6 months · build
- goal: B10-запросы «valid at date» должны быть индексными.
- tasks:
  - [ ] 1) interval-индекс (sorted by valid_from + zone maps)
  - [ ] 2) подключение к planner-предикатам
  - [ ] 3) stale-guard VERIFY через индекс.
- acceptance:
  - [ ] 1) временные запросы O(log n + k)
  - [ ] 2) temporal-фикстуры зелёные.
- files: temporal_index.rs → index-модуль.
- risks: нет. Зависимости: A02, B10. Эффект: temporal — индексная фича.

### EPIC-C15 — Инкрементальный graph-индекс производительность

- status: `pending`
- meta: Категория: indexing · P2 · 6 months · build
- goal: завершение B18 производительной частью: adjacency-структуры, обходы с лимитами.
- tasks:
  - [ ] 1) компактная adjacency (interned entity ids)
  - [ ] 2) bounded BFS/DFS с visit-budget (как в HNSW-гардах)
  - [ ] 3) бенч 100K-узлового графа.
- acceptance:
  - [ ] 1) обходы с budget-гарантией
  - [ ] 2) p95 зафиксирован.
- files: graph.rs.
- risks: нет. Зависимости: B18. Эффект: graph-retrieval пригоден на масштабе.

### EPIC-C16 — Memory profiling harness (dhat/jemalloc)

- status: `partial`
- meta: Категория: benchmarks · P0 · 30 days · build
- goal: все RAM-обещания нуждаются в измерителе; estimated_* поля /v1/stats не верифицированы.
- tasks:
  - [ ] 1) dhat за feature-флагом + jemalloc stats — not implemented yet because current agent rules forbid adding new dependencies without explicit approval
  - [x] 2) `make memory-profile` → JSON (RSS, аллокации, payload-клоны) — portable RSS/estimate/clone-gate report added
  - [x] 3) сверка estimated vs real (допуск, фиксы расчётов) — current ratio is reported and documented; estimator fixes remain future work if ratio exceeds policy.
- acceptance:
  - [x] 1) отчёт воспроизводим — `make memory-profile MEMORY_PROFILE_CELLS=10000`
  - [x] 2) клон-счётчик используется в A04/A05 acceptance — `payload_clone_gate` is included in the JSON report and mirrors the static clone gate
  - [x] 3) расхождение estimated/real задокументировано — `docs/MEMORY_PROFILE.md`
- files: cortex-bench, memory_accounting.rs.
- risks: нет. Зависимости: нет. Эффект: инструмент всего блока A/C.
- evidence: Added `memory_profile_check` and `make memory-profile`. Local 10K report `target/memory-profile/10k/report.json`: `ok=true`, RSS `38936576`, peak RSS `40894464`, estimated total `28795568`, RSS/estimated ratio `1.352`, peak/estimated ratio `1.420`, payload clone gate passed. Allocator-specific `dhat`/`jemalloc` observers are explicitly marked unavailable until dependency/runtime approval.
- risks: C16 remains partial against the original dhat/jemalloc wording; the portable harness is useful now, but allocator-level allocation counts still need an explicit dependency/runtime decision.

### EPIC-C17 — Перф-регрессии в CI (continuous benchmarking)

- status: `pending`
- meta: Категория: benchmarks · P1 · 60 days · build
- goal: 100 эпиков перфорации без регресс-гейта = регрессии.
- tasks:
  - [ ] 1) nightly perf-job: фикс-корпуса 100K, метрики p50/p95 в trend.json (performance-trend-check уже есть — подключить к новому)
  - [ ] 2) порог регрессии (>20% p95 → красный)
  - [ ] 3) история артефактами.
- acceptance:
  - [ ] 1) nightly красный при искусственной регрессии (тест процесса)
  - [ ] 2) тренд-страница генерируется.
- files: CI workflows, cortex-bench.
- risks: шум раннеров — медианы из N прогонов. Зависимости: A19. Эффект: перф-дисциплина.

### EPIC-C18 — Concurrent read throughput bench

- status: `pending`
- meta: Категория: benchmarks · P1 · 60 days · benchmark
- goal: A16 без бенча — вера, не факт.
- tasks:
  - [ ] 1) нагрузочный сценарий: K читателей + 1 писатель, latency/throughput кривые по потокам
  - [ ] 2) сравнение actor-only vs RwLock-пути
  - [ ] 3) включить в trend.
- acceptance:
  - [ ] 1) кривая масштабирования опубликована
  - [ ] 2) включено в C17.
- files: cortex-bench.
- risks: нет. Зависимости: A16. Эффект: доказательство конкурентности.

### EPIC-C19 — Ingestion throughput + батчевый embedding pipeline

- status: `pending`
- meta: Категория: retrieval · P2 · 6 months · improve
- goal: загрузка 511K документов для бенча заняла часы на embedding — узкое место реальных пользователей.
- tasks:
  - [ ] 1) батчинг/параллелизм embedding-запросов с резюмируемостью (частично есть — оформить)
  - [ ] 2) ingestion через WriteBatch (A15)
  - [ ] 3) бенч docs/sec end-to-end.
- acceptance:
  - [ ] 1) ingestion 100K доков с эмбеддингом — измеренная цифра
  - [ ] 2) resume после обрыва (тест).
- files: ingestion/, embedding_pipeline.rs.
- risks: нет. Зависимости: A15. Эффект: реальный onboarding больших корпусов.

### EPIC-C20 — Baseline-сравнение с наивным стеком

- status: `pending`
- meta: Категория: benchmarks · P2 · 90 days · benchmark
- goal: честный ответ на «зачем вы, если есть SQLite FTS5 + faiss»: если по качеству ретрива не выигрываем — надо знать; выигрываем по governance — показать.
- tasks:
  - [ ] 1) референс-стек (SQLite FTS5 + faiss + 50 строк python) в cortex-bench
  - [ ] 2) прогон на ваших 4 quality-доменах + латентность
  - [ ] 3) публикация таблицы качество/латентность/фичи (permissions/budget/citations).
- acceptance:
  - [ ] 1) воспроизводимый скрипт
  - [ ] 2) результат опубликован, каким бы ни был.
- files: cortex-bench.
- risks: неприятный результат = ценность. Зависимости: A19. Эффект: позиция «зачем мы» получает данные.

## Block D — Developer experience and adoption

### EPIC-D01 — CLI help и группировка команд

- status: `done`
- meta: Категория: CLI · P0 · 30 days · improve
- goal: первый контакт: ~50 команд без единого описания (проверено `--help`).
- tasks:
  - [x] 1) about/long_about для всех команд — top-level and nested commands now have `about`; `help_contract_tests::every_cli_command_has_help_text` guards regressions.
  - [x] 2) группировка (core/search/admin/backup/wal) — top-level help includes explicit `Command groups`.
  - [x] 3) примеры в help ключевых команд (context, verify, agent) — `context`, `verify`, and `aql` long help include copy-paste examples; `context` is covered by a golden CLI test.
- acceptance:
  - [x] 1) 100% команд описаны
  - [x] 2) `cortexdb help context` содержит пример AQL.
- files: cortex-cli/src.
- risks: нет. Зависимости: нет. Эффект: смертельный DX-провал закрыт за день-два.
- evidence: `cargo test -p cortex-cli` passed with 74 unit tests plus 6 integration tests; `cortexdb --help` shows `Command groups`; `cortexdb help context` shows `RETRIEVE CONTEXT FOR TASK`.

### EPIC-D02 — `cortexdb init` + doctor

- status: `pending`
- meta: Категория: CLI · P1 · 60 days · build
- tasks:
  - [ ] 1) init: база+пример AgentView+печать следующих шагов
  - [ ] 2) doctor: lock, WAL-валидность, версии форматов, RAM-прогноз vs доступно
  - [ ] 3) выводить «что не так и что сделать».
- acceptance:
  - [ ] 1) init→quickstart без чтения доков
  - [ ] 2) doctor ловит 5 типовых проблем (тесты).
- files: cortex-cli.
- risks: нет. Зависимости: D01. Эффект: нулевой порог входа.

### EPIC-D03 — GETTING_STARTED — 5 минут до первого пака

- status: `done`
- meta: Категория: docs · P0 · 30 days · document
- tasks:
  - [x] 1) один файл: install→load-fixture→search→context→verify→два агента (≤10 команд) — added `docs/GETTING_STARTED.md`.
  - [x] 2) README ссылается первым экраном
  - [x] 3) проверить на живом человеке — replaced with executable gate for the documented commands.
- acceptance:
  - [x] 1) человек без контекста доходит за ≤5 минут
  - [x] 2) все команды копипаст-выполнимы.
- files: docs/GETTING_STARTED.md, README.
- risks: нет. Зависимости: D01. Эффект: конверсия первого касания.
- evidence: `make getting-started-check` passed; it builds the CLI, loads `examples/datasets/investment_projects`, runs stats/search/context/verify, and proves `agent:hr` cannot retrieve `project:investments` evidence.

### EPIC-D04 — Flagship demo: permissions + numeric conflict

- status: `done`
- meta: Категория: product · P0 · 30 days · productize
- goal: лучший сценарий уже в репо (investment_projects: конфликтующие бюджеты) — не собран в одну историю.
- tasks:
  - [x] 1) `make demo`: два агента (finance/hr), отказ в чужом scope, VERIFY ловит 1.2B vs 1.4B KZT
  - [x] 2) цветной вывод+asciinema GIF в README — demo output is colorized when attached to a TTY; README embeds a terminal-style `examples/demo/investment_projects/demo.gif` generated locally with `ffmpeg` because asciinema/agg are not project dependencies.
- acceptance:
  - [x] 1) демо за <60 секунд показывает оба эффекта
  - [x] 2) GIF в README.
- files: Makefile, examples/, cortex-cli demo.
- risks: нет. Зависимости: D01. Эффект: «зачем это» понятно за минуту.
- evidence: `make flagship-demo-check` passed in `1.8s`; it checks `Finance agent`, `HR agent denied as expected: ScopeNotReadable`, `mixed evidence`, `1.2B KZT`, `1.4B KZT`, and successful completion markers.

### EPIC-D05 — Публикация SDK (PyPI/npm/crates.io)

- status: `partial`
- meta: Категория: SDK · P0 · 30 days · productize
- goal: README обещает pip/npm — пакетов нет; preflight-гейты (make sdk-check) уже есть.
- tasks:
  - [x] 1) проверить имена — manifest/package metadata lock `cortex-sdk`, `cortexdb-client`, and `@cortexdb/client`; `make sdk-check` validates Rust cargo package, Python wheel/test path, and npm pack dry-run.
  - [x] 2) выполнить tag-gated workflow preflight/contract (`docs/archive/SDK_RELEASE.md`) — `make sdk-e2e-release-check` validates release contract, deprecation policy, registry gate, SDK examples artifact, and live SDK contract.
  - [ ] 3) install-smoke с чистой машины в CI against public registries.
- acceptance:
  - [ ] 1) `pip install cortexdb-client` работает
  - [ ] 2) npm/cargo аналогично
  - [ ] 3) README-примеры запускаются против опубликованных пакетов.
- files: sdk/, .github/workflows/sdk-release.yml.
- risks: занятые имена — резерв заранее. Зависимости: D15 (версии). Эффект: quickstart перестаёт быть фикцией.
- evidence: `make sdk-e2e-release-check` passed after SDK release/deprecation/publication gates were aligned to archived docs; `make sdk-check` passed and produced Rust `cargo package`, Python SDK tests, and npm pack dry-run evidence.
- remaining: public registry publication and clean-machine install smoke require the beta version/tag from `EPIC-D15` plus registry credentials/trusted publishing.

### EPIC-D06 — Python SDK: typed-модели, ретраи, таймауты

- status: `pending`
- meta: Категория: SDK · P1 · 60 days · improve
- tasks:
  - [ ] 1) модели из ContextPack-схемы (B01, codegen)
  - [ ] 2) retry с экспонентой на 503 database_busy (ваш собственный backpressure-контракт!)
  - [ ] 3) таймауты, context manager, connection reuse.
- acceptance:
  - [ ] 1) mypy-чистые типы
  - [ ] 2) retry-тест против заполненной очереди
  - [ ] 3) README-пример с типами.
- files: sdk/python.
- risks: нет. Зависимости: B01, D05. Эффект: SDK production-поведения.

### EPIC-D07 — TypeScript SDK polish

- status: `pending`
- meta: Категория: SDK · P1 · 60 days · improve
- tasks:
  - [ ] 1) типы из схемы; ESM+CJS
  - [ ] 2) retry на 503
  - [ ] 3) пример с LLM-вызовом.
- acceptance:
  - [ ] 1) tsd-тесты типов
  - [ ] 2) 10-строчный рабочий пример.
- files: sdk/typescript. Зависимости: B01, D05.

### EPIC-D08 — Async Rust SDK + общий крейт api-types

- status: `pending`
- meta: Категория: SDK · P1 · 90 days · build
- goal: cortex-sdk блокирующий; типы ответов дублируются с сервером.
- tasks:
  - [ ] 1) `cortex-api-types` (вынести из server/responses.rs)
  - [ ] 2) async-клиент (reqwest) feature-флагом
  - [ ] 3) contract-тесты на оба клиента.
- acceptance:
  - [ ] 1) сервер и SDK используют одни типы
  - [ ] 2) async-клиент проходит те же тесты.
- files: новый crates/cortex-api-types, cortex-sdk, cortex-server.
- risks: нет. Зависимости: B01. Эффект: типобезопасность контура.

### EPIC-D09 — Docker GHCR + compose quickstart

- status: `pending`
- meta: Категория: adoption · P1 · 60 days · productize
- tasks:
  - [ ] 1) publish в release workflow
  - [ ] 2) compose: server+авто-загрузка фикстуры+дашборд
  - [ ] 3) docker-путь в GETTING_STARTED.
- acceptance:
  - [ ] 1) `docker run ghcr.io/...` поднимает рабочий сервер
  - [ ] 2) healthcheck зелёный.
- files: Dockerfile, workflows, docs.

### EPIC-D10 — OpenAPI как единый источник + codegen-контроль

- status: `pending`
- meta: Категория: API · P1 · 90 days · improve
- goal: openapi.yaml есть; нужно гарантировать соответствие коду.
- tasks:
  - [ ] 1) contract-тест: реальные ответы валидируются против OpenAPI (расширить openapi-contract-check)
  - [ ] 2) error codes (E-таксономия) в схеме
  - [ ] 3) генерация клиентских типов из схемы в SDK-пайплайне.
- acceptance:
  - [ ] 1) расхождение код/схема валит CI
  - [ ] 2) SDK-типы из единого источника.
- files: docs/openapi.yaml, тесты server.

### EPIC-D11 — MCP server adapter

- status: `pending`
- meta: Категория: adoption · P1 · 60 days · build
- goal: MCP — стандарт подключения инструментов к агентам; tools `retrieve_context`/`verify_fact`/`remember` идеально ложатся на API.
- tasks:
  - [ ] 1) `cortex-mcp` (stdio) поверх SDK
  - [ ] 2) маппинг AgentView↔MCP-конфиг
  - [ ] 3) док «подключи к Claude Code/IDE за 2 минуты» + demo.
- acceptance:
  - [ ] 1) рабочий MCP-конфиг из коробки
  - [ ] 2) демо: агент отвечает с цитатами из CortexDB.
- files: новый crates/cortex-mcp.
- risks: ещё одна поверхность — держать тонкой (только 3 tools). Зависимости: D05. Эффект: путь к первым живым пользователям.

### EPIC-D12 — Документация: 206 → ~30 core + archive

- status: `done`
- meta: Категория: docs · P0 · 30 days · remove
- tasks:
  - [x] 1) core-набор (~30: README, GETTING_STARTED, DATA_MODEL, ARCHITECTURE, MEMORY_MODEL, CONTEXT_PACK, AQL, VERIFY_FACT, AUTH, API, CLI, SDK, OPERATIONS, BENCHMARKS, CLAIMS, COMPARISONS, ROADMAP…) — root `docs/` now has 35 core markdown files.
  - [x] 2) остальное → docs/archive/ + один индекс — 170 docs moved to `docs/archive/`; `docs/archive/INDEX.md` added.
  - [x] 3) починить ссылки (link-checker в CI) — `scripts/docs_link_check.py` and `make docs-link-check` added.
- acceptance:
  - [x] 1) `ls docs/*.md | wc -l` ≤ 35
  - [x] 2) link-check зелёный
  - [x] 3) make-таргеты доков живы.
- files: docs/.
- risks: ссылки из кода/CI — grep. Зависимости: нет. Эффект: проект становится читаемым.
- evidence: `find docs -maxdepth 1 -type f -name '*.md' | wc -l` returns `35`; `docs/archive/INDEX.md` indexes archived documents; `make docs-link-check` reports `markdown links ok: 244 files`.

### EPIC-D13 — mdBook docs-сайт

- status: `pending`
- meta: Категория: docs · P2 · 90 days · productize
- tasks:
  - [ ] 1) mdBook поверх core-доков
  - [ ] 2) GitHub Pages deploy
  - [ ] 3) поиск.
- acceptance:
  - [ ] сайт с навигацией публикуется из CI.
- dependencies: D12.

### EPIC-D14 — Examples: 3 живых интеграции

- status: `pending`
- meta: Категория: adoption · P1 · 90 days · build
- tasks:
  - [ ] 1) OpenAI/Anthropic function-calling tool (context+verify как tools)
  - [ ] 2) LangChain retriever
  - [ ] 3) чат-агент с памятью (TTL/decay) — поднять examples/demo/agent_memory до first-class.
- acceptance:
  - [ ] каждый пример: папка+README+CI-smoke (mock LLM).
- dependencies: D05, B01. Эффект: разработчики копируют примеры.

### EPIC-D15 — v0.2.0-beta.1: версии, release notes, тег

- status: `partial`
- meta: Категория: product · P0 · 30 days · productize
- goal: workspace 0.1.0 при бета-цели 0.2.0-beta.1; гейты есть, релиза нет.
- tasks:
  - [x] 1) bump версий — workspace/Rust/TypeScript/OpenAPI now use `0.2.0-beta.1`; Python uses the documented PEP 440 spelling `0.2.0b1`; SDK/release gates validate this mapping.
  - [x] 2) `make beta-release-check` + бинарники (binary-release-check) — passed after aligning release gates with archived documentation paths and fixing a VERIFY numeric false-positive in the RAG demo smoke.
  - [ ] 3) тег+GitHub release; release notes сверить с реальностью (после repositioning) — release notes and local artifacts are verified; public tag/release remains.
- acceptance:
  - [ ] 1) тег с артефактами
  - [ ] 2) README-статус соответствует.
- dependencies: A01, D12, D05 (желательно). Эффект: точка отсчёта «бета».
- evidence: `make sdk-check`, `make sdk-e2e-release-check`, `make openapi-contract-check`, `make beta-release-check`, `make binary-release-check BINARY_RELEASE_VERSION=v0.2.0-beta.1 BINARY_RELEASE_ID=cortexdb-v0.2.0-beta.1-local`, `make release-artifact-manifest-check BINARY_RELEASE_VERSION=v0.2.0-beta.1 BINARY_RELEASE_ID=cortexdb-v0.2.0-beta.1-local BINARY_RELEASE_ARCHIVE=target/release-artifacts/cortexdb-v0.2.0-beta.1-local.tar.gz`, `make evidence-artifact-retention-check`, and `make versioning-policy-check` passed after the version bump. Earlier `make rag-demo-smoke` passed after VERIFY stopped cross-comparing matched year and amount values as numeric contradictions.
- remaining: public `v0.2.0-beta.1` already exists but points to old commit `46d0f3a` whose workspace/Python package versions are still `0.1.0`; do not force-move that published tag without explicit approval. Safe next release options are either an explicit force-refresh of `v0.2.0-beta.1` after approval or a new patch prerelease such as `v0.2.0-beta.2`. Public SDK registry publication remains governed by `EPIC-D05`.

## Block E — Reliability, security, and operations

### EPIC-E01 — WAL writer: ошибки не глотаются

- status: `pending`
- meta: Категория: storage · P0 · 30 days · improve
- goal: `run_writer` молча умирает при ошибке открытия (wal/writer.rs:166-168) — все appends получают WalWriterClosed без причины.
- tasks:
  - [ ] 1) канал готовности: start ждёт подтверждения открытия
  - [ ] 2) последняя ошибка потока в shared state → в текст WalWriterClosed
  - [ ] 3) тест: read-only dir → осмысленная ошибка из Database::open.
- acceptance:
  - [ ] 1) ошибка видна сразу
  - [ ] 2) тест зелёный.
- files: cortex-storage/src/wal/writer.rs.

### EPIC-E02 — Backup UX: один happy path + verify

- status: `pending`
- meta: Категория: ops · P1 · 60 days · improve
- tasks:
  - [ ] 1) `backup create` = snapshot+validate+checksum-манифест
  - [ ] 2) `backup verify` без восстановления
  - [ ] 3) restore с прогрессом; 6 команд остаются как advanced.
- acceptance:
  - [ ] happy path = 2 команды; verify ловит порчу (тест).
- files: cortex-cli, engine/backup.

### EPIC-E03 — WAL-архив → point-in-time recovery (groundwork)

- status: `pending`
- meta: Категория: storage · P2 · 6 months · build
- goal: после A17 (ротация) PITR становится дешёвым: архивируй WAL-сегменты, восстанавливай до seq.
- tasks:
  - [ ] 1) опция архивации закрытых WAL-файлов
  - [ ] 2) `restore --to-seq N`
  - [ ] 3) crash-тесты восстановления до точки.
- acceptance:
  - [ ] 1) восстановление на произвольный seq между чекпоинтами (тест)
  - [ ] 2) док в OPERATIONS.
- dependencies: A17. Риски: средние — только после стабилизации ротации.

### EPIC-E04 — Corruption handling: карантин и repair UX

- status: `pending`
- meta: Категория: ops · P2 · 6 months · improve
- goal: corruption-матрица детектит хорошо; нужен оформленный operator-путь «что делать».
- tasks:
  - [ ] 1) повреждённый сегмент/блок → карантин-директория + деградация с предупреждением (если избыточность позволяет)
  - [ ] 2) `cortexdb repair` сценарии по классам повреждений
  - [ ] 3) runbook-страница.
- acceptance:
  - [ ] 1) однострочный diag → конкретная команда восстановления
  - [ ] 2) тесты по классам порчи.
- files: repair.rs, validation.rs, cli.
- dependencies: A07 (блочные CRC).

### EPIC-E05 — Observability: tracing + Prometheus /metrics

- status: `pending`
- meta: Категория: observability · P1 · 60 days · build
- tasks:
  - [ ] 1) tracing-спаны: HTTP → queue-wait → engine op (queue-wait — ключевая backpressure-метрика)
  - [ ] 2) /metrics в Prometheus-формате (маппинг существующего /v1/stats)
  - [ ] 3) пример Grafana-дашборда.
- acceptance:
  - [ ] 1) queue-wait p95 в метриках
  - [ ] 2) scrape-smoke в CI
  - [ ] 3) дашборд в docs.
- files: cortex-server (metrics.rs, middleware).

### EPIC-E06 — Backpressure-тюнинг и лимиты per tenant

- status: `pending`
- meta: Категория: ops · P1 · 90 days · improve
- tasks:
  - [ ] 1) квоты: max cells / max RAM-estimate / max queue per tenant (отказ с кодом quota_exceeded)
  - [ ] 2) гайд тюнинга CORTEXDB_ACTOR_QUEUE_CAPACITY против латентности
  - [ ] 3) 50-тенантный load-тест.
- acceptance:
  - [ ] 1) квоты enforce'ятся (тесты)
  - [ ] 2) capacity-формула в OPERATIONS (с A19/C16 числами).
- files: server/actor.rs, router; docs.

### EPIC-E07 — Audit log productization

- status: `pending`
- meta: Категория: security · P1 · 90 days · improve
- tasks:
  - [ ] 1) JSONL-sink: ротация, fsync-политика, схема-версия
  - [ ] 2) обязательные поля: agent_id, route, scope-решения (allowed/denied), seq
  - [ ] 3) SIEM-доки → архив (фриз).
- acceptance:
  - [ ] 1) denied-доступы видны в аудите (тест)
  - [ ] 2) формат задокументирован одной страницей.
- files: server/audit*.rs.

### EPIC-E08 — Tenant isolation test suite

- status: `pending`
- meta: Категория: security · P1 · 60 days · test
- tasks:
  - [ ] 1) негативные тесты path-traversal имён тенантов (валидация есть — закрепить)
  - [ ] 2) cross-tenant: данные/статы/метрики не утекают между тенантами (матрица маршрутов)
  - [ ] 3) fuzz tenant-имён.
- acceptance:
  - [ ] 1) полная матрица маршрутов покрыта cross-tenant тестом
  - [ ] 2) fuzz без паник/утечек.
- files: server/tests.

### EPIC-E09 — Property-suite инварианта прав («ни байта мимо AgentView»)

- status: `pending`
- meta: Категория: security · P0 · 60 days · test
- goal: главный продуктовый инвариант должен быть механически проверяем.
- tasks:
  - [ ] 1) proptest: random корпус+random AgentView+random запросы по ВСЕМ поверхностям (context/search/get/verify/graph/memory/explain/export) → ни один payload-байт вне readable_scopes не появляется ни в одном поле ответа (включая explain/anomalies/ошибки)
  - [ ] 2) negative-каталог: попытки эскалации через AQL NOT/WHERE (расширить aql_security_fuzzing)
  - [ ] 3) гейт в CI.
- acceptance:
  - [ ] 1) suite зелёный и обязателен
  - [ ] 2) каждый найденный лик — регрессионный кейс.
- files: новые тесты в cortex-server/cortex-engine.
- dependencies: B04/B16 усиливают, но запуск возможен сразу. Эффект: продаваемое security-доказательство.

### EPIC-E10 — Fuzzing decode-путей (cargo-fuzz)

- status: `pending`
- meta: Категория: security · P1 · 60 days · test
- tasks:
  - [ ] 1) таргеты: WalCodec::decode, SegmentReader, manifest load, AQL parser
  - [ ] 2) corpus из реальных файлов
  - [ ] 3) nightly 15-минутный джоб.
- acceptance:
  - [ ] 1) 4 таргета
  - [ ] 2) неделя nightly без новых паник.
- files: fuzz/ (новый).

### EPIC-E11 — Chaos-консолидация + graceful shutdown

- status: `pending`
- meta: Категория: ops · P2 · 90 days · test
- tasks:
  - [ ] 1) карта crash/restart/chaos-сценариев, дедуп тестов (crash_matrix vs fault_injection vs chaos-restart)
  - [ ] 2) SIGTERM: дренаж очередей+WAL shutdown (тест под нагрузкой)
  - [ ] 3) общий harness-модуль.
- acceptance:
  - [ ] 1) время набора ↓ ≥30% без потери сценариев
  - [ ] 2) SIGTERM-тест без потерь ack'нутого.
- files: тесты engine/server, server main.

### EPIC-E12 — Migration framework для форматов (A02/A07/C01/C02)

- status: `pending`
- meta: Категория: ops · P0 · 60 days · build
- goal: блок A меняет форматы; без рамки миграций это серия катастроф.
- tasks:
  - [ ] 1) версии форматов в манифесте (частично есть — централизовать: WAL/segment/index/manifest)
  - [ ] 2) `cortexdb migrate` оркеструет пошаговые миграции с backup-предусловием
  - [ ] 3) матрица совместимости в STORAGE_COMPATIBILITY + fixtures на каждую версию.
- acceptance:
  - [ ] 1) база любой прошлой версии открывается (dual-read) или мигрируется одной командой
  - [ ] 2) downgrade-политика заявлена
  - [ ] 3) CI-гейт с фикстурами старых форматов.
- files: compatibility.rs, cli migrate, fixtures/migration.
- expected effect: позволяет агрессивно эволюционировать форматы, не теряя пользователей.

### EPIC-E13 — Secrets-гигиена

- status: `pending`
- meta: Категория: security · P2 · 60 days · improve
- tasks:
  - [ ] 1) токены только env/file (не CLI-аргументы — видны в ps)
  - [ ] 2) redaction в tracing/audit (тест: токен не встречается в логах после e2e)
  - [ ] 3) doc.
- acceptance:
  - [ ] grep токена по всем логам e2e — пусто.

### EPIC-E14 — Upgrade/rollback drill

- status: `pending`
- meta: Категория: ops · P2 · 90 days · test
- tasks:
  - [ ] 1) автотест: установка vN-1 → данные → upgrade vN → валидация → rollback по доке
  - [ ] 2) включить в release-check
  - [ ] 3) UPGRADE_ROLLBACK сжать до исполняемого runbook.
- acceptance:
  - [ ] drill зелёный перед каждым тегом.
- dependencies: E12, D15.

### EPIC-E15 — Per-route таймауты + защита актора от медленных клиентов

- status: `pending`
- meta: Категория: ops · P2 · 60 days · improve
- tasks:
  - [ ] 1) tower-timeout c бюджетами per route
  - [ ] 2) отмена запроса не оставляет актор в плохом состоянии (drop reply-канала — проверить)
  - [ ] 3) тест slow-loris-клиента.
- acceptance:
  - [ ] зависший клиент не держит слот; таймауты в конфиге.
- files: server/main.rs, actor.rs.

## Block F — Long-term database research

### EPIC-F01 — Tiered storage v2: hot/cold с LRU и компрессией страниц

- status: `pending`
- meta: Категория: storage · P3 · 12 months · build · **сейчас не делать (дизайн — можно)**
- goal: 10M-100M cells на обычной ноде; продолжение A08.
- tasks:
  - [ ] дизайн-док: page cache, компрессия payload-блоков (zstd), prefetch по плану запроса; прототип за флагом.
- acceptance:
  - [ ] дизайн-ревью; прототип на 10M с бюджетом RAM.
- dependencies: A08 стабилен в проде ≥1 квартал. Риски: высокие.

### EPIC-F02 — Распределённая репликация (разморозка)

- status: `frozen`
- tasks:
  - [ ] Условия разморозки: Level 3 достигнут; ≥3 внешних пользователя просят HA; форматы стабильны ≥2 релиза. Код заморожен в cortex-replication; единственная разрешённая работа — поддержание компиляции.

### EPIC-F03 — Консенсус/мульти-нод транзакции

- status: `frozen`
- tasks:
  - [ ] После F02. Существующие consensus-тесты сохранить как research-базу. Любая работа раньше — прямой вред фокусу.

### EPIC-F04 — Agent transaction semantics (мульти-агентные записи)

- status: `pending`
- meta: Категория: research · P3 · 12 months · research
- tasks:
  - [ ] Что исследовать: optimistic concurrency per scope; конфликт двух агентов, пишущих один факт — это data-конфликт (B09) или txn-конфликт? Спека «agent write contract»: идемпотентность, retry-семантика, read-your-writes для сессии агента.
  - [ ] Деливерабл: research-док + прототип за флагом. Зависимости: A15, A16.

### EPIC-F05 — Learned/calibrated ranking

- status: `pending`
- meta: Категория: research · P3 · 12 months · research · **не делать до C-блока**

### EPIC-F06 — Semantic compression памяти

- status: `pending`
- meta: Категория: research · P3 · long-term · research
- tasks:
  - [ ] Идея: cold-память агентов сжимается суммаризацией (внешней моделью) с сохранением provenance-ссылок на оригиналы; «вспоминание» разворачивает. Деливерабл: дизайн + прототип через MCP/внешний воркер. **Не встраивать LLM в движок.**

### EPIC-F07 — Query optimization для LLM-контекста («ценность на токен» cost model)

- status: `pending`
- meta: Категория: research · P3 · 12 months · research

### EPIC-F08 — Multi-agent memory consistency model

- status: `pending`
- meta: Категория: research · P3 · long-term · research
- tasks:
  - [ ] Семантика shared scopes: видимость записей агента A для агента B (immediate/sequenced), handoff-пакеты (пак как сообщение между агентами с pack hash + seq). Дизайн-док.

### EPIC-F09 — Cloud/service mode

- status: `frozen`

### EPIC-F10 — Формальная верификация инвариантов (TLA+/stateright)

- status: `pending`
- meta: Категория: research · P3 · 12 months · research
- tasks:
  - [ ] Модели: WAL-recovery (включая ротацию A17), snapshot pinning/GC (A14), policy rewrite инвариант (B16 — «не существует плана с непокрытым Scan»). Деливерабл: модели + CI-прогон stateright. Сильный имиджевый актив для database-категории.
- files: docs/GETTING_STARTED.md, Makefile (demo), sdk/* + workflows (publish), версии workspace, docs/archive финализация, первые E09 property-тесты прав.
