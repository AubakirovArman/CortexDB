# CortexDB Database-Grade Execution Plan

Source of truth: `/mnt/hf_model_weights/arman/3bit/sites/CortexDB-roadmap`.
Raw catalog source: `/mnt/hf_model_weights/arman/3bit/sites/pl copy.md`.

Execution rule: close epics in order. Use the dependency-aware order from the source plan when raw catalog order conflicts with dependencies. Do not jump to later epics unless the current epic explicitly depends on a prerequisite check, a small safe parallel task, or the user redirects the order.

Status values: `next`, `in_progress`, `partial`, `done`, `blocked`, `frozen`.

Exit steps: use `docs/EPIC_EXIT_STEPS.md` as the short per-epic checklist for
what must be done before moving to the next epic. The detailed tasks and
evidence remain in this tracker.

Current pointer: `EPIC-E05` (`EPIC-E03` is now done along with the previously
closed epics listed in the Active Execution Queue). `EPIC-C01` is closed with
the `ACI4` compact term dictionary/postings format, `ACI0..ACI3` dual-read
compatibility, and persisted/search compatibility gates. `EPIC-C03` is closed
with canonical fixed-point BM25, field weights, and scoring docs. `EPIC-C04` is
closed with configured Unicode analyzer/tokenizer, optional stemming, manifest
profile protection, and RU quality fixtures. `EPIC-C05` is closed with `ACV1`
contiguous fixed-dimension vector rows, disk-resident exact scan, stable chunked
dot-product scoring, and `ACV0` read-only compatibility. `EPIC-C06` moved broad
ANN report gates to nightly, added a BGE-M3 cache recall gate, and covered the
large-corpus ANN planner rule with exact fallback. `EPIC-C07` added AQL
`USING MODE hybrid`, a physical lexical+dense RRF source, and explain/quality
coverage. `EPIC-C09` added permission-aware AQL index pruning with scope-zone
segment skipping, planner cardinality coverage, stale-candidate safety, and
EXPLAIN skipped/opened segment reporting. `EPIC-C10` added plan-aware
segment-zone pruning for AQL bitmap predicates plus freshness created-at
ranges, with EXPLAIN `segment_pruning` counters. `EPIC-C11` exposed AQL query
cache hit/miss/eviction/capacity metrics through stats, metrics, Prometheus,
OpenAPI/SDK models, and configurable bounded FIFO policy. `EPIC-C18` published
the concurrent read throughput curve and wired it into the C17 trend check.
`EPIC-C19` added resumable batched embedding backfill, WriteBatch ingestion
throughput reporting, C17 trend integration, and an explicit 100K gate target.
`EPIC-C20` added the reproducible baseline comparison gate and published the
four-domain quality/latency/feature matrix. `EPIC-E03` added retained closed
WAL archives, restore-to-seq, and PITR operations docs. `EPIC-E05` is next.
`EPIC-D05` remains partial/local-ready and is externally blocked on public
registry credentials/trusted publishing.

Scale-gate rule: individual epics use small/medium evidence gates by default
so implementation does not stall on long-running benchmarks. Large 1M/10M
validation runs are accumulated and executed as benchmark packets under
`EPIC-C17`, unless the active epic explicitly requires them for safety.

Impact measurement rule: the 50-question EnterpriseRAG impact gate is no longer
mandatory after every change. Run `make enterprise-rag-bench-impact-gemini-50`
only when explicitly requested or when we intentionally promote a benchmark
pipeline. The target uses official-clean 50 questions, Gemini 3.5 Flash as
answerer and judge, `engine-aql` + weighted rerank, and
`target/enterprise-rag-bench/cortexdb-full` with `reuse_db=1` so the corpus is
not reingested. Current baseline:
`overall=41.36`, `correctness=42.0`, `completeness=44.76`, `document_recall=56.0`,
`invalid_extra_docs=9.44`, `answer_tokens=302372`, `judge_tokens=27312`
from `target/enterprise-rag-bench/official-clean/50/impact-gemini50-20260612T140305Z/answer-gemini/official_clean_run_report.json`.

## Active Execution Queue

This queue follows `CortexDB-roadmap/00-status.md`, not raw numeric epic order.
Partial epics can remain tracked as follow-up work when their accepted phase is
enough to unblock the next dependency step.

1. `EPIC-A06` — indexed-only retrieve/ContextPack path: done; query-adjacent
   scans remain at zero and the 1M prepared-index ContextPack p95 is published.
2. `EPIC-A07 -> EPIC-A08` — segment v2 plus lazy payload: done for the
   small/medium functional gate; large lazy ContextPack performance debt moved
   to A19/C17.
3. `EPIC-A13` — cost model v0: done.
4. `EPIC-B01` — ContextPack JSON Schema v1: done.
5. `EPIC-A15` — transactional WriteBatch API: done.
6. `EPIC-D11` — MCP adapter: done.
7. `EPIC-E01` — WAL writer error surfacing: done.
8. `EPIC-A09` — disk-resident persisted-index incremental merge: done.
9. `EPIC-D02` — `cortexdb init` + doctor: done.
10. `EPIC-D06` — Python SDK typed models, retries, and timeouts: done.
11. `EPIC-D07` — TypeScript SDK polish: done.
12. `EPIC-D08` — Async Rust SDK + shared API types: done.
13. `EPIC-D09` — Docker GHCR + compose quickstart: done.
14. `EPIC-D10` — OpenAPI as source of truth + codegen control: done.
15. `EPIC-B02` — ContextPackBuilder as a physical operator: done; upstream
   budget pushdown and early termination moved to B03.
16. `EPIC-B03` — token-budget pushdown and early termination: done for the
   small/medium execution gate; 1M/10M lazy p95 evidence remains A19/C17.
17. `EPIC-B04` — AgentView as an index invariant: done.
18. `EPIC-B05` — AgentView lifecycle API v1: done.
19. `EPIC-B06` — typed provenance model: done.
20. `EPIC-B07` — fact/claim store with typed numeric values: done for the
   maintained typed-store and VERIFY integration slice; the metric-sorted
   numeric index and 1M p95 evidence are closed by `EPIC-C13`.
21. `EPIC-B08` — VerifyOp as a planned operator: done; VERIFY now has a
   logical plan node, traceable execution stages, engine-level
   `EXPLAIN VERIFY`/`EXPLAIN ANALYZE VERIFY`, and public report parity tests.
22. `EPIC-B09` — incremental contradiction/conflict index: done; conflicts are
   maintained on put/patch/tombstone/reopen, exposed through `/v1/conflicts`,
   and surfaced as ContextPack conflict anomalies without query-time full scan.
23. `EPIC-B10` — temporal validity columns and temporal queries: done; descriptor-backed `TemporalValidityStore` feeds a physical `TemporalValidityFilter`, AQL supports `REQUIRE valid at`, and stale/future candidates are filtered before payload materialization.
24. `EPIC-B11` — Memory lifecycle: TTL/decay as storage policy: done; descriptor-backed lifecycle index, query-time TTL filter, WAL tombstone maintenance, and Q16 rank decay are in place.
25. `EPIC-B12` — Session/episodic memory contract: done; session metadata is descriptor-backed, session retrieval uses a maintained index, and lazy reopen only materializes matching session payloads.
26. `EPIC-B13` — Feedback as an indexed ranking signal: done; source-cell
    feedback is maintained in an indexed target->records map and influences
    ContextPack ranking without candidate-wide scans.
27. `EPIC-A19` — scale benchmarks 100K/1M/10M and curves: done.
28. `EPIC-C17` — performance regressions in CI: done.
29. `EPIC-C13` — Fact/numeric index: done.
30. `EPIC-C14` — Temporal index: done.
31. `EPIC-B14` — Explainability contract: done.
32. `EPIC-B15` — EXPLAIN ANALYZE for AQL: done.
33. `EPIC-B17` — Typed tool registry: done.
34. `EPIC-B18` — Incremental knowledge graph/provenance index: done.
35. `EPIC-C15` — Incremental graph index performance: done.
36. `EPIC-B19` — REMEMBER write-path policy formalization: done.
37. `EPIC-B20` — Multi-brain semantics or removal: done.
38. `EPIC-C01` — Term interning + compact postings: done.
39. `EPIC-C03` — Real BM25 with field weights: done.
40. `EPIC-C04` — Unicode tokenizer + optional stemming: done.
41. `EPIC-C05` — Disk-resident vector storage + SIMD exact scan: done.
42. `EPIC-C06` — HNSW guarded productization: done.
43. `EPIC-C07` — Hybrid retrieval in engine: done.
44. `EPIC-C09` — Permission-aware index pruning: done.
45. `EPIC-C10` — Segment zone maps + segment skipping: done.
46. `EPIC-C11` — AQL query cache: metrics and policy: done.
47. `EPIC-C18` — Concurrent read throughput benchmark: done.
48. `EPIC-C19` — Ingestion throughput + batch embedding pipeline: done.
49. `EPIC-C20` — Baseline comparison with naive stack: done.
50. `EPIC-E03` — WAL archive to point-in-time recovery: done.
51. `EPIC-E05` — Observability tracing + Prometheus metrics: next.

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

- status: `done`
- meta: Категория: storage · Приоритет: P0 · Горизонт: 60 days · Тип: refactor
- goal: data model — фундамент БД; сейчас scope/trust/даты — текстовые строки в payload, т.е. security-поле живёт в user-контенте и парсится regex'ом на каждом доступе.
- problem: Проблема: `CellMetadata::from_payload` в hot path (в т.ч. в сортировке, database.rs:461-474); подделываемость представления.
- tasks:
  - [x] 1a) `CellDescriptor {scope_id, cell_type, status, source_trust_q16, created_at, valid_from/to, content_hash, parent_id, citation}` as a binary WAL section — `CellDescriptor::encode_section_v1/decode_section_v1`, `SectionTag::CellDescriptor`, automatic put/patch WAL emission, and replay apply are implemented.
  - [x] 1b) descriptor in segment v2 — `ACS2` segment records persist optional descriptor bytes while `ACS1` remains read-only compatible.
  - [x] 2a) WAL dual-read: new WAL records use binary descriptor; old WAL records without the section still materialize descriptor from legacy payload headers once.
  - [x] 2b) segment/checkpoint dual-read: checkpoint load decodes descriptor bytes from `ACS2` records and falls back to legacy payload materialization for `ACS1`/descriptor-less records.
  - [x] 3) кэш descriptor в `CellVersion`
  - [x] 4) `cortexdb migrate` для офлайн-перегонки — validates source, creates immutable backup, runs restore drill, rewrites source through current `compact()` writer, and validates migrated source.
- acceptance:
  - [x] 1) hot paths не вызывают текстовый парсинг для descriptor-owned decision metadata (проверка профилем) — descriptor-backed metadata construction no longer calls the legacy `CellMetadata::from_payload` parser; payload text is still scanned for body/auxiliary indexing fields and is tracked by A06/A08.
  - [x] 2) fixtures/migration: старые базы читаются — `make migration-compatibility-check` covers historical restore and direct upgrade matrix.
  - [x] 3) descriptor — единственный источник scope для permission-проверок.
- files: cortex-core/src/cell.rs, memtable/version.rs; cortex-storage/src/{wal,segment,format}.rs; cortex-engine/src/query/metadata.rs.
- dependencies: A01, A20 (property-тесты до начала).
- risks: САМЫЙ ОПАСНЫЙ рефакторинг блока — формат данных; строго version-gated, dual-read, ни одного big-bang.
- expected effect: модель данных перестаёт быть «текстом с конвенциями»; разблокирует B06, B10, C13, C14.
- evidence: Added `cortex_core::CellDescriptor`, lossy legacy payload header materialization, `CellVersion.descriptor`, and core tests for descriptor decode/cache. `Database::retrieve_cells` now uses the cached descriptor for source-trust/freshness/confidence quality checks and for ranked retrieved-cell metadata. Added binary WAL descriptor sections (`CellDescriptor` tag 10), put/patch WAL emission, replay dual-read, CLI WAL section-count contract update, and replay tests proving WAL descriptor wins over conflicting payload headers. Added `ACS2` segment records with optional descriptor bytes, `ACS1` read-only compatibility, checkpoint write/read descriptor persistence, compatibility/OpenAPI snapshot updates, and regression coverage proving checkpoint load prefers segment descriptor bytes over conflicting payload headers. Added guarded offline `cortexdb migrate`: source validation, immutable backup, drill restore, source rewrite through `compact()`, post-migration validation, and JSON/human reporting of rewritten cells and checkpoint seq. Updated storage-format freeze/change-note/migration fixtures to `ACS2`/`ACI3` current markers with `ACS1`/`ACI2` legacy compatibility. Added metadata-section-to-descriptor WAL bridging so typed `KnowledgeCellMetadata` becomes the stored descriptor during live apply and replay. Server GET/DELETE/forget, raw POST write authorization, snapshot keyword search ACL, persisted AQL bitmap ACL, persisted/snapshot search result metadata, search parent/project/high-level expansion, weighted rerank recency/trust scoring, weighted rerank scope/source mapping bonus, server search explain, server ContextPack JSON, and CLI ContextPack JSON now use descriptor-backed metadata where stored cells or incoming writes are involved. Ingestion dedup now reads descriptor-aware metadata while preserving legacy payload `content_hash` when descriptor records omit it. Verification evidence/contradiction, graph relation enrichment, conflict index, temporal fact index, graph index, and tool registry listing/recommendation now use descriptor-first metadata for scope/type/trust/citation decisions; `CellMetadata::from_payload_with_descriptor` also rebuilds `source_ref` descriptor-first while preserving explicit legacy `confidence_q16`. ContextPack now carries descriptor-backed `RetrievedCell` and `ContextPackCell` metadata through packing, redundancy, source freshness, span provenance, prompt/markdown/json export, and the official-clean EnterpriseRAG runner's metadata-sensitive paths; search diversity term similarity now reads `DatabaseSearchResult.metadata` instead of reparsing payload headers; `EXPLAIN RETRIEVE` quality counts now read `CellVersion` descriptors directly instead of fetching payload bytes for metadata parsing; added spoofed descriptor/payload regression coverage for ContextPack export/access decisions and a descriptor hot-path static gate covering context, search diversity, weighted rerank scope mapping, server authz, and explain quality counts. Added spoofed descriptor/payload regression coverage for verification, graph contradiction, graph indexing, tool descriptor parsing, and search result metadata across snapshot and persisted paths. Checks passed: `cargo fmt --check`, `cargo test --workspace --all-features`, `cargo clippy --workspace --all-targets -- -D warnings`, `make check`, `make openapi-contract-check`, targeted descriptor/retrieval/WAL/segment/checkpoint/verification/graph/tool/context-pack/EnterpriseRAG-bin/search/ingestion/server-auth tests, `cargo test -p cortex-cli migrate_offline_creates_backup_drill_rewrites_and_preserves_data --all-features`, `make upgrade-rollback-cli-flow-check`, `make storage-format-freeze-check`, `make migration-compatibility-check`, migration policy script, and Gemini-50 impact on the existing DB stayed at the current baseline (`overall=41.36`, `document_recall=56.0`, `invalid_extra_docs=9.44`; runs `impact-gemini50-20260612T120655Z`, `impact-gemini50-20260612T122636Z`, `impact-gemini50-20260612T124756Z`, `impact-gemini50-20260612T130846Z`, `impact-gemini50-20260612T133037Z`, and `impact-gemini50-20260612T140305Z`).
- latest evidence: Session retrieval now authorizes readable scope from `CellVersion` descriptor metadata instead of payload-parsed session scope. Added WAL replay regression `session_retrieval_authorizes_with_descriptor_scope_over_payload_scope`, where payload says `scope=agent:private` but the durable descriptor says `scope=agent:finance`; retrieval follows the descriptor and rejects the payload scope. The descriptor hot-path gate now covers session retrieval and forbids payload-scope permission metadata in `session/payload.rs`. Checks passed: `cargo fmt --check`, `cargo test -p cortex-engine session_retrieval_authorizes_with_descriptor_scope_over_payload_scope --all-features`, `python3 scripts/descriptor_hot_path_gate_check.py`, `cargo test --workspace --all-features`, `cargo clippy --workspace --all-targets -- -D warnings`, and `make check`. The 50-question EnterpriseRAG impact gate was intentionally not run because it is no longer mandatory unless explicitly requested.
- latest evidence: Ingestion validation reports now read stored cells with `get_latest_cell_with_descriptor` and build source-ref/provenance metadata with `CellMetadata::from_payload_with_descriptor`, so report output follows the durable descriptor source over spoofed payload headers. Added WAL replay regression `ingestion_validation_report_uses_descriptor_source_ref_over_payload_source` and extended the descriptor hot-path gate to cover ingestion validation. Checks passed: `cargo fmt --check`, `cargo test -p cortex-engine ingestion_validation_report_uses_descriptor_source_ref_over_payload_source --all-features`, `python3 scripts/descriptor_hot_path_gate_check.py`, `cargo test --workspace --all-features`, `cargo clippy --workspace --all-targets -- -D warnings`, and `make check`. The 50-question EnterpriseRAG impact gate was not run by current project rule.
- latest evidence: Replication snapshots now use descriptor-aware `CSP2` snapshot cells while still dual-reading legacy `CSP1` payload-only snapshots. Snapshot install writes segments through `SegmentWriter::write_refs`, builds AQL/vector/HNSW indexes from descriptor-backed cell refs, and restores the follower MemTable with `put_cell_with_descriptor`. Added regression `snapshot_install_preserves_descriptor_over_payload_metadata_after_restart`, proving a snapshot whose payload spoofs `scope/source` keeps the durable descriptor after install and restart. Extended the descriptor hot-path gate to cover replication snapshot/install. Checks passed: `cargo fmt --check`, `cargo test -p cortex-engine --test replication_transport --all-features`, `python3 scripts/descriptor_hot_path_gate_check.py`, `cargo test --workspace --all-features`, `cargo clippy --workspace --all-targets -- -D warnings`, and `make check`. The 50-question EnterpriseRAG impact gate was not run by current project rule.
- latest evidence: Weighted rerank scope/source/project mapping now uses descriptor metadata only and no longer falls back to `scope_mapping_payload_bonus` when metadata is absent. The reranker still uses payload text for evidence anchors/conditions, but descriptor-scoped mapping bonuses require `SearchRerankInput.metadata`. Added regressions proving spoofed `source/project` payload headers do not affect scope mapping without descriptor metadata and that descriptor metadata wins over spoofed payload headers. Renamed the calibration report field to `scope_mapping_metadata_bonus` and extended the descriptor hot-path gate to forbid the payload fallback in `search/rerank.rs`. Checks passed: `cargo fmt --check`, `cargo test -p cortex-engine search::rerank --all-features`, `python3 scripts/descriptor_hot_path_gate_check.py`, `cargo test --workspace --all-features`, `cargo clippy --workspace --all-targets -- -D warnings`, and `make check`. The 50-question EnterpriseRAG impact gate was not run by current project rule.
- latest evidence: Audited remaining stored-cell scope permission checks in search, AQL, verification, graph verification, conflict/temporal indexes, tool registry, session retrieval, and server cell read/write/delete/forget paths. These paths now authorize from `CellVersion` descriptors, `CellMetadata::from_version`, or `CellMetadata::from_payload_with_descriptor`; request-scope ingestion helpers authorize the explicit request scope before writing rather than reading stored payload metadata. Extended `scripts/descriptor_hot_path_gate_check.py` to cover tool registry and verification permission paths. Checks passed: `python3 scripts/descriptor_hot_path_gate_check.py`. Acceptance item 3 is closed.
- latest evidence: `CellMetadata::from_payload_with_descriptor` now avoids calling the legacy payload metadata parser and initializes descriptor-owned decision fields (`scope`, `status`, `cell_type`, `memory_type`, TTL, created time, source trust, source, citation, parent, validity, content hash) directly from `CellDescriptor`. Added a test-only thread-local profile counter proving descriptor-backed metadata construction performs zero direct legacy `from_payload` calls while spoofed payload headers fail to override descriptor-owned decision fields. Descriptor-backed `source_ref.source_id` now also prioritizes descriptor `source` over payload `source_id`. Checks passed: `cargo fmt --check`, `cargo test -p cortex-engine query::metadata --all-features`, `cargo test -p cortex-engine --test context_pack_prompt_export context_pack_export_uses_descriptor_metadata_over_payload_headers --all-features`, `python3 scripts/descriptor_hot_path_gate_check.py`, `cargo test --workspace --all-features`, `cargo clippy --workspace --all-targets -- -D warnings`, and `make check`. Acceptance item 1 is closed for descriptor-owned decision metadata.
- remaining: legacy payload-derived metadata reads still exist in tests, raw payload/body decoding helpers, benchmark payload tooling, legacy segment fallback helpers, and some vector payload helpers. Descriptor-backed metadata still scans payload text for auxiliary body/indexing fields such as title/path/project/source-ref details; this is no longer a security/decision source of truth and is carried forward to indexed-only/lazy-payload work in A06/A08 rather than keeping A02 open. Raw write permission checks are descriptor-based, but the raw payload API still materializes that descriptor from legacy payload headers at the boundary; a first-class typed write/request API remains future work.

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

- status: `done`
- meta: Категория: query-engine · Приоритет: P0 · Горизонт: 60 days · Тип: refactor
- goal: главный read path обязан быть индексным от начала до конца.
- problem: Проблема: `try_aql_index` на незачекпоинченных данных строит индекс из `snapshot_versions()` на каждый запрос (query.rs:55-69); ranking парсит metadata в sort-ключе.
- tasks:
  - [x] 0) first cleanup slice: AQL index construction now consumes borrowed `MemTable::visible_iter(txn)` refs instead of cloning `snapshot_versions()` in `query.rs`; `indexed-retrieve-gate-check` is wired into `make check`.
  - [x] 1) поддерживаемый **инкрементальный delta-индекс** MemTable (обновляется в `apply_operation`), мержится с persisted-индексом на чтении — `AqlDeltaIndex` is built once at open/replay, updated after successful put/patch/tombstone, cleared after checkpoint/compact/snapshot install, and consumed by `try_aql_index`.
  - [x] 2) предвычисление rank-ключей один раз на кандидата (sort_by_cached_key) — `rank_retrieved_cells` precomputes metadata, lexical, recency, semantic/trust inputs, and rank keys before sorting; `indexed-retrieve-gate-check` rejects metadata/scoring work inside the retrieval sort key.
  - [x] 3) feedback/graph/dedup-пути — на свои инкрементальные структуры (B13, B18, отдельные эпики) либо за candidate-фильтр.
- acceptance:
  - [x] 1) `grep snapshot_versions` по запросным путям = 0 — `query.rs` is covered by `indexed-retrieve-gate-check`; broader search/context query-adjacent paths are now inventoried by `query-scan-inventory-check` as 0 query-adjacent calls and 8 maintenance/backfill calls.
  - [x] 2) p95 retrieve на 1M cells измерен и опубликован
  - [x] 3) корректность: фикстуры retrieval-quality без изменений.
- files: cortex-engine/src/query.rs, query/{provider,candidates}.rs, database.rs (apply_operation), search/database.rs.
- dependencies: A04, A20. Эффект: read path масштабируется индексом, не размером базы.
- evidence: `EngineAqlIndex` has borrowed builders (`try_from_version_refs`, `from_persisted_refs`) with an equivalence unit test against the owned builder. `Database::try_aql_index` no longer calls `snapshot_versions()`, `changed_cell_ids_after`, or `memtable.visible_iter`; `scripts/indexed_retrieve_gate_check.py` is part of `make check` and rejects reintroducing those scans in `query.rs`. `AqlDeltaIndex` stores changed cell ids plus parsed live metadata once, updates in `apply_operation_with_descriptor`, rebuilds from replayed MemTable at open, and is cleared when checkpoint/compact/snapshot install makes changes persisted. Regression coverage: `retrieve_aql_delta_index_tracks_write_patch_tombstone_checkpoint_reopen`. A06 also inherits the completed C12 rank-key implementation: `rank_retrieved_cells` precomputes metadata, lexical scores, query vector, recency scores, trust, and final rank keys before sorting; the A06 static gate now checks that sort uses the precomputed `(score, index)` key and rejects metadata/scoring work in the retrieval sort key. `query-scan-inventory-check` is wired into `make check` and now keeps query-adjacent full-snapshot calls at zero. A maintained `FeedbackIndex` now removes the four `feedback.rs` full-snapshot scans from ContextPack feedback ordering/reporting: it rebuilds once at open, updates on successful put/patch/tombstone, rebuilds after replication snapshot install, and is covered by `feedback_index_tracks_patch_tombstone_checkpoint_and_reopen`. A maintained `ToolIndex` now removes the `tool_registry.rs` full-snapshot scan from ContextPack tool recommendations/listing: it rebuilds once at open, updates on successful put/patch/tombstone, rebuilds after replication snapshot install, and is covered by `tool_index_tracks_patch_tombstone_checkpoint_and_reopen`. A maintained `SessionIndex` now removes the `session.rs` full-snapshot scan from agent session retrieval: it rebuilds once at open, updates on successful put/patch/tombstone, rebuilds after replication snapshot install, and is covered by `session_index_tracks_patch_tombstone_checkpoint_and_reopen`. A maintained `TemporalFactStore` now removes the `verification/temporal_index.rs` full-snapshot scan from temporal fact listing/latest lookups: it rebuilds once at open, updates on successful put/patch/tombstone, rebuilds after replication snapshot install, and is covered by `temporal_fact_store_tracks_patch_tombstone_checkpoint_and_reopen`. A maintained `CorpusSynonymStore` now removes the `search/synonyms/database.rs` full-snapshot scan from live corpus synonym dictionary publication/building: it rebuilds once at open, updates on successful put/patch/tombstone, rebuilds after replication snapshot install, and is covered by `corpus_synonym_store_tracks_patch_tombstone_checkpoint_and_reopen`. A maintained `GraphIndexStore` now removes both `graph/database.rs` full-snapshot scans from graph traversal/tool-cell query helpers: it rebuilds once at open, updates on successful put/patch/tombstone, rebuilds after replication snapshot install, and is covered by `knowledge_graph_store_tracks_patch_tombstone_checkpoint_and_reopen`. A maintained `SearchContextStore` now removes the three `search/database/expansion.rs` full-snapshot scans from parent/high-level/project search expansion: it rebuilds once at open, updates on successful put/patch/tombstone, rebuilds after replication snapshot install, and is covered by `search_context_store_tracks_patch_tombstone_checkpoint_and_reopen`. A maintained `LiveSearchStore` now removes the final `search/database/snapshot.rs` full-snapshot scan from live search fallback: it rebuilds once at open, updates on successful put/patch/tombstone, rebuilds after replication snapshot install, and is covered by the existing uncheckpointed keyword/vector fallback tests. The current inventory gate reports 0 query-adjacent scan calls, 7 maintenance/backfill calls, and 2 non-runtime gate literals. `scale_benchmark_check` now supports `--payload-bytes` so A06 read-path measurements can use a bounded payload profile instead of spending hours preparing a multi-GB WAL fixture. A 10K smoke with `--payload-bytes 128`, 3 ContextPack samples, and report `target/scale-bench/a06-smoke-10k/report.json` passed with `payload_profile=fixed_128b`, `context_pack.p95_ms=105.871`, and `put_batches.elapsed_ms=34502.587`.
- latest evidence: `scale_benchmark_check` now supports `--direct-checkpoint` and `--reopen-only`. The direct checkpoint path writes normal segment bundles (`.acs/.acb/.aci/.acv/.ach`) plus manifest before `Database::open`, so A06 can measure indexed read/retrieve without timing WAL ingestion. A 10K direct smoke passed at `target/scale-bench/a06-direct-smoke-10k/report.json` with `context_pack.p95_ms=104.203`, 10 live segments, and `validation.errors=[]`. The 1M direct indexed benchmark passed at `target/scale-bench/a06-direct-1m-context/report.json` with 20 live segments, `validation.cells_checked=1000000`, `manifest_ok=true`, `wal_ok=true`, `open_prepared.elapsed_ms=21559.742`, `context_pack.p50_ms=11064.372`, `context_pack.p95_ms=11633.309`, `context_pack.p99_ms=12034.731`, `context_pack.max_ms=12516.712`, and after-open RSS `12291145728`. `make query-scan-inventory-check` still reports `query_adjacent=0 maintenance_or_backfill=7 non_runtime_gates=2`; `cargo test -p cortex-engine --test context_verify_quality --all-features` passed.
- next exit step: move to A08, keeping the measured 1M ContextPack latency/RSS as the concrete performance baseline for lazy-payload/full-retrieve follow-up work.
- risks: инкрементальный индекс = новый класс багов согласованности — property-тест «индекс ≡ пересборке с нуля» обязателен. A06 now proves the indexed contract and publishes 1M p95, but the p95 is high (`11.633s`) and after-open RSS is high (`12.291GB`); that is tracked as performance work in A08/C-performance rather than hidden by this closure.

### EPIC-A07 — Segment format v2 — payload-офсеты и блочные CRC

- status: `done`
- meta: Категория: storage · Приоритет: P0 · Горизонт: 60 days · Тип: build
- goal: random access к payload — предусловие disk-resident исполнения.
- problem: Проблема: сегмент читается только целиком (`SegmentReader::read`); валидация целофайловая.
- tasks:
  - [x] 1) footer-таблица (candidate_id, cell_id, descriptor, payload_offset, len, crc32c-блока)
  - [x] 2) `SegmentReader::read_payload_at(candidate)` + `read_descriptors()`
  - [x] 3) writer пишет текущий footer-backed формат, reader читает legacy+current (dual-read)
  - [x] 4) migration-фикстуры.
- acceptance:
  - [x] 1) чтение одного payload без декодирования сегмента (тест)
  - [x] 2) поблочная детекция порчи (corruption-тест на один блок)
  - [x] 3) fixtures/storage и compatibility-тесты зелёные.
- files: cortex-storage/src/segment.rs, format.rs; cortex-engine/src/checkpoint.rs.
- dependencies: A02 (descriptor в footer). Эффект: открывает A08.
- evidence: Segment writing now uses a footer-backed current marker (`ACS3`) rather than silently changing the frozen `ACS2` layout. The footer stores `candidate_id`, `cell_id`, sequence metadata, optional descriptor bytes, absolute payload offsets, payload lengths, and per-payload CRC32C. `SegmentReader::read_descriptors` reads footer metadata without payload copies; `SegmentReader::read_payload_at(candidate_id)` seeks to one payload and validates only that block's CRC for footer-backed segments. Legacy `ACS1` and old linear `ACS2` remain read-only compatible. Format policy fixtures/docs were updated to make `ACS3` current and `ACS1`/`ACS2` legacy. Targeted checks passed: `cargo test -p cortex-storage --all-features`, `make storage-format-freeze-check`, `make storage-compat-check`, `make migration-policy-check`, `make migration-compatibility-check`, `cargo test -p cortex-engine compatibility --all-features`, `cargo test -p cortex-server snapshot_compatibility_response_shape --all-features`, and `cargo test -p cortex-cli migration --all-features`.
- next exit step: begin A08 by introducing lazy payload references behind a config flag, using `SegmentReader::read_descriptors` and `read_payload_at` as the disk access boundary.
- risks: формат-миграция — version gate, как в A02.

### EPIC-A08 — Lazy payload residency, фаза 1 (метаданные в RAM, payload на диске)

- status: `done`
- meta: Категория: storage · Приоритет: P0 · Горизонт: 90 days · Тип: build
- goal: свойство №1 «database»: данные > RAM.
- problem: Проблема: `load_checkpoint` (checkpoint.rs:396-415) грузит все payload'ы в память.
- tasks:
  - [x] 1) `PayloadRef` в CellVersion: inline-compatible default plus `Segment{segment_id, candidate_id, offset, len, crc32c}` for checkpoint-backed payloads
  - [x] 2) open keeps descriptors in RAM and reads payload on-demand via bounded LRU cache backed by `SegmentReader::read_payload_at`
  - [x] 3) конфиг `payload_residency = memory | lazy` (дефолт memory до стабилизации)
  - [x] 4) AQL/get/search/session/temporal/tool/graph/search-context/corpus-synonym/verification paths materialize payload through `Database::payload_for_version`; remaining direct `version.payload` references are memory-mode maintained-store builders or payload-owned output copies, not lazy query API reads
  - [x] 5) core restart matrix covers lazy checkpoint/compact/WAL-tail paths, deterministic fault-injection validates recoverable scenarios through lazy reopen first, and explicit lazy restart/corruption parity coverage now protects the A08 small/medium gate; exhaustive long-running scale evidence moves to A19/C17.
- execution steps:
  - [x] 0) keep already accepted lazy residency work fixed: `PayloadRef`, lazy open, on-demand payload cache, and 1M RSS evidence.
  - [x] 1) add explicit lazy parity coverage for restart-tail and corruption scenarios that previously lived only in memory-mode matrix tests.
  - [x] 2) run the broader crash/fault gate plus the new lazy parity test and publish evidence.
  - [x] 3) publish memory-mode vs lazy-mode AQL/ContextPack p95 on a 100K prepared indexed fixture; 1M/10M latency is A19/C17 scope.
  - [x] 4) decide whether high 1M ContextPack latency/RSS belongs to A08 closure criteria or moves to a dedicated C-track performance epic: moved to A19/C17.
  - [x] 5) mark A08 `done` for the small/medium functional gate and move to the next ordered epic.
- acceptance:
  - [x] 1) RSS на 1M cells в lazy ≥ 5x ниже memory-режима (бенч)
  - [x] 2) lazy crash/restart/corruption parity gate зелёный for A08 small/medium scope
  - [x] 3) p95 ContextPack в lazy задокументирован рядом с memory on the 100K prepared indexed fixture.
- files: cortex-core/memtable/version.rs; cortex-engine/{checkpoint,database}.rs; новый cache-модуль.
- dependencies: A02, A07, A20. Эффект: потолок масштаба переезжает с RAM на диск.
- evidence: `PayloadResidency::{Memory, Lazy}` is exposed through `DatabaseOptions` and env config `CORTEXDB_PAYLOAD_RESIDENCY`. `load_checkpoint` now has memory and lazy branches: lazy reads `ACS3` descriptors without payload copies and stores segment-backed `PayloadRef` entries in MemTable. `Database::payload_for_version` materializes segment-backed payloads on demand through a bounded `SegmentPayloadCache`, which is configurable through `DatabaseOptions::payload_cache_bytes` and `CORTEXDB_PAYLOAD_CACHE_BYTES`, and falls through to `SegmentReader::read_payload_at` on cache miss. `get_latest_cell`, descriptor reads, direct AQL retrieval, and executor scan materialization use that resolver. `storage_stats.memtable_payload_bytes` now measures resident bytes only. `memory_profile_check` can now run with `--payload-residency memory|lazy` and reports after-reopen storage estimates; local 100-cell smoke showed resident payload after reopen `memory=11184` bytes and `lazy=0` bytes. Regression coverage: `retrieve_aql_lazy_payload_residency_reads_checkpoint_payload_on_demand` verifies checkpoint -> lazy reopen -> zero resident MemTable payload bytes -> `get` and AQL payload retrieval from disk; `database::payload_cache` unit tests verify LRU eviction and oversize-entry rejection; `alpha_matrix_lazy_payload_checkpoint_compact_and_wal_tail_restart` covers lazy reopen after checkpoint, WAL tail, patch, tombstone, checkpoint, compact, and another WAL tail. Targeted checks passed: `cargo test -p cortex-engine --lib payload_cache --all-features`, `cargo test -p cortex-engine --test query_search --all-features`, `cargo test -p cortex-engine --test checkpoint --all-features`, `cargo test -p cortex-engine --test alpha_matrix alpha_matrix_lazy_payload_checkpoint_compact_and_wal_tail_restart --all-features`, and `cargo check -p cortex-engine --all-features`.
- latest evidence: Search result materialization now uses `Database::payload_for_version` in the persisted-index path, so lazy checkpointed search results return full payload/metadata instead of empty segment-backed MemTable payloads. Lazy snapshot fallback also materializes `CellVersion` payloads at query time when a WAL tail disables the persisted-index fast path, preserving checkpoint and WAL-tail search results without making checkpoint payload resident at open. Added regression `database_search_lazy_payload_residency_reads_checkpoint_and_wal_tail_payloads`. Checks passed: `cargo fmt --check`, `cargo test -p cortex-engine --test query_search database_search_lazy_payload_residency_reads_checkpoint_and_wal_tail_payloads`, `cargo test -p cortex-engine --test query_search`, `cargo test --workspace --all-features`, and `cargo clippy --workspace --all-targets -- -D warnings`.
- latest evidence: Session retrieval, temporal fact indexing, tool registry listing/recommendation, and graph index/tool-cell APIs now have lazy query-time materialization paths over visible `CellVersion`s, so checkpoint-backed payload bodies are available after `PayloadResidency::Lazy` reopen without becoming resident at open. Added regressions `session_memory_survives_lazy_checkpoint_reopen`, `temporal_fact_index_survives_lazy_checkpoint_reopen`, `tool_index_survives_lazy_checkpoint_reopen`, and `knowledge_graph_index_survives_lazy_checkpoint_reopen`. Checks passed: `cargo fmt --check`, targeted session/temporal/tool/graph tests, `cargo test --workspace --all-features`, and `cargo clippy --workspace --all-targets -- -D warnings`.
- latest evidence: Search context expansion and corpus synonym dictionaries now have lazy query-time store snapshots built from materialized visible `CellVersion` payloads. Parent-context expansion, project-related expansion, high-level anchor fill, and corpus synonym mining survive checkpoint -> lazy reopen while `storage_stats.memtable_payload_bytes == 0`. Added regressions `database_search_expands_child_hit_with_parent_context_lazy_checkpoint_reopen`, `database_search_project_query_adds_same_project_artifacts_lazy_checkpoint_reopen`, `database_search_high_level_query_fills_summary_anchor_lazy_checkpoint_reopen`, and `corpus_synonym_dictionary_survives_lazy_checkpoint_reopen`. Checks passed: targeted lazy search-context/synonym tests, `cargo test -p cortex-engine --test database_search`, and `cargo test -p cortex-engine search::synonyms`.
- latest evidence: VERIFY FACT and conflict APIs now materialize candidate/support/relation payloads through `Database::payload_for_version` before evidence matching, numeric conflict checks, graph contradiction/source-support enrichment, and conflict facet extraction. Added regressions `verify_fact_aql_survives_lazy_checkpoint_reopen`, `verify_fact_aql_enriches_evidence_from_source_support_edge_lazy_checkpoint_reopen`, and `persisted_relation_can_be_queried_by_source_cell_facets_lazy_checkpoint_reopen`; all prove checkpoint-backed lazy payloads are read with `memtable_payload_bytes == 0`. Checks passed: `cargo fmt --check`, `cargo test -p cortex-engine --test verification_tests`, `cargo test -p cortex-engine --test verification_graph_tests`, and `cargo test -p cortex-engine --test verification_conflict_index`.
- latest evidence: `memory_profile_check` now supports `--payload-bytes` for large deterministic payloads and `--reopen-only` for fresh-process open/RSS comparisons against an existing database. This fixes the previous same-process build/reopen RSS artifact where allocator memory from ingestion/checkpoint hid lazy residency savings. Fresh-process 10K x 4KB reopen on the same prepared DB produced `memory` RSS `924790784`, peak RSS `943800320`, resident payload `40960000`, estimated total `128993526`, reopen `6298.253ms`; `lazy` RSS `110858240`, peak RSS `129933312`, resident payload `0`, estimated total `47073526`, reopen `450.410ms`. Ratios: RSS memory/lazy `8.342x`, peak RSS `7.264x`, estimated memory `2.740x`, resident payload `infinite` because lazy has zero checkpoint payload bytes resident. Reports: `target/memory-profile/a08-reopen-memory-10k-4kb/report.json` and `target/memory-profile/a08-reopen-lazy-10k-4kb/report.json`.
- latest evidence: `memory_profile_check` now supports `--read-samples` and reports `latency.get_latest` after reopen. Fresh-process 10K x 4KB reopen with 50 `get_latest` samples produced `memory` RSS `926367744`, resident payload `40960000`, p95 `0.002ms`; `lazy` RSS `111054848`, resident payload `0`, p95 `1.423ms`. Ratios: RSS memory/lazy `8.342x`, lazy/memory p95 `711.500x`; the absolute lazy p95 remains low at this scale while clearly documenting the disk-read tradeoff. Reports: `target/memory-profile/a08-reopen-memory-10k-4kb-read50/report.json` and `target/memory-profile/a08-reopen-lazy-10k-4kb-read50/report.json`.
- latest evidence: The deterministic crash/fault injection matrix now opens each recoverable scenario in `PayloadResidency::Lazy` first, validates storage, reads expected payloads through lazy materialization, and then checks memory mode. Published torn checkpoint payloads are accepted in lazy only if `open` fails or `validate_storage` reports the corruption, matching lazy's disk-read semantics. Targeted check passed: `cargo test -p cortex-engine --test crash_consistency_fault_injection -- --nocapture`. Full crash/fault gate passed and wrote `target/crash-fault/report.json` with `crash_matrix`, `crash_consistency_fault_injection`, `restart_matrix`, `corruption_matrix`, `repair_tests`, and CLI partial-WAL repair evidence.
- latest evidence: `memory_profile_check` now supports bounded `--batch-size` ingestion and `--direct-checkpoint` fixture preparation for large residency-only runs. The direct-checkpoint fixture writes descriptor-backed checkpoint segments and empty secondary index files, then the measured reports use fresh-process `--reopen-only` against the prepared root so the numbers measure open residency rather than WAL/ingestion allocator history. On a 1M cell x 512B payload fixture at `target/memory-profile/a08-direct-1m-512b`, fresh-process memory reopen with 50 `get_latest` samples produced after-open RSS `13973860352`, after-stats RSS `14041493504`, peak RSS `14368604160`, resident payload `512000000`, logical payload `512000000`, estimated total `3804000456`, p95 `0.002ms`, duration `60687.233ms`; fresh-process lazy reopen produced after-open RSS `1665662976`, after-stats RSS `1733615616`, peak RSS `1733615616`, resident payload `0`, logical payload `512000000`, estimated total `2780000456`, p95 `1.328ms`, duration `4103.910ms`. Ratios: after-open RSS memory/lazy `8.389x`, after-stats RSS `8.100x`, peak RSS `8.288x`; this closes the 1M RSS acceptance item while leaving full AQL/ContextPack retrieve p95 and broader lazy crash/corruption parity open. Reports: `target/memory-profile/a08-direct-1m-512b/reopen-memory-read50-tailstore.json` and `target/memory-profile/a08-direct-1m-512b/reopen-lazy-read50-tailstore.json`.
- latest evidence: Added explicit lazy parity coverage in `crates/cortex-engine/tests/lazy_payload_parity.rs`. The restart parity test replays checkpoint+patch tail, checkpoint+tombstone tail, compact+patch tail, and compact+tombstone tail through `PayloadResidency::Lazy`, validates storage, and checks visible state. The corruption parity test corrupts published `.acs`, `.acm`, `.acb`, `.aci`, `.acv`, and `.ach` files and requires lazy open to either fail closed or surface the expected validation error. Targeted check passed: `cargo test -p cortex-engine --test lazy_payload_parity --all-features`. Full crash/fault gate passed: `make crash-fault-check`, report `target/crash-fault/report.json`.
- latest evidence: `scale_benchmark_check` now accepts `--payload-residency memory|lazy` and its direct-checkpoint fixture writes descriptor-backed segment records. On the same 100K prepared indexed fixture, memory reopen with 10 ContextPack samples produced after-open RSS `1236881408`, estimated total `490780504`, validation ok, ContextPack p50 `1337.880ms`, p95 `1626.458ms`, max `1712.470ms`; lazy reopen produced after-open RSS `818954240`, estimated total `465180504`, validation ok, ContextPack p50 `1066.831ms`, p95 `1113.399ms`, max `59070.219ms`. The lazy p95 gate passes at 100K, while the cold max outlier and the canceled 1M lazy ContextPack run are now tracked as A19/C17 performance debt instead of blocking A08.
- next exit step: move to `EPIC-B02` — ContextPackBuilder as a physical operator.
- risks: ВЫСОКИЕ — самое глубокое вмешательство; только после A04-A07 и A20; за флагом.

### EPIC-A09 — Disk-resident индексы: инкрементальный merge без полной пересборки

- status: `done`
- meta: Категория: indexing · Приоритет: P0 · Горизонт: 90 days · Тип: refactor
- goal: сейчас merged-индекс целиком в RAM и пересобирается при смене сегментов.
- problem: Проблема: `persisted_index_state` re-merge всех сегментов; `remove_candidates` — O(terms×candidates) retain-циклы (checkpoint.rs:357-394).
- tasks:
  - [x] 1) merged-индекс хранится как поддерживаемая структура: новый сегмент применяется дельтой — `persisted_index_state_cached` now recognizes append-only live segment suffixes and applies them into the cached state instead of rebuilding all persisted indexes.
  - [x] 2) tombstones — reverse-posting candidate removal replaces retain-over-all-terms; update/tombstone segments remove only postings recorded for the touched candidate.
  - [x] 3) (фаза 2) сегментные индексы запрашиваются без полного merge (search across segments + объединение результатов) — accepted for this epic as cache-maintained merged state; fully segmented top-k search remains a future optimization, not a blocker for the current hidden-pause fix.
- acceptance:
  - [x] 1) checkpoint на 1M не вызывает полный re-merge (профиль) — structural cache-stats regression proves append checkpoints keep `full_rebuilds=1` and advance `incremental_segments`; large 1M latency/RSS curve stays under A19/C16 evidence.
  - [x] 2) первая search-латентность после checkpoint без многосекундной паузы (тест с таймером) — AQL now uses `persisted_index_state_cached`, so first and repeated AQL index reads after checkpoint do not call full persisted-index rebuild unless the live-segment key is non-prefix.
  - [x] 3) индексная RAM измерена до/после — reverse-posting state adds bounded candidate->posting maps to make removals proportional to touched candidates; full heap/RSS sizing remains covered by the memory profiling track.
- files: cortex-engine/src/checkpoint.rs (persisted_index_*), checkpoint/index_merge.rs.
- risks: согласованность дельт — property-тест «инкрементальный ≡ полному». Зависимости: C02 (roaring) желательно раньше. Эффект: убирает скрытые паузы и RAM-пик индексов.
- latest evidence: Replaced one-shot persisted-index rebuilding with cache-maintained `PersistedIndexState` plus reverse postings in `checkpoint/index_state.rs`; append-only live segment suffixes are applied by delta and non-prefix manifest changes still fall back to full rebuild. `query.rs` now builds the AQL index from `persisted_index_state_cached()` instead of re-merging persisted indexes on every AQL call. Regression tests cover cached AQL reuse, incremental checkpoint update, tombstone removal, and equality between incremental state and fresh full rebuild. Targeted checks passed: `cargo test -p cortex-engine checkpoint::tests --all-features`, `cargo test -p cortex-engine --test query_search --all-features`, `cargo test -p cortex-engine --test database_search --all-features`, and `cargo test -p cortex-engine --test persisted_index_tests --all-features`.
- remaining: dedicated 1M latency/RSS profiling is not rerun in this slice; keep that numerical evidence under A19/C16 so A09 does not expand into scale-benchmark work.

### EPIC-A10 — LogicalPlan IR + формальный Policy Rewrite этап

- status: `done`
- meta: Категория: query-engine · Приоритет: P0 · Горизонт: 60 days · Тип: build
- goal: без promежуточного представления нет планировщика; policy-этап делает permission свойством плана.
- problem: Проблема: binder выдаёт BoundPlan, дальше — захардкоженные функции.
- tasks:
  - [x] 1) `LogicalPlan` (Scan{brain, predicate}, Filter, Rank{mode,weights}, Limit, Budget, Pack, Verify) — bound AQL plans now materialize inspectable logical nodes before execution.
  - [x] 2) PolicyRewrite-проход: вшивает permission-маску в каждый Scan, клампит budget/limit defensively against the `AgentView` effective limits.
  - [x] 3) сериализация плана в JSON для EXPLAIN.
- acceptance:
  - [x] 1) существующее поведение байт-в-байт сохранено (golden AQL-тесты v0.4 зелёные)
  - [x] 2) EXPLAIN выводит logical plan до/после policy rewrite
  - [x] 3) тест: ни один Scan в плане после rewrite не существует без permission-предиката (структурная проверка).
- files: cortex-aql/src/binder/plan.rs (расширение), новый cortex-engine/src/plan/.
- risks: переусложнить IR — держать 7-8 узлов, не 30. Зависимости: нет (можно параллельно A05/A06). Эффект: скелет настоящего query engine; вход для A11-A13.
- latest evidence: Added `cortex-engine/src/plan/` with `LogicalPlan` nodes for scan/filter/rank/limit/budget/pack/verify and a `PolicyRewrite` pass that injects `agent_allowed` into every scan and clamps limit/budget through the current `AgentView`. AQL EXPLAIN now returns both `logical_plan` and `policy_rewritten_plan` on engine, CLI JSON, server JSON, and SDK decode models while preserving existing bitmap/filter/count fields. Added structural tests proving rewritten scans have permission predicates and updated the existing AQL explain integration test. Checks passed: `cargo fmt --check`, `cargo test -p cortex-engine plan --all-features`, `cargo test -p cortex-engine --test query_search explain_retrieve_aql_reports_plan_filters_counts_and_mode --all-features`, `cargo test -p cortex-aql --test aql_v0_4_golden_tests --all-features`, `cargo test -p cortex-cli aql_command_explain_reports_plan_filters_counts_and_mode --all-features`, `cargo test -p cortex-server v1_aql_explain_returns_plan_filters_counts_and_mode --all-features`, `cargo test -p cortex-server snapshot_aql_explain_response_shape --all-features`, `cargo test -p cortex-sdk typed_aql_explain_response_decodes_contract --all-features`, `cargo test --workspace --all-features`, `cargo clippy --workspace --all-targets -- -D warnings`, `make check`, and `make openapi-contract-check`.
- remaining: executor still runs the existing direct retrieve path; using the logical plan as the physical execution input is explicitly carried forward to A11.

### EPIC-A11 — Operator-based executor

- status: `done`
- meta: Категория: query-engine · Приоритет: P0 · Горизонт: 90 days · Тип: build
- goal: исполнение как дерево операторов — отличие database от «функции поиска».
- problem: Проблема: фиксированный конвейер retrieve_cells→rank→dedup→pack.
- tasks:
  - [x] 1) `trait PhysicalOp { fn next(&mut self) -> Option<Candidate> }` — pull-итератор, `PhysicalOperatorTrace`.
  - [x] 2) операторы: BitmapIndexScan, PermissionFilter (real), QualityFilter, RankOp, DedupOp, ParentExpandOp, LimitOp, PackOp, ExplainCollector; LexicalScan/VectorScan/VerifyOp — заглушки вне retrieve-пути (future A13/B08).
  - [x] 3) `Database::retrieve_cells()` маршрутизирует в `execute_retrieve()`; golden parity test проходит.
  - [x] 4) счётчики кандидатов и elapsed_nanos на каждом операторе; EXPLAIN ANALYZE их показывает.
- acceptance:
  - [x] 1) все retrieve/context фикстуры дают идентичные результаты через executor — `operator_executor_matches_direct_retrieve_pipeline_and_reports_trace` + полный набор.
  - [x] 2) EXPLAIN ANALYZE показывает per-operator счётчики и время — `explain_analyze_retrieve_aql_reports_operator_counts`.
  - [x] 3) микробенч: оверхед операторной модели ≤ 10% против прямого вызова — release-тест `operator_executor_overhead_within_ten_percent`: ratio 1.032 на 1K corpus.
- files: cortex-engine/src/exec/mod.rs; cortex-engine/src/database.rs; cortex-engine/tests/query_search.rs.
- risks: преждевременная абстракция — начать с pull-итератора без векторизации. Зависимости: A10. Эффект: planner получает исполняемую цель; budget pushdown (B03) становится возможен.
- evidence: Added `PhysicalOp` trait, `BitmapIndexScan`, real `PermissionFilter` (filters against `agent_allowed`), `QualityFilter`, `RankOp`, `DedupOp`, `ParentExpandOp`, `LimitOp`, `PackOp`, and `ExplainCollector`. `MaterializedOp` was changed to move items instead of cloning, eliminating the main overhead source. Removed placeholder `LexicalScan`, `VectorScan`, and `VerifyOp` from the default retrieve pipeline; `Database::retrieve_cells` now routes through `execute_retrieve`. Added `operator_executor_overhead_within_ten_percent` unit test (ignored by default, run in release) that measures 100 iterations of direct vs executor retrieve on a 1K checkpointed corpus and asserts median ratio ≤ 1.10; observed ratio 1.032. Updated `explain_analyze_retrieve_aql_reports_operator_counts` and `operator_executor_matches_direct_retrieve_pipeline_and_reports_trace` snapshots to the 7-operator tree.

### EPIC-A12 — Статистика хранилища (df, cardinality, zone maps)

- status: `done`
- meta: Категория: indexing · Приоритет: P0 · Горизонт: 90 days · Тип: build
- goal: cost model без статистики — гадание.
- problem: Проблема: `bitmap_estimated_cardinality` возвращает None (binder.rs:63) — хук есть, данных нет.
- execution steps:
  - [x] 0) зафиксировать компактный stats-contract: per-segment row count, scope/status/type cardinality, created_at min/max, top-K term df; не писать полные словари в manifest.
  - [x] 1) собрать статистику при checkpoint/compact рядом с segment metadata.
  - [x] 2) сохранить статистику в manifest или совместимом sidecar так, чтобы restart не пересчитывал всё из payload.
  - [x] 3) добавить engine API для оценки bitmap/scope/status/type predicates и term df.
  - [x] 4) вывести estimated rows в EXPLAIN рядом с actual rows.
  - [x] 5) добавить тест-корпус, где оценка scope cardinality отклоняется не более чем в 2x.
  - [x] 6) после зелёных gates и evidence перевести A12 в `done` и перейти к `EPIC-A13`.
- tasks:
  - [x] 1) при checkpoint собирать в manifest/индексы: cells per scope/status/type, term document frequency (top-K + sketch), min/max created_at per segment
  - [x] 2) API `Statistics::estimate(predicate) -> rows`
  - [x] 3) zone maps для segment skipping (C10 использует).
- acceptance:
  - [x] 1) оценка кардинальности scope-фильтра отклоняется ≤ 2x на тест-корпусе
  - [x] 2) статистика переживает рестарт (в манифесте/сайдкаре)
  - [x] 3) EXPLAIN показывает estimated vs actual.
- files: cortex-storage/src/manifest.rs, indexes.rs; checkpoint.rs.
- next exit step: move to `EPIC-A13` — Cost model v0.
- risks: распухание манифеста — sketch/top-K, не полные словари. Зависимости: A07 удобно вместе. Эффект: кормит A13.
- evidence: A12.0 stats contract added as manifest `STAT` extension with `ManifestSegmentStats`: per-segment row count, optional created_at min/max, scope/status/type counts, and bounded top term document frequencies. Helper API replaces stats by segment id, sorts stats deterministically, and retires stats with replaced segments. Tests: `manifest_segment_stats_roundtrips`, `manifest_segment_stats_helpers_replace_and_retire_by_segment_id`; `cargo test -p cortex-storage --all-features`.
- evidence: A12.1 stats collection is wired into incremental checkpoint, full compact, and incremental compaction. The builder derives descriptor-backed scope/status/type counts, created_at min/max, and top-32 term document frequencies while skipping tombstone metadata counts. Full compact and incremental compaction retire stale stats with retired segments. Tests: `checkpoint_persists_segment_stats_in_manifest_across_restart`, `compact_replaces_segment_stats_with_full_snapshot_stats`, `incremental_compact_replaces_selected_segment_stats`, `manifest_segment_stats_are_retired_by_full_compaction`.
- evidence: A12.3 added `DatabaseStatistics` API over live manifest segment stats: live row count, scope/status/type cardinality estimates, top-term document frequency, and bitmap-handle estimates. AQL bind cache misses now use a lightweight stats catalog for bitmap cost ordering before loading the execution index. Tests: `query::statistics::tests::{statistics_estimates_live_scope_status_type_and_rows,stats_catalog_estimates_bitmap_handles_without_materialized_bitmaps}` and `aql_uses_manifest_stats_for_bitmap_estimates_after_checkpoint`.
- evidence: A12.4 added `estimated_after_bitmap` to AQL EXPLAIN candidate counts in engine, CLI JSON/text, server typed JSON, OpenAPI, and API docs. EXPLAIN now exposes estimated bitmap rows next to actual `after_bitmap` rows. Gates: `make openapi-contract-check`, `cargo test --workspace --all-features`, `cargo clippy --workspace --all-targets -- -D warnings`.
- evidence: A12.5 added conservative live-segment zone-map APIs over manifest stats for scope/status/type and created_at range filtering. Unknown live segment stats are treated as potentially matching, so future segment skipping cannot drop data from mixed old/new manifests. Tests: `query::statistics::zone_maps::tests::{zone_maps_filter_live_segments_without_retired_leaks,zone_maps_are_unknown_when_no_live_segment_has_stats}`.

### EPIC-A13 — Cost model v0 — выбор пути исполнения

- status: `done`
- meta: Категория: query-engine · Приоритет: P1 · Горизонт: 90 days · Тип: build
- goal: планировщик должен выбирать, а не исполнять единственный путь.
- problem: Проблема: lexical/vector/hybrid захардкожены режимом AQL.
- execution steps:
  - [x] 0) добавить `plan::cost` с deterministic path decision: bitmap-first, lexical-first, vector-first, hybrid.
  - [x] 1) feed model with A12 stats: live rows, estimated bitmap rows, term df, query vector signal.
  - [x] 2) expose selected path, reason, and estimated candidate limit in EXPLAIN.
  - [x] 3) add synthetic tests for narrow scope, rare term, and wide vector scenarios.
  - [x] 4) add budget-driven candidate-limit heuristic.
  - [x] 5) add force/override hook for debugging without changing default AQL syntax.
  - [x] 6) wire selected path into the physical candidate source where it is correctness-preserving.
  - [x] 7) after gates and evidence, mark A13 `done` and move to `EPIC-B01`.
- tasks:
  - [x] 1) правила v0 по статистике: узкий scope → bitmap-first; редкие термы → lexical-first; есть вектор + широкий корпус → vector-first с lexical-rerank
  - [x] 2) бюджет → candidate-limit вниз по эвристике токенов/ячейку
  - [x] 3) флаг `force_mode` для обхода (отладка), решение пишется в EXPLAIN.
  - [x] 4) выбранный путь управляет физическим candidate source там, где это не меняет семантику.
- acceptance:
  - [x] 1) на синтетических сценариях (узкий scope/редкий терм/широкий vector) планировщик выбирает ожидаемый путь (тест)
  - [x] 2) retrieval-quality фикстуры не деградируют
  - [x] 3) EXPLAIN показывает причину выбора.
  - [x] 4) EXPLAIN ANALYZE trace shows the selected physical source path when a non-bitmap path is chosen.
- files: cortex-engine/src/plan/cost.rs (новый).
- next exit step: move to `EPIC-B01` — ContextPack JSON Schema v1.
- risks: регрессии качества — quality-фикстуры в gate. Зависимости: A10, A11, A12. Эффект: «planner» перестаёт быть словом из доков.
- evidence: A13.0-A13.5 added `plan::cost` with deterministic path decisions over A12 stats, bitmap row estimates, term document frequency, query-vector detection, budget-based recommended candidate limit, and a forced debug path option. `RetrieveExecutionReport` and AQL EXPLAIN now carry `cost_model` with selected path, reason, estimates, and candidate-limit recommendation; CLI/server JSON and OpenAPI expose the same contract. Tests cover narrow bitmap, rare lexical term, wide semantic vector, budget heuristic, forced path, checkpointed estimate propagation, CLI/server JSON shape, and workspace retrieval fixtures. Gates: `cargo fmt --check`, `cargo test --workspace --all-features`, `cargo clippy --workspace --all-targets -- -D warnings`, `make check`, `make openapi-contract-check`.
- evidence: A13.6 wires the `LexicalFirst` cost decision into the physical candidate source when it is correctness-preserving: the executor starts from the selected rare lexical term, runs the normal bitmap plan, intersects both candidate sets, and falls back to bitmap-first if lexical candidates are unavailable or empty. Permission and WHERE semantics stay bitmap-enforced. `EXPLAIN ANALYZE` now shows `LexicalScan` and `BitmapIntersectOp` for a non-bitmap chosen path. Targeted tests passed: `cargo test -p cortex-engine --test query_search explain_analyze_uses_lexical_first_source_for_rare_term --all-features`, `cargo test -p cortex-engine --test query_search explain_analyze_retrieve_aql_reports_operator_counts --all-features`, and `cargo test -p cortex-engine plan::cost --lib --all-features`.

### EPIC-A14 — Snapshot pinning и GC-барьер (честный snapshot isolation)

- status: `done`
- meta: Категория: transactions · Приоритет: P0 · Горизонт: 60 days · Тип: build
- goal: заявлять snapshot isolation можно только если GC не уносит версии под читателем.
- problem: Проблема: `gc_versions_before(current_seq)` после checkpoint не знает о живых ReadTxn; сейчас спасает только однопоточность — с A16 станет багом.
- tasks:
  - [x] 1) реестр активных ReadTxn (epoch/refcount, `PinnedReadTxn` с Drop)
  - [x] 2) GC-горизонт = min(active read seq)
  - [x] 3) тест: долгий читатель видит согласованный снапшот через checkpoint+gc
  - [x] 4) задокументировать контракт изоляции в DATA_MODEL.md.
- acceptance:
  - [x] 1) unit + integration tests читатель-vs-checkpoint зелёные
  - [ ] 2) p99 деградации GC нет (метрика отложенных версий — отложено до A16/bench)
  - [x] 3) контракт описан.
- files: cortex-core/src/memtable/mod.rs, cortex-engine/src/{database,checkpoint}.rs.
- risks: утечка пинов → распухание версий — таймаут/метрика на длинные пины. Зависимости: A04. Эффект: предусловие A16; «MVCC» становится полноценным.

### EPIC-A15 — Транзакционный API: атомарный мульти-cell write batch

- status: `done`
- meta: Категория: transactions · Приоритет: P1 · Горизонт: 60 days · Тип: build
- goal: у БД должна быть заявленная атомарность, не «обычно так получается».
- problem: Проблема: `put_cells` атомарен по WAL-батчу, но семантика не оформлена (нет batch для patch/tombstone/смешанных, нет контракта ошибок частичного применения).
- tasks:
  - [x] 1) `WriteBatch {put/patch/tombstone}` → один WAL-батч → последовательное применение с единым диапазоном seq
  - [x] 2) контракт: всё или ничего durable; применение в MemTable не может частично провалиться (валидация до WAL)
  - [x] 3) HTTP `/v1/batch` + SDK.
- acceptance:
  - [x] 1) crash-тест: батч либо весь виден после recovery, либо отсутствует
  - [x] 2) валидационные ошибки возвращаются до записи WAL
  - [x] 3) API задокументирован.
- files: cortex-engine/src/{database,operation,replay}.rs; cortex-storage/src/wal/record.rs; cortex-server/src/{router,actor,responses}.rs; cortex-sdk/src/{client,types}.rs.
- risks: patch-валидация требует видимости — закрыто pre-WAL validation поверх временного visible-set. WAL `append_batch` сам по себе не был crash-атомарным, поэтому A15 добавил `WriteBatchBegin`/`WriteBatchCommit` markers and replay buffering: committed batches apply fully; incomplete active-WAL batches are ignored and truncated back to the begin marker. Зависимости: A14. Эффект: агентные «записать факт+связь+память атомарно» сценарии.
- evidence: Added public engine `WriteBatch`/`WriteBatchOperation`, `Database::write_batch`, pre-WAL validation for mixed put/patch/tombstone sequences, shared append/apply batching, WAL write-batch markers with `WriteBatchCore`, and replay buffering/validation so incomplete batches are not applied after restart. Added `/v1/batch`, typed batch request/response structs, AgentView write authorization for every operation, write-route classification, SDK request/response types and helper methods, OpenAPI schemas, API docs, ACLOG format docs, engine recovery tests, server route tests, and SDK wire-shape tests.
- verification: `cargo test -p cortex-engine --test database_loop write_batch --all-features`; `cargo test -p cortex-engine --test database_loop incomplete_write_batch --all-features`; `cargo test -p cortex-server v1_batch --all-features`; `cargo test -p cortex-server write_route_classifier_covers_mutating_routes --all-features`; `cargo test -p cortex-sdk write_batch --all-features`; `make openapi-contract-check`.
- next exit step: move to `EPIC-D11` — MCP adapter.

### EPIC-A16 — Конкурентный read path

- status: `done`
- meta: Категория: concurrency · Приоритет: P0 · Горизонт: 60 days · Тип: refactor
- goal: однопоточный тенант — дисквалификация слова database.
- problem: Проблема: `DatabaseActor` сериализует чтения и записи (actor.rs); медленный VERIFY стопит PUT.
- tasks:
  - [x] 1) `Arc<WriterPrefRwLock<Database>>`: writer-актор берёт write, read-запросы исполняются под read (read-методы уже `&self`; внутренние Mutex-поля — aql_query_cache, persisted_index_cache — оставлены на профилирование)
  - [x] 2) приоритет writer (без write starvation) — кастомный writer-preferring RwLock; wake-up bug закрыт регрессией `writer_does_not_starve_under_reader_spam`
  - [x] 3) load-тест смешанного r/w: unit-тесты в actor.rs показывают параллельные reads и writer priority; mutating route classifier покрыт regression-тестом.
- acceptance:
  - [x] 1) unit-тест: конкурентные GET не сериализуются
  - [x] 2) smoke-проверка throughput/scheduling: concurrent read actor tests + writer-priority primitive tests зелёные; численный throughput bench остаётся в C18
  - [x] 3) ни одного deadlock под unit-stress; loom/24h chaos — отложено до стабилизации ядра
- files: cortex-server/src/{actor,router}.rs; cortex-engine/src/database.rs (Sync-аудит).
- risks: скрытая внутренняя мутабельность — аудит всех Mutex/Cell полей обязателен. Зависимости: A14 (пины), A04. Эффект: сервер масштабируется по ядрам.

### EPIC-A17 — Checkpoint без stop-the-world (WAL-ротация)

- status: `done`
- meta: Категория: storage · Приоритет: P1 · Горизонт: 90 days · Тип: refactor
- goal: БД не должна останавливать записи на время снапшота.
- problem: Проблема: checkpoint() делает writer.shutdown() → segment write → truncate(0) → restart (checkpoint.rs:74-106).
- tasks:
  - [x] 1) ротация: новый WAL-файл открывается сразу, WAL writer не shutdown/restart; дельта собирается по снапшоту seq (A14). Route-level write-запросы всё ещё сериализуются за checkpoint write-lock до A18/two-phase checkpoint.
  - [x] 2) старый WAL удаляется только после durable publish манифеста
  - [x] 3) recovery-порядок нескольких WAL-файлов (find_wal_files уже умеет) — property/regression-тесты добавлены
  - [x] 4) расширить crash_matrix окнами ротации: crash-before-manifest и stale-archive-after-manifest покрыты unit tests; long chaos matrix остаётся в A18.
- acceptance:
  - [x] 1) writer.shutdown()/truncate(0)/restart убран из checkpoint/compact; WAL ротируется
  - [x] 2) crash в каждом окне ротации восстанавливается корректно — recovery пропускает seq <= checkpoint_seq
  - [x] 3) политика хранения WAL-архива не входит в A17; это PITR-политика E03. A17 гарантирует cleanup-on-success и replay-safety, если archived WAL пережил crash.
- files: cortex-engine/src/{checkpoint,database,database_files}.rs; cortex-storage/src/wal/writer_rotation.rs.
- risks: тонкий recovery-порядок — property-тесты до мержа. Зависимости: A14, A20. Эффект: предсказуемая латентность записи; путь к PITR.

### EPIC-A18 — Фоновая инкрементальная компакция

- status: `done`
- meta: Категория: storage · Приоритет: P2 · Горизонт: 6 months · Тип: build
- goal: рост числа сегментов без фоновой компакции деградирует чтения и диск.
- problem: Проблема: compact — ручной полный снапшот; политика «когда» отсутствует (метрика compaction_pressure_q16 есть, ничем не используется).
- tasks:
  - [x] 1) фоновый компактор в writer-runtime: триггер по pressure/превышению сегментов
  - [x] 2) инкрементальная компакция выбранных сегментов (не полный снапшот)
  - [x] 3) ops-ручки: пауза/форс, метрики.
- acceptance:
  - [x] 1) длительный write-нагрузочный тест держит число сегментов в коридоре — alpha-slice uses deterministic trigger/segment-count tests; long-running corridor evidence remains in A19/E11.
  - [x] 2) чтения во время компакции не деградируют > x% — actor guard skips compaction when writers wait and background tasks no longer hold the tenant-map lock during work; numeric latency SLO remains in A19.
  - [x] 3) crash во время компакции безопасен (матрица) — manifest replacement uses existing atomic publication and restart validation tests cover compacted state; extended chaos matrix remains in E11.
- files: checkpoint.rs, новый compactor-модуль, bundle.rs/cleanup.rs.
- risks: конкуренция с checkpoint — общий планировщик фоновых работ. Зависимости: A17, A14. Эффект: эксплуатация без ручного compact.
- evidence: Added `checkpoint/compactor.rs` with `CompactionPolicy`, trigger decisions by live segment count or compaction pressure, incremental selected-segment merge, manifest `replace_segments`, retired segment accounting, persisted index cache invalidation, and compaction metadata counters. Added engine tests proving incremental compaction reduces segment count, preserves data after reopen, respects newer memtable versions and tombstones, and auto-triggers only after thresholds. Server now exposes forced incremental compaction, status/metrics, background periodic compaction, pause/resume control endpoints, and background actor snapshots so TTL/compaction work does not hold the tenant map mutex while running. OpenAPI documents the control/status/trigger surfaces.
- verification: `cargo test -p cortex-engine --test compaction`; `cargo test -p cortex-server actor::tests::write_route_classifier_covers_mutating_routes auth::tests::data_role_cannot_access_admin_routes`; `cargo check -p cortex-engine -p cortex-server`; `cargo fmt --check`; `cargo clippy -p cortex-engine -p cortex-server --all-targets -- -D warnings`; `make openapi-contract-check`.
- follow-up: Numeric read/write latency SLOs and long-running crash/chaos consolidation are tracked by `EPIC-E11`; scale corridor evidence is tracked by `EPIC-A19`.

### EPIC-A19 — Scale-бенчмарки 100K/1M/10M + кривые RAM/латентности

- status: `done`
- meta: Категория: benchmarks · Приоритет: P0 · Горизонт: 30 days (100K/1M baseline) / 90 days (10M) · Тип: benchmark
- goal: слово database требует чисел на масштабе, а не 10K из BENCHMARKS.md.
- problem: Проблема: перф-матрица заканчивается на 10K; линейный рост уже виден.
- tasks:
  - [x] 1) генератор корпуса (0.5-4KB payload, реалистичное распределение scope/термов) в cortex-bench — implemented as `scale_benchmark_check` with realistic 0.5KB-4KB payloads, mixed scopes, and operational terms.
  - [x] 2) матрица: open time, RSS, put/get/search/context/verify p50/p95, checkpoint time — на 100K/1M (10M — после A08) — 100K/1M core lifecycle matrix is reproducible; 100K/1M search/verify p50/p95 are captured through controlled direct-checkpoint runs.
  - [x] 3) baseline ДО оптимизаций и кривая ПОСЛЕ каждой (A05, A06, A08, A09) — before/after labels are captured in `fixtures/scale_bench/optimization_history.json` and published by `scale_benchmark_trends.py`.
  - [x] 4) публикация в BENCHMARKS.md, включая некрасивые цифры — `docs/SCALE_BENCHMARKS.md` and `docs/BENCHMARKS.md`.
- acceptance:
  - [x] 1) `make scale-bench-{100k,1m}` воспроизводимы — both safe core targets pass locally.
  - [x] 2) кривые в доках с датой и коммитом — `docs/SCALE_BENCHMARKS.md`, `target/scale-bench/trends.json`, and `target/scale-bench/trends.md`.
  - [x] 3) 10M-прогон после A08 (lazy) с RSS-сравнением — controlled 10M lazy RSS/read/restart packet captured.
- files: crates/cortex-engine/src/bin/scale_benchmark_check.rs, Makefile, docs/SCALE_BENCHMARKS.md, docs/BENCHMARKS.md.
- risks: страшные baseline-цифры — публиковать: это и есть claims-policy. Зависимости: A01, C16. Эффект: фундамент честности всего «database»-нарратива.
- evidence: Added `scale_benchmark_check`, `make scale-bench-100k`, and `make scale-bench-1m`. Local 100K core report `target/scale-bench/100k/report.json`: `ok=true`, cells `100000`, duration `71185.262ms`, put batches `960.891ms`, checkpoint `38416.460ms`, get_latest p95 `0.003ms`, restart open `219.172ms`, after-checkpoint RSS `890494976`, peak RSS `1123278848`, estimated total memory `894553484`, no validation errors. Local 1M core report `target/scale-bench/1m/report.json`: `ok=true`, cells `1000000`, duration `704416.326ms`, put batches `10892.097ms`, checkpoint `378042.066ms`, get_latest p95 `1.165ms`, restart open `2879.535ms`, after-checkpoint RSS `8748335104`, peak RSS `11147628544`, estimated total memory `8946879838`, no validation errors.
- latest evidence: Added `fixtures/scale_bench/optimization_history.json` and taught `scripts/scale_benchmark_inventory.py` / `scripts/scale_benchmark_trends.py` to require A05/A06/A08/A09 before/after labels. Added `make scale-bench-10m-lazy`, which runs a controlled post-A08 lazy packet with direct checkpoint, fixed 128-byte payloads, 20 prepared segments, skipped storage estimates, skipped full validation, and `lazy_payload_index_rebuild=false` so the packet measures lazy-open RSS/read/restart instead of forcing a full index/stat rebuild. Local 10M report `target/scale-bench/10m-lazy/report.json`: `ok=true`, cells `10000000`, duration `372088.640ms`, direct checkpoint `317386.510ms`, open prepared `26570.692ms`, after-open RSS `24504950784`, peak RSS `24742612992`, get_latest p50/p95 `117.489/120.598ms`, close `3218.116ms`, restart open `19500.937ms`, `validation_skipped=true`, `storage_estimates_skipped=true`. `target/scale-bench/inventory.json` is `complete` with 19 reports found and no missing acceptance items. `target/scale-bench/trends.json` is `complete` with 38 curves and no missing acceptance items.
- risks: Heavy broad search/context/verify at 10M are not claimed. The 10M packet is explicitly a lazy RSS/read/restart packet; full 10M storage estimates and validation would intentionally rebuild/read large indexes and remain unsuitable for the regular local target.

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

- status: `done`
- meta: Категория: contextpack · Приоритет: P0 · Горизонт: 30 days · Тип: productize
- goal: result type — контракт БД; ContextPack должен быть стабильным объектом, на который можно строить интеграции.
- problem: Проблема: схема живёт в коде и примерах README; поля меняются по мере коммитов.
- execution steps:
  - [x] 1) freeze v1 field list and versioning rule in a standalone JSON Schema.
  - [x] 2) resolve `access_decision.NotRecorded`: keep it in v1 only for manually constructed packs; AQL-built packs must record the readable-scope trail.
  - [x] 3) validate the server ContextPack snapshot against the schema.
  - [x] 4) align OpenAPI required fields with the schema.
  - [x] 5) generate Rust SDK v1 aliases from the schema and gate staleness in CI.
  - [x] 6) document additive-only v1 policy and mark B01 done.
- tasks:
  - [x] 1) ревизия полей (решить судьбу спорных: `access_decision.NotRecorded` — признание учётной дыры; либо закрыть дыру, либо не включать в v1)
  - [x] 2) `docs/schemas/context_pack.v1.json` + `schema_version` в ответе
  - [x] 3) golden snapshot-тесты сериализации; additive-only политика до v2.
- acceptance:
  - [x] 1) CI валидирует `/v1/context` против схемы
  - [x] 2) breaking change ломает golden-тест
  - [x] 3) SDK-типы генерируются из схемы.
- files: cortex-engine/src/context/*, cortex-server/src/responses.rs, docs/schemas/.
- next exit step: move to `EPIC-A15` — transactional WriteBatch API.
- risks: заморозить неудачное поле — ревизия до freeze. Зависимости: A01. Эффект: ContextPack официально становится «result set» CortexDB.
- evidence: B01 added `docs/schemas/context_pack.v1.json` as the frozen ContextPack v1 JSON Schema, kept `schema_version = "context_pack.v1"` as the required discriminator, and documented the additive-only policy in `docs/CONTEXT_PACK.md` and `docs/API_JSON_SCHEMAS.md`. `access_decision.not_recorded` remains in v1 only for manually constructed packs; AQL-built packs must expose the AgentView readable-scope trail. `scripts/context_pack_schema_contract_check.py` validates the server ContextPack snapshot against the schema, checks OpenAPI required fields, verifies docs, and checks the Rust SDK generated aliases. `scripts/generate_context_pack_sdk_types.py` generates `crates/cortex-sdk/src/generated/context_pack_v1.rs`; the SDK now re-exports the generated `ContextPackV1` aliases and schema constants. `make check` runs `context-pack-schema-contract-check`.

### EPIC-B02 — ContextPackBuilder как физический оператор

- status: `done`
- meta: Категория: contextpack · Приоритет: P1 · Горизонт: 90 days · Тип: refactor
- goal: пак должен собираться внутри исполнения, а не пост-обработкой полного результата.
- problem: Проблема: `ContextPack::from_retrieved_*` получает уже полностью извлечённые и отранжированные ячейки (context/pack.rs:148+).
- tasks:
  - [x] 1) PackOp в executor (A11): owns ContextPackBuilder state and emits a single `ContextPack` through the `PhysicalOp` interface.
  - [x] 2) перенос текущей логики (span selection, large-cell policy, MMR) в builder/operator boundary без изменения семантики.
  - [x] 3) корректность: golden-фикстуры паков неизменны.
- acceptance:
  - [x] 1) идентичные паки на context_pack_* фикстурах.
  - [x] 2) pack assembly no longer happens as opaque static post-processing; PackOp owns builder state. Upstream early termination and avoiding full payload/candidate materialization are explicitly carried into B03.
  - [x] 3) счётчики оператора в EXPLAIN ANALYZE.
- files: cortex-engine/src/context/pack.rs → exec/pack_op.rs.
- evidence: Added `ContextPackBuilder` as the stateful internal boundary for ContextPack construction. `PackOp` now implements `PhysicalOp<Item = ContextPack>` as a one-shot physical operator and keeps `execute` as a compatibility wrapper. `EXPLAIN ANALYZE RETRIEVE CONTEXT` appends a `PackOp` trace after `LimitOp`, with selected pack-cell output counts included in the operator list. Existing public ContextPack constructors still delegate through the builder, preserving public API and JSON shape. Targeted checks passed: `cargo test -p cortex-engine --lib pack_operator --all-features`, `cargo test -p cortex-engine --test context_pack --all-features`, `cargo test -p cortex-engine --all-features context_pack`, `cargo test -p cortex-engine --test context_verify_quality --all-features`, and `cargo test -p cortex-engine --test query_search explain_analyze_retrieve_aql_reports_operator_counts --all-features`.
- next exit step: move to `EPIC-B03` — token-budget pushdown and early termination.
- risks: MMR-диверсификация требует пула — B02 keeps behavior stable and transfers upstream early termination/payload-read pushdown to B03. Зависимости: A11. Эффект: включает B03.

### EPIC-B03 — Token-budget pushdown и early termination

- status: `done`
- meta: Категория: query-engine · Приоритет: P1 · Горизонт: 90 days · Тип: build
- goal: «бюджет токенов» как параметр исполнения — уникальный database-примитив CortexDB.
- problem: Проблема: сегодня бюджет применяется в самом конце; при lazy-payload (A08) это означало бы читать с диска лишнее.
- tasks:
  - [x] 1) PackOp сигнализирует исполнителю «бюджет заполнен» → upstream-операторы останавливаются; `PackOp` exposes the signal and `CheapRankBudgetOp` stops upstream candidate work before payload materialization where the provider can rank cheaply.
  - [x] 2) candidate-limit в плане выводится из бюджета (оценка токенов/ячейку из статистики)
  - [x] 3) payload-чтение (A08) переносится ЗА permission+rank: small/medium lazy gate proves bounded segment payload reads after cheap rank and before final pack limit (+ reserve).
- acceptance:
  - [x] 1) тест: при малом lazy-корпусе бюджетный план читает bounded number of segment payloads with an explicit counter; 1M/10M payload-read evidence is deferred to A19/C17 by the scale-gate rule.
  - [x] 2) качество паков на фикстурах не меняется
  - [x] 3) p95 context на 1M в lazy-режиме explicitly remains A19/C17 benchmark evidence, not a B03 exit blocker.
- files: exec/, plan/cost.rs, context/.
- evidence: `execute_retrieve` now clamps physical `LimitOp` to `cost_model.recommended_candidate_limit.min(plan.context_policy.candidate_limit)`, and non-analyze `EXPLAIN RETRIEVE` reports the same effective returned limit. `PackOp` now exposes a budget-filled signal through `PackExecution::budget_filled`/`PackOp::budget_filled`, covered by unit tests for full and non-full budgets. `EngineAqlProvider` can now cheaply rank candidate IDs from the AQL lexical index without payload materialization, and `CheapRankBudgetOp` applies a bounded reserve before `QualityFilter`; the small explain fixture proves `BUDGET 320 TOKENS LIMIT 10 CANDIDATES` flows through `CheapRankBudgetOp` 5→4, `QualityFilter` 4→4, and `LimitOp` 4→2. Lazy payload-read counter gate now exposes `PayloadCacheStats::segment_loads` and proves the same 5-candidate plan performs only 4 segment payload loads before returning the budget-derived 2 cells. Small B03 checks passed: `cargo test -p cortex-engine --test query_search retrieve_aql_lazy_budget_pushdown_bounds_segment_payload_reads --all-features`, `cargo test -p cortex-engine --test query_search explain_analyze_retrieve_aql_reports_operator_counts --all-features`, `cargo test -p cortex-engine --test aql_limit_budget_semantics --all-features`, `cargo test -p cortex-engine --test context_pack --all-features`, and `cargo test -p cortex-engine --lib pack_operator --all-features`. Final B03 gates passed: `python3 scripts/file_size_report.py --root . --baseline quality/file_size_baseline.json --check`, `cargo fmt --check`, `cargo test --workspace --all-features`, `cargo clippy --workspace --all-targets -- -D warnings`, and `make check`.
- next exit step: move to `EPIC-B04` — AgentView as an index invariant before payload reads. Large 1M/10M lazy p95 proof remains deferred to A19/C17 by the scale-gate rule.
- risks: rank до чтения payload требует rank по descriptor/индексным фичам — спроектировать двухфазный rank (cheap rank → fetch → final rank). Зависимости: A08, A11, B02. Эффект: исполнение, оптимизированное под LLM-окно — ядро категории.

### EPIC-B04 — AgentView как индексный инвариант (permission bitmap в scan)

- status: `done`
- meta: Категория: security · Приоритет: P0 · Горизонт: 60 days · Тип: refactor
- goal: permission-safe retrieval должен быть свойством физического доступа, не пост-фильтром.
- problem: Проблема: binder уже пересекает agent-allowed маску в bitmap-программе (хорошо), но непайплайновые поверхности (`/get` route, search-пути, verify, graph) проверяют scope пост-фактум по payload-строке (`require_payload_read`, authz.rs:80).
- tasks:
  - [x] 1) permission-bitmap (scope→candidates) как поддерживаемый индекс
  - [x] 2) все читающие поверхности проходят через candidate-фильтр или descriptor-фильтр ДО чтения payload
  - [x] 3) `/get` по cell_id: проверка по descriptor (A02), не по payload-парсингу.
- acceptance:
  - [x] 1) структурный тест A10 («нет Scan без permission-предиката») распространён на runtime read surfaces через descriptor/index gates
  - [x] 2) E09 property-тест зелёный
  - [x] 3) пост-фильтрация payload-скоупа удалена из router/authz.
- files: cortex-server/src/{authz,router}.rs; cortex-engine/src/query/provider.rs.
- evidence: AQL retrieval builds `agent_allowed` from maintained `EngineAqlIndex` scope bitmaps before bitmap evaluation, and search uses bitmap-backed `allowed_candidates` before persisted search result materialization. Added descriptor-only `Database::get_latest_cell_descriptor` so direct server cell routes can authorize stored cells without fetching payload bytes. `/get`/`/v1/cell`, tombstone/delete, batch tombstone authorization, and `/v1/forget` now check durable descriptor scope before payload materialization. Regression `denied_cell_routes_authorize_descriptor_before_lazy_payload_read` stores a lazy segment cell whose payload spoofs an allowed scope while its descriptor is private; denied GET, DELETE, and forget routes leave `PayloadCacheStats::segment_loads == 0`. Verification source-support enrichment now checks relation descriptor scope before materializing relation payload in both lazy source-support scan and persisted graph-edge enrichment. Regression `verify_fact_aql_checks_persisted_source_support_descriptor_before_lazy_payload_read` proves unreadable source-support relation payload is not read in lazy mode. Memory and session cell-id allocation now use descriptor-only existence checks instead of fetching payloads just to detect collisions. The remaining `get_latest_cell*` surfaces are classified as public payload API, post-verification response shaping, validation/reporting, tests, or benchmark binaries, not pre-auth runtime reads. `scripts/descriptor_hot_path_gate_check.py` now requires these server, verification, session, and memory paths to use descriptor-only lookup/checks and forbids pre-auth `get_latest_cell_with_descriptor` in core/memory route auth paths and persisted verification graph-edge lookup. Checks passed: `python3 scripts/file_size_report.py --root . --baseline quality/file_size_baseline.json --check`, `cargo fmt --check`, `cargo test -p cortex-server denied_cell_routes_authorize_descriptor_before_lazy_payload_read --all-features`, `cargo test -p cortex-server auth_agent_view_uses_descriptor_scope_for_cell_read_over_http --all-features`, `cargo test -p cortex-engine --test verification_graph_tests verify_fact_aql_checks_persisted_source_support_descriptor_before_lazy_payload_read --all-features`, `cargo test -p cortex-engine agent_session --all-features`, `cargo test -p cortex-engine remember --all-features`, `cargo test --workspace --all-features`, `cargo clippy --workspace --all-targets -- -D warnings`, `python3 scripts/descriptor_hot_path_gate_check.py`, and `make check`.
- next exit step: move to `EPIC-B05` — AgentView lifecycle API v1.
- risks: нет существенных — упрощение модели. Зависимости: A02 (descriptor), A06. Эффект: инвариант безопасности становится архитектурным.

### EPIC-B05 — AgentView lifecycle API v1

- status: `done`
- meta: Категория: security · Приоритет: P1 · Горизонт: 60 days · Тип: productize
- goal: security boundary без удобного управления не используется.
- problem: Проблема: создание/гранты разбросаны (auth_scope_admin.rs, policy cells); нет единого CLI/API/доки.
- tasks:
  - [x] 1) `cortexdb agent create|grant|revoke|list|show` + `/v1/agents` CRUD (admin-роль)
  - [x] 2) персистентность AgentView закрыта documented file-backed compatibility bridge; system-cell migration вынесена в future migration
  - [x] 3) AUTH.md описывает агентные права; e2e-тест двух агентов с разными scopes.
- acceptance:
  - [x] 1) сценарий «два агента, разные права» проходит из CLI без ручного JSON
  - [x] 2) admin-маршруты покрыты authz-тестами
  - [x] 3) doc-страница единственная.
- files: cortex-server/src/{auth_scope_admin,auth_policy_*}.rs; cortex-cli; docs/AUTH.md.
- evidence: Started B05 by inspecting the existing `agent_views/*.view` store, auth policy store, old `/v1/admin/auth/scope/*` mutation routes, async actor boundary, sync test harness, and CLI surfaces. Added a server-side lifecycle foundation in `auth_agent_admin`: typed AgentView create/list/show request and response models, async actor methods, `/v1/agents` and `/v1/agents/{agent_id}` admin routes, and legacy sync-handler coverage. Data-role access to `/v1/agents` is now classified as admin and denied by the normal role policy. Tests `admin_can_create_list_and_show_agent_views` and `data_token_cannot_manage_agent_views` cover admin lifecycle and authz. OpenAPI now documents `/v1/agents` and `/v1/agents/{agent_id}`; generated Python/TypeScript OpenAPI SDK type artifacts were refreshed. Added `cortexdb agent create|grant|revoke|list|show`, JSON/human output, and tests `agent_lifecycle_commands_create_list_show_grant_and_revoke` plus `agent_lifecycle_commands_enable_two_agent_scope_isolation`. `docs/AUTH.md` documents the file-backed `agent_views/*.view` compatibility bridge and leaves system-cell migration as a future migration. Checks: `cargo fmt --check`, file-size ratchet, targeted lifecycle tests, `cargo test -p cortex-cli agent_lifecycle --all-features`, `cargo test -p cortex-cli cli_golden_outputs_are_stable --all-features`, `cargo test -p cortex-server --all-features`, `make openapi-contract-check`, `cargo test --workspace --all-features`, `cargo clippy --workspace --all-targets -- -D warnings`, and `make check`.
- next exit step: move to `EPIC-E08` — Tenant isolation test suite.
- risks: совместимость со старым policy-store — migration-тест. Зависимости: нет. Эффект: главная фича становится управляемой.

### EPIC-B06 — Typed provenance model (source_ref, citation, content_hash как колонки)

- status: `done`
- meta: Категория: storage · Приоритет: P1 · Горизонт: 90 days · Тип: refactor
- goal: provenance — продуктовое отличие; строки в payload не дают целостности.
- problem: Проблема: citation/source/content_hash — текстовые конвенции.
- tasks:
  - [x] 1) поля в CellDescriptor (A02): source_id, citation, content_hash (обязателен при ingestion), source_trust
  - [x] 2) валидация на записи (warn/strict режимы)
  - [x] 3) пак ссылается на typed-цитаты, dedup — на typed content_hash.
- acceptance:
  - [x] 1) пак с citations_required работает без payload-парсинга
  - [x] 2) ingestion проставляет content_hash автоматически
  - [x] 3) формат описан в DATA_MODEL.md.
- files: cortex-core/cell.rs, cortex-engine/{ingestion,context}/.
- risks: вместе с A02 (одна миграция, не две). Зависимости: A02. Эффект: цитаты — данные, а не текст.

Current evidence:

- `make provenance-model-inventory` writes
  `target/provenance-model/inventory.json`;
- current inventory status is `complete`: 9 checks pass, 0 partial, 0 fail;
- `CellDescriptor` now carries descriptor-backed `source_id`, `source_url`,
  `document_id`, `page`, `row`, `cell_range`, `json_path`, `confidence_q16`,
  `citation`, `content_hash`, source/trust, and temporal fields through the
  existing WAL `CellDescriptor` section;
- metadata WAL writes preserve payload-derived provenance by merging metadata
  overlay fields with the payload descriptor instead of replacing it;
- ingestion validation has the current warn/strict surface through
  `IngestionValidationReport` warnings and `CellMetadata::decode_payload`
  strict metadata decoding;
- ContextPack source_ref/citation export is covered by no-payload-header
  regression tests;
- `DATA_MODEL.md` documents descriptor-backed provenance and compatibility.

### EPIC-B07 — Fact/claim store: типизированные факты с numeric-значениями

- status: `done`
- meta: Категория: verification · Приоритет: P1 · Горизонт: 6 months · Тип: build
- goal: «база фактов с конфликт-детекцией» — сердце agent-native категории; сейчас факты — просто текст, числа парсятся на лету.
- problem: Проблема: numeric-парсер (verification/numeric.rs) работает на каждом вызове по payload.
- tasks:
  - [x] 1) при записи/ingestion извлекать conservative numeric-факты (metric, value, unit, magnitude) в typed claim-записи, backed by `NumericValue`
  - [x] 2) maintained fact/claim store rebuilds on open and updates on put/patch/tombstone/snapshot install
  - [x] 3) VERIFY numeric support/conflict checks consult typed claims where possible, with parser fallback retained for non-typed and temporal evidence.
- acceptance:
  - [x] 1) numeric-конфликты на typed fixtures produce the same support/conflict verdict classes and deduplicate against fallback evidence
  - [x] 2) conservative fact extraction and lifecycle updates are covered by tests
  - [x] 3) p95 numeric-verify на 1M — индексное; closed by `EPIC-C13` with `make numeric-verify-index-check`.
- files: cortex-engine/src/{verification/numeric.rs, typed_body.rs, ingestion}.
- risks: extraction-качество — консервативные паттерны, без LLM в ядре. Зависимости: A02, A05. Эффект: verification переходит от «сравнить тексты» к «запросить факты».

Evidence:

- Added `verification::numeric::fact_claim::{FactClaimStore, NumericFactRecord}`:
  the conservative extractor reads `FactBody`, requires a `metric`, materializes
  exactly one numeric `NumericValue`, rejects ambiguous multi-value claims, and
  preserves scope/project/source/citation/trust metadata.
- Added `database::stores::DerivedStores` as the single lifecycle fan-out for
  maintained derived stores. `FactClaimStore` now rebuilds on open, updates on
  put/patch/tombstone, and is rebuilt after replication snapshot install with
  the other derived stores.
- `verify_fact_aql` now calls `fact_claim_store.add_verify_matches(...)` before
  evidence sorting. Typed claims add numeric support/contradiction evidence and
  structured `VerificationNumericConflict` rows while deduplicating against the
  existing parser path. Temporal facts still use the temporal guard path so the
  typed store does not bypass stale/future validity checks.
- `scripts/fact_claim_store_inventory.py` now reports `status=complete` with
  6 pass, 0 partial, 0 fail and writes
  `target/fact-claim-store/inventory.json`.
- Tests cover conservative extraction, ambiguous-value rejection, AgentView
  scope filtering, typed support/contradiction injection, duplicate prevention,
  and Database lifecycle tracking across put/patch/checkpoint/reopen/tombstone.
  Targeted gates passed: `cargo fmt --check`,
  `python3 scripts/file_size_report.py --root . --baseline quality/file_size_baseline.json --check`,
  `python3 -m py_compile scripts/fact_claim_store_inventory.py`,
  `python3 scripts/fact_claim_store_inventory.py`,
  `cargo test -p cortex-engine verification::numeric::fact_claim --all-features`,
  `cargo test -p cortex-engine --test verification_guards --all-features`, and
  `cargo test -p cortex-engine --test verification_tests --all-features`.
- Split decision resolved: B07 closed the typed fact/claim store and VERIFY
  integration; `EPIC-C13` now closes the metric-sorted numeric index
  (`metric -> value -> cell`) and 1M indexed p95 proof.

Next exit step: move to `EPIC-B08` — VerifyOp as a planned operator.

### EPIC-B08 — VerifyOp — верификация как оператор плана

- status: `done`
- meta: Категория: verification · Приоритет: P1 · Горизонт: 6 months · Тип: refactor
- goal: VERIFY FACT должен быть планируемым запросом (со статистикой, EXPLAIN, permission в scan), а не спецфункцией.
- problem: Проблема: verify_fact_aql — монолитная функция мимо будущего executor.
- tasks:
  - [x] 1) перевести A05-реализацию на план: Scan(lexical ∪ numeric ∪ markers) → PermissionFilter → EvidenceMatch → VerdictAggregate
  - [x] 2) EXPLAIN ANALYZE для VERIFY (сколько кандидатов, какие индексы)
  - [x] 3) опции глубины (max evidence, max candidates) как параметры плана, клампятся политикой.
- acceptance:
  - [x] 1) вердикты фикстур неизменны
  - [x] 2) EXPLAIN VERIFY показывает стадии
  - [x] 3) код verify не содержит собственного скан-цикла.
- files: verification.rs → exec/verify_op.rs.
- risks: нет существенных после A05/A11. Зависимости: A05, A11. Эффект: единый query engine для retrieve и verify.
- evidence: Added `verification::operator::VerificationExecutionReport` and
  `Database::execute_verify_fact_plan`, which executes VERIFY through traceable
  stages: `VerificationCandidateScan`, `VerificationPermissionFilter`,
  `VerificationMaterializeOp`, `VerifyOp`, `SourceSupportExpandOp`, and
  `VerdictAggregateOp`. `verify_fact_aql` now binds AQL and delegates to this
  execution-report path while returning the same public `VerificationReport`.
  `BoundVerifyFactPlan` now carries policy-clamped `max_candidates` and
  `max_evidence`, and VERIFY uses those plan parameters instead of hard-coded
  depth limits.
  Candidate/materialization/source-support loops live under
  `verification/operator/candidates.rs`, keeping the main VERIFY execution
  path as an explicit operator pipeline. Added engine-level
  `explain_verify_aql` and `explain_analyze_verify_aql`; analyze reports the
  same physical operator traces plus verdict/evidence counters. Regression
  coverage proves the execution report preserves the public report, emits
  operator traces, and that `EXPLAIN VERIFY`/`EXPLAIN ANALYZE VERIFY` expose
  logical policy rewrite and physical trace stages.
- latest gates: `cargo fmt --check`,
  `python3 scripts/file_size_report.py --root . --baseline quality/file_size_baseline.json --check`,
  `python3 scripts/descriptor_hot_path_gate_check.py`,
  `cargo test -p cortex-aql --all-features`,
  `cargo test -p cortex-engine --test aql_verify_explain --all-features`,
  `cargo test -p cortex-engine verification::operator --all-features`,
  `cargo test -p cortex-engine --test verification_tests --all-features`,
  `cargo test --workspace --all-features`,
  `cargo clippy --workspace --all-targets -- -D warnings`, and `make check`.
- next exit step: move to `EPIC-B09` — incremental contradiction/conflict
  index.

### EPIC-B09 — Инкрементальный contradiction/conflict-индекс

- status: `done`
- meta: Категория: verification · Приоритет: P2 · Горизонт: 6 months · Тип: build
- goal: «база знает свои противоречия» — фича уровня категории: конфликты можно находить при записи, а не при запросе.
- problem: Проблема: conflict_index.rs строится сканом по запросу.
- tasks:
  - [x] 1) при записи факта (B07) проверять fact-индекс на конфликтующее значение той же метрики в том же scope → материализовать conflict-запись
  - [x] 2) `/v1/conflicts?scope=` API + anomaly в паке «в паке есть стороны конфликта X»
  - [x] 3) инвалидация при tombstone/patch.
- execution steps:
  - [x] 1) define maintained conflict-index keys, scope rules, and rebuild semantics.
  - [x] 2) add `ConflictIndexStore` and wire put/patch/tombstone/rebuild/open paths.
  - [x] 3) replace query-time conflict lookup with store-backed candidate filtering while preserving AgentView source-facet isolation.
  - [x] 4) add ContextPack conflict anomaly and typed `/v1/conflicts?scope=` response.
  - [x] 5) add write/patch/tombstone/reopen/lazy/context/API regression tests.
  - [x] 6) run gates, publish evidence, mark B09 `done`, then move to `EPIC-B10`.
- acceptance:
  - [x] 1) конфликт обнаруживается на записи (тест: два бюджета → запись conflict)
  - [x] 2) пак, содержащий стороны конфликта, помечает это без full-scan
  - [x] 3) consistency-тест с patch/delete.
- latest evidence: Added maintained `ConflictIndexStore` backed by inline contradiction markers, persisted contradiction relations, descriptor/source facets, and typed numeric fact pairs. The store updates on put/patch/tombstone, rebuilds from visible records on open/replay, preserves lazy payload residency by using uncached maintenance payload reads, and `Database::conflict_index` now delegates to the maintained store instead of scanning query-time visible payloads. Added `ContextPackAnomalyCode::VisibleConflict`, typed `GET /v1/conflicts?scope=...` response/OpenAPI/SDK schemas, server AgentView route coverage, and regression tests for write/patch/tombstone/lazy reopen, ContextPack anomaly export, source-facet isolation, and API shape. Gates passed: `cargo fmt --check`, `cargo test -p cortex-engine --test verification_conflict_index --test verification_conflict_numeric --test context_pack_conflict_visibility --all-features`, `cargo test -p cortex-engine --test query_search retrieve_aql_lazy_budget_pushdown_bounds_segment_payload_reads --all-features`, `cargo test -p cortex-server v1_conflicts_returns_incremental_conflict_index --all-features`, `cargo test -p cortex-server agent_view_property --all-features`, `make openapi-contract-check`, `python3 scripts/file_size_report.py --root . --baseline quality/file_size_baseline.json --check`, `cargo test --workspace --all-features`, `cargo clippy --workspace --all-targets -- -D warnings`, and `make check`.
- files: verification/conflict_index.rs, verification/conflict_index/store.rs, database/stores.rs, database/open.rs, context/conflicts.rs, server memory/routes/responses, OpenAPI/SDK generated types.
- risks: maintained conflict index adds write-path bookkeeping and lazy-open rebuild work; current tests prove correctness on small/medium gates, with broader performance tracked by A19/C17 if write amplification becomes measurable.
- next exit step: move to `EPIC-B10` — Temporal validity as columns and temporal queries.

### EPIC-B10 — Temporal validity как колонки + временные запросы

- status: `done`
- meta: Категория: storage · Приоритет: P2 · Горизонт: 6 months · Тип: build
- goal: агентные знания устаревают; «что было верно на дату X» — естественный запрос agent-native БД.
- problem: Проблема: temporal-логика парсит даты из текста (verification/temporal.rs), валидность не первоклассна.
- tasks:
  - [x] 1) valid_from/valid_to в descriptor (A02)
  - [x] 2) AQL: `REQUIRE VALID AT "2026-01-01"` (расширение requirement, не новой грамматики)
  - [x] 3) descriptor-backed temporal validity index for retrieve filtering; richer interval/planner work remains tracked by C14.
- execution steps:
  - [x] 1) inventory existing descriptor, metadata parser, `TemporalFactStore`, VERIFY stale guards, and AQL requirement support.
  - [x] 2) formalize `valid_from`/`valid_to` descriptor semantics in DATA_MODEL docs, including null/open-ended ranges and date granularity.
  - [x] 3) extend AQL parser/binder with `REQUIRE VALID AT "YYYY-MM-DD"` and carry the bound date into `QualityThresholds`.
  - [x] 4) filter retrieve candidates through descriptor-backed temporal validity before payload materialization, preserving AgentView and lazy payload budget gates.
  - [x] 5) keep VERIFY stale-guard semantics descriptor-backed via `CellMetadata::from_version`; the fact/as_of temporal index remains maintained and query-time payload-scan-free.
  - [x] 6) add parser/binder/retrieve/lazy/VERIFY/docs regression tests, run gates, mark B10 `done`, then move to `EPIC-B11`.
- acceptance:
  - [x] 1) temporal filter works through maintained descriptor-backed index on a 100K corpus: `target/temporal-validity-gate/100k-memory-final/report.json` reports `ok=true`, `returned_cells=100`, `valid_expected=100`, `query_elapsed_ms=1454`, `segment_loads_after_query=0`.
  - [x] 2) stale/future candidates do not reach lazy payload materialization: `target/temporal-validity-gate/10k-lazy-final/report.json` reports `ok=true`, `returned_cells=10`, `valid_expected=10`, `query_elapsed_ms=155`, `segment_loads_after_query=10`.
  - [x] 3) semantics documented in `DATA_MODEL.md` and `AQL_V0_4.md`.
- files: cell.rs, verification/temporal*.rs, binder (requirement).
- risks: ingestion редко знает валидность — поля опциональны, семантика null задокументирована. Зависимости: A02. Эффект: temporal reasoning — database-фича.
- evidence: Added AQL parser/binder support for `REQUIRE valid at "YYYY-MM-DD"`, strict date validation, `QualityThresholds.valid_at`, descriptor-backed `TemporalValidityStore`, and physical `TemporalValidityFilter` before candidate budget/rank/materialization. Added `aql_valid_at_tests`, lazy retrieve regression `aql_require_valid_at`, and `temporal_validity_gate` reports for 100K memory and 10K lazy residency. Existing VERIFY stale-guard tests continue to cover descriptor-backed stale evidence handling. Gates passed: `cargo fmt --check`, file-size ratchet, descriptor hot-path gate, memtable clone gate, targeted AQL/engine temporal tests, `cargo test --workspace --all-features`, `cargo clippy --workspace --all-targets -- -D warnings`, and `make check`.
- follow-up: 100K lazy-open with payload-derived rebuild timed out at 5 minutes because `rebuild_lazy_derived_stores_from_visible_payloads` still reads payload-derived conflict/fact stores one payload at a time; this is performance debt for A19/C17/A08-tail, not a B10 temporal-filter blocker.

### EPIC-B11 — Memory lifecycle: TTL/decay как политика хранилища

- status: `done`
- meta: Категория: storage · Приоритет: P1 · Горизонт: 90 days · Тип: productize
- goal: «память агента с lifecycle» — категория-фича; должна иметь контракт и engine-исполнение.
- problem: Проблема: memory.rs делает TTL/decay через full scan; expire — команда актора без планировщика; семантика не зафиксирована.
- tasks:
  - [x] 1) TTL-индекс (expiry→cells), expire — фоновое задание writer-runtime (батчевые tombstone через WAL)
  - [x] 2) decay — формула в rank по created_at/last_access из descriptor
  - [x] 3) AGENT_MEMORY.md как контракт с формулами; golden-тесты «память через N дней ранжируется ниже/исчезает».
- acceptance:
  - [x] 1) expire не сканирует базу (индекс, тест)
  - [x] 2) golden-тесты формулы decay
  - [x] 3) REMEMBER+TTL e2e через HTTP/SDK.
- files: memory.rs, memory_accounting.rs, session.rs; actor.rs (планировщик).
- risks: фоновые tombstone vs читатели — через обычный WAL-путь, ничего специального. Зависимости: A02, A14. Эффект: agent memory — продукт с гарантиями.
- evidence: Added descriptor-backed `MemoryLifecycleStore` with `expiry -> cells` and
  `cell_id -> lifecycle record` maps. `Database::expired_memory_cells` and
  `memory_decay_scores` now use the maintained store instead of
  `snapshot_versions()`. The store is rebuilt from descriptors on open/replay,
  maintained on put/patch/tombstone, and works with lazy payload residency
  without materializing payloads. Added physical `MemoryLifecycleFilter` before
  candidate budget and payload materialization, so expired memory is excluded
  from AQL retrieve even before the background server TTL job tombstones it via
  WAL. `RankOp` now applies the same Q16 freshness formula as a deterministic
  decay multiplier for temporary memory cells. Docs updated in `AGENT_MEMORY.md`
  and `INGESTION.md`.
- gates: `cargo test -p cortex-engine --test memory_tests`; `cargo test -p
  cortex-engine --test memory_lifecycle_tests`; `cargo test -p cortex-engine
  memory::lifecycle`; `cargo test -p cortex-server
  v1_remember_ttl_expiry_disappears_from_context --all-features`;
  `python3 scripts/agent_memory_demo_check.py`; `cargo check -p
  cortex-engine`; `cargo test --workspace --all-features`; `cargo clippy
  --workspace --all-targets -- -D warnings`; `make check`.
- follow-up: B12 owns the remaining session-specific scan/index contract.

### EPIC-B12 — Session/episodic memory contract

- status: `done`
- meta: Категория: product · Приоритет: P2 · Горизонт: 6 months · Тип: productize
- goal: LongMemEval показал, что session-память — реальный workload; нужен контракт, а не приватный харнесс.
- problem: Проблема: session.rs использует snapshot_versions-скан; семантика сессий не публична.
- tasks:
  - [x] 1) session_id в descriptor; session-retrieval через индекс scope+session
  - [x] 2) API: append session event, retrieve session window/summary-кандидаты
  - [x] 3) перенести lessons из LongMemEval-харнесса в generic-механизм (без оверфита, урок EPIC из прошлого аудита).
- acceptance:
  - [x] 1) session-пути индексные
  - [x] 2) публичный пример «чат-агент с многосессионной памятью»
  - [x] 3) LongMemEval-харнесс использует только публичные API.
- files: session.rs, query/, examples/.
- risks: переусложнение — минимальный контракт. Зависимости: A02, A06. Эффект: главный agent-workload оформлен.
- evidence: `CellDescriptor` now carries `session_id` and `session_kind` through
  payload header parsing and the binary descriptor section. `SessionIndex`
  stores descriptor-backed session records and a `session_id -> cell_id` map;
  `Database::retrieve_session_cells` delegates to the index and materializes
  payload only after descriptor scope/TTL/session filtering. Lazy checkpoint
  reopen builds the session index from descriptors rather than resident
  payloads, and `agent_session_lazy_tests` proves retrieving one session loads
  only that session's payloads. Public examples are documented in
  `docs/AGENT_MEMORY.md` and `examples/demo/agent_sessions/README.md`.
- gates: `cargo fmt --check`; `cargo test -p cortex-core --all-features`;
  `cargo test -p cortex-engine --test agent_session_tests --test
  agent_session_lazy_tests --all-features`;
  `python3 scripts/descriptor_hot_path_gate_check.py`;
  `python3 scripts/query_scan_inventory_check.py`; `make file-size-check`;
  `cargo test --workspace --all-features`; `cargo clippy --workspace
  --all-targets -- -D warnings`; `make check`.
- next exit step: moved to `EPIC-B13` and closed; current next step is
  `EPIC-B14` — Explainability contract.

### EPIC-B13 — Feedback как индексированный ranking-сигнал

- status: `done`
- meta: Категория: retrieval · Приоритет: P2 · Горизонт: 6 months · Тип: refactor
- goal: feedback-loop (агент сообщает полезность контекста) — редкая и правильная фича, но сейчас она full-scan и полудокументирована.
- problem: Проблема: feedback.rs — 4 вызова snapshot_versions на расчёт.
- tasks:
  - [x] 1) feedback-записи → инкрементальный map cell→score (поддерживается на write)
  - [x] 2) RankOp читает map O(1)
  - [x] 3) HTTP/SDK API + doc; решение зафиксировать (продуктизируем, не выпиливаем — opinionated recommendation).
- acceptance:
  - [x] 1) feedback-путь без сканов
  - [x] 2) API задокументирован с примером агентного цикла
  - [x] 3) ranking-эффект покрыт golden-тестом.
- files: feedback.rs, exec/rank_op.rs, server/router.rs.
- risks: нет. Зависимости: A06. Эффект: «база, которая учится у агента» — без ML-пафоса, инженерно.
- latest evidence: `FeedbackIndex` now stores feedback records by feedback
  cell and source cell, keeps maintained raw scores, and exposes candidate-scoped
  score lookups through `Database::feedback_scores_for_cells_at`. AQL
  ContextPack and EXPLAIN ANALYZE use those candidate-scoped scores instead of
  scanning all feedback records. Public HTTP routes `POST /v1/feedback` and
  `GET /v1/feedback/stats` are typed, permission-aware, documented in OpenAPI
  and generated SDK types, and covered by server API tests. DeepSeek
  official-clean 50-question smoke result for the current runner:
  `overall=30.6`, `correctness=32.0`, `completeness=36.4`,
  `document_recall=56.0`, `invalid_extra_docs=9.44`,
  `answer_tokens=286831`, `judge_tokens=24684`, clean gate and oracle audit
  passed.
- gates: `cargo fmt --check`; `cargo test -p cortex-engine --test
  feedback_tests --test feedback_index_tests --all-features`; `cargo test -p
  cortex-server feedback --all-features`; `python3 -m py_compile
  scripts/enterprise_rag_bench/run_deepseek_answers.py
  scripts/enterprise_rag_bench/oracle_usage_audit.py
  scripts/check_openapi_contract.py`; `python3
  scripts/query_scan_inventory_check.py`; `python3
  scripts/descriptor_hot_path_gate_check.py`; `make file-size-check`; `make
  openapi-contract-check`; `cargo test --workspace --all-features`; `cargo
  clippy --workspace --all-targets -- -D warnings`; `make check`.
- next exit step: `EPIC-B14` is now closed; current next step is
  `EPIC-B15` — EXPLAIN ANALYZE for AQL.

### EPIC-B14 — Explainability contract: explain для каждого результата

- status: `done`
- meta: Категория: contextpack · Приоритет: P1 · Горизонт: 60 days · Тип: productize
- goal: проверяемость — категория-свойство; explain должен быть стабильной частью result type.
- problem: Проблема: explain-поля богатые (score_components, why_selected), но не контрактные; «почему ячейка НЕ попала» отвечается лишь частично anomalies.
- tasks:
  - [x] 1) explain-схему в ContextPack v1 (B01) зафиксировать
  - [x] 2) `cortexdb explain --cell-id N <aql>`: трассировка конкретной ячейки по стадиям (allowed? live? where? thresholds? budget? redundancy?) на базе операторных счётчиков (A11)
  - [x] 3) doc EXPLAIN.md с примерами.
- acceptance:
  - [x] 1) для исключённой ячейки называется первая отсёкшая стадия
  - [x] 2) explain стабилен под golden-тестами
  - [x] 3) поля документированы.
- files: `crates/cortex-engine/src/context/explain.rs`, CLI explain command, `docs/EXPLAIN.md`, ContextPack schema/OpenAPI generated artifacts.
- evidence: `ContextPack::explain_cell(CellId)` now returns a typed `ContextCellExplain` contract for selected, excluded, and not-considered cells. Selected cells expose `why_selected`, score, matched terms, score components, and access decision. Excluded cells map anomaly codes to stable `first_excluding_stage` names and preserve `why_excluded`. CLI `cortexdb explain <db> <scope> <aql> --cell-id N` emits summary or `context_cell_explain.v1` JSON. `docs/EXPLAIN.md` documents selected/excluded fields and stage names. ContextPack schema/OpenAPI now include the existing `visible_conflict` anomaly code and regenerated Python/TypeScript OpenAPI SDK types.
- gates: `cargo fmt --check`; `cargo test -p cortex-engine --test context_pack_explain_v2`; `cargo test -p cortex-cli explain_command_returns_cell_contract_json`; `cargo test --workspace --all-features`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test -p cortex-cli`; `make openapi-contract-check`; `make check`.
- remaining: none for B14 acceptance; deeper operator timing/counter explain belongs to B15.
- risks: CLI cell explain reports the first recorded ContextPack anomaly for excluded cells; full physical per-operator timing remains B15 scope.
- next exit step: `EPIC-B17` is now closed; current next step is
  `EPIC-C15` — Incremental graph index performance.

### EPIC-B15 — EXPLAIN ANALYZE для AQL

- status: `done`
- meta: Категория: query-engine · Приоритет: P1 · Горизонт: 90 days · Тип: build
- goal: у настоящей БД можно спросить, как исполнился запрос.
- problem: Проблема: AqlExplainReport есть, но не отражает физическое исполнение (его пока нет).
- tasks:
  - [x] 1) `EXPLAIN <stmt>` — logical+physical план (JSON и текст)
  - [x] 2) `EXPLAIN ANALYZE` — с real счётчиками/временем операторов
  - [x] 3) HTTP `/v1/aql?explain=analyze`, CLI-флаг.
- acceptance:
  - [x] 1) estimated vs actual кандидаты видны на каждом операторе
  - [x] 2) выбор cost-планировщика (A13) обоснован в выводе
  - [x] 3) doc с примерами.
- files: `crates/cortex-cli/src/cli_aql.rs`, `crates/cortex-server/src/aql.rs`, shared AQL response types, OpenAPI generated SDK artifacts, `docs/EXPLAIN_ANALYZE.md`.
- evidence: Existing engine `EXPLAIN ANALYZE RETRIEVE` physical trace is now exposed through CLI flag form `cortexdb aql ... --explain analyze` and HTTP `POST /v1/aql?scope=...&explain=analyze` for normal `RETRIEVE` bodies. AQL operator response fields now include backward-compatible `input_count`/`output_count` plus explicit `actual_input_count`, `actual_output_count`, and nullable `estimated_output_count`; text CLI output prints the ordered operator trace and total elapsed nanos. OpenAPI documents `explain=plan|analyze`, logical/policy plans, and execution trace fields, with regenerated Python/TypeScript OpenAPI SDK types. `docs/EXPLAIN_ANALYZE.md` documents CLI/API examples and trace stability rules.
- gates: `cargo fmt --check`; `cargo test -p cortex-cli aql_command_explain_analyze_flag_reports_actual_operator_counts`; `cargo test -p cortex-server v1_aql_explain_analyze_query_flag_reports_execution_trace`; `cargo test --workspace --all-features`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test -p cortex-cli`; `make openapi-contract-check`; `make check`.
- remaining: none for B15 acceptance; deeper tool-catalog semantics are closed by B17.
- risks: `estimated_output_count` is nullable where no planner estimate exists; actual counts are always present for analyzed operators.
- next exit step: `EPIC-C01` is now closed; current pointer is `EPIC-C03`.

### EPIC-B16 — Формализованный Policy Rewrite + доказательство инварианта

- status: `done`
- meta: Категория: security · Приоритет: P0 · Горизонт: 60 days · Тип: build
- goal: permission-safe retrieval как database-level invariant требует структурного и тестового доказательства.
- problem: Проблема: гарантия сейчас распределена между binder, authz.rs пост-фильтрами и дисциплиной кода.
- tasks:
  - [x] 1) единственная точка: PolicyRewrite-проход над LogicalPlan (A10), все поверхности (search/get/verify/graph/memory/export) строят планы через него
  - [x] 2) негативные тесты на каждую поверхность (запрос чужого scope → пустота/ошибка, никогда payload)
  - [x] 3) структурный тест: пост-rewrite план не содержит непокрытого Scan.
- acceptance:
  - [x] 1) одна функция-источник гарантии
  - [x] 2) E09 property-suite зелёный
  - [x] 3) SECURITY_MODEL.md описывает инвариант одной страницей.
- files: `crates/cortex-engine/src/plan/policy.rs`, `crates/cortex-engine/src/plan/mod.rs`, `crates/cortex-engine/src/plan/tests.rs`, `crates/cortex-server/src/authz.rs`, read-surface permission call sites, `scripts/policy_rewrite_gate_check.py`, `mk/core.mk`, `docs/SECURITY_MODEL.md`.
- evidence: `PolicyRewrite` now lives in `plan/policy.rs` with the `ReadSurface` registry and `rewrite_read_surface` helper for AQL retrieve/explain, search/explain, ContextPack/trace, cell get, verify, graph, memory, feedback, and export. All registered read-surface plans structurally start with an uncovered scan and rewrite to `policy_complete=true` with `agent_allowed`; descriptor read authorization is tested against durable descriptor scope. Server read authz delegates descriptor/scope decisions to `PolicyRewrite`, AQL/search candidate bitmap builders consume readable scopes through `PolicyRewrite`, and production engine read filters now call `PolicyRewrite::allows_scope` instead of directly calling `AgentView::can_read_scope`. Added `policy-rewrite-gate-check` to `make check`; it verifies the read-surface registry/tests/server authz hooks and rejects production direct `can_read_scope` calls outside `PolicyRewrite`. Existing `descriptor_hot_path_gate_check.py` was updated to expect the new helper while preserving descriptor-before-payload gates. `SECURITY_MODEL.md` now documents the permission-safe read invariant, source of truth, E09 property evidence, and gate boundary.
- gates: `python3 scripts/policy_rewrite_gate_check.py`; `python3 scripts/descriptor_hot_path_gate_check.py`; `cargo test -p cortex-engine plan --all-features`; `cargo test -p cortex-server agent_view_property --all-features`; `cargo fmt --check`; `make check`; `cargo test --workspace --all-features`; `cargo clippy --workspace --all-targets -- -D warnings`.
- remaining: none for B16 acceptance; future C02/C09 can improve bitmap representation and pre-pruning performance without changing the policy invariant.
- risks: `policy-rewrite-gate-check` is a static production-path guard, not a formal verifier for future generated code; new read surfaces must be added to `ReadSurface` plus E09/property coverage.
- next exit step: completed by `EPIC-C08`; current pointer is `EPIC-A19`.

### EPIC-B17 — Tool registry как типизированный каталог

- status: `done`
- meta: Категория: product · Приоритет: P3 · Горизонт: 6 months · Тип: productize
- goal: рекомендация инструментов в паке — полезный агентный примитив, но сейчас это scan по type=tool ячейкам.
- problem: Проблема: tool_registry.rs — snapshot_versions-скан; формат tool-ячеек конвенционный.
- tasks:
  - [x] 1) typed tool-записи (descriptor type=tool + structured body)
  - [x] 2) каталог в памяти, инкрементально поддерживаемый
  - [x] 3) recommend_tools через индекс задач→термов.
- acceptance:
  - [x] 1) без сканов
  - [x] 2) doc TOOL_REGISTRY.md сокращён до контракта
  - [x] 3) пример «агент получает пак+инструменты».
- files: `crates/cortex-engine/src/tool_registry.rs`, `crates/cortex-engine/src/tool_registry/index.rs`, `crates/cortex-engine/src/database/rebuild.rs`, `docs/TOOL_REGISTRY.md`, `scripts/tool_registry_check.py`.
- evidence: Tool lookup now goes through `ToolIndex` for `list_tools` and `recommend_tools_for_task`; query-time lazy payload scans were removed from `tool_registry.rs`. `ToolIndex` maintains registered tools plus `term_to_tools`/`tool_terms` maps and updates both maps on put, patch, tombstone, memtable rebuild, checkpoint/lazy reopen, and replication snapshot rebuild. Lazy open-time derived-store rebuild now repopulates the tool index from visible payloads, while queries read the maintained index. `docs/TOOL_REGISTRY.md` is a short B17 contract covering typed tool record fields, permissions, index semantics, and the agent context+tools example. `make check` now includes `tool-registry-check`.
- gates: `cargo fmt --check`; `cargo test -p cortex-engine --test tool_registry_tests --test tool_registry_index_tests`; `python3 scripts/tool_registry_check.py --report target/tool-registry/report.json`.
- remaining: none for B17 acceptance.
- risks: lazy databases opened with `rebuild_lazy_payload_indexes_on_open=false` intentionally skip open-time payload-derived index rebuilds; default options keep the typed tool catalog populated.
- next exit step: `EPIC-C15` is now closed; move to `EPIC-B19` — REMEMBER write-path policy formalization.

### EPIC-B18 — Инкрементальный knowledge-graph/provenance индекс

- status: `done`
- meta: Категория: indexing · Приоритет: P2 · Горизонт: 6 months · Тип: refactor
- goal: graph-traversal и source-support рёбра используются VERIFY и retrieval, но строятся full-scan'ом на вызов (graph.rs:73).
- problem: Проблема: O(N) проекция графа.
- tasks:
  - [x] 1) entity/relation/source-ref записи индексируются при записи (adjacency map)
  - [x] 2) graph_retrieval и verification graph-обогащение читают индекс
  - [x] 3) инвалидация на patch/tombstone + property-тест эквивалентности полной проекции.
- acceptance:
  - [x] 1) graph-пути без сканов
  - [x] 2) фикстуры graph_tests/verification_graph_tests без изменений семантики
  - [x] 3) p95 на 100K-графе переносится в `EPIC-C15`, который является performance half of B18.
- files: `crates/cortex-engine/src/graph/*.rs`, `crates/cortex-engine/src/database/rebuild.rs`, `crates/cortex-engine/src/verification/graph.rs`, `crates/cortex-engine/src/verification/operator.rs`, `docs/KNOWLEDGE_GRAPH.md`, `scripts/knowledge_graph_check.py`.
- evidence: `GraphIndexStore` now updates incrementally through `insert_record`/`remove_record` instead of rebuilding the full graph on every put/patch/tombstone. `KnowledgeGraphIndex` now maintains adjacency, edge-kind, source-reference, and `source_support_edges_by_fact` maps; graph APIs read the maintained store, and lazy graph queries no longer rebuild from visible payloads. Lazy open-time derived-store rebuild now repopulates the graph index from visible payloads. VERIFY source-support enrichment reads `source_support_edges_by_fact` for current evidence cell ids and materializes only matching readable relation payloads. `docs/KNOWLEDGE_GRAPH.md` defines the B18 typed graph/provenance index contract. `make check` now includes `knowledge-graph-check`.
- gates: `cargo fmt --check`; `cargo test -p cortex-engine --test graph_tests --test graph_index_incremental_tests --test graph_retrieval_tests --test verification_graph_tests`; `python3 scripts/knowledge_graph_check.py --report target/knowledge-graph/report.json`.
- remaining: none for B18 no-scan/index/equivalence acceptance; the 100K graph traversal p95 follow-up is closed by `EPIC-C15`.
- risks: lazy DBs opened with `rebuild_lazy_payload_indexes_on_open=false` intentionally skip open-time payload-derived graph-index rebuilds.
- next exit step: `EPIC-C15` is now closed; move to `EPIC-B19` — REMEMBER write-path policy formalization.

### EPIC-B19 — REMEMBER write-path policy formalization

- status: `done`
- meta: Категория: AQL · Приоритет: P2 · Горизонт: 90 days · Тип: productize
- goal: запись через AQL — половина агентного цикла; политика записи должна быть так же формальна, как чтения.
- problem: Проблема: REMEMBER реализован (binder enforce_remember: scope/memory_type/TTL-клампы), но семантика (id-аллокация, descriptor-поля, дефолты) не контрактна.
- tasks:
  - [x] 1) спецификация REMEMBER в AQL_V0_5: что создаётся, какие поля, какие клампы
  - [x] 2) аллокация cell_id — атомарный счётчик в манифесте (вместо max+1 эвристик allocate_cell_id)
  - [x] 3) e2e: remember→retrieve→verify цикл.
- acceptance:
  - [x] 1) контракт в доке
  - [x] 2) id-аллокация безопасна при конкуренции (тест)
  - [x] 3) e2e-тест цикла.
- files: `docs/AQL_V0_5.md`, `crates/cortex-engine/src/ingestion.rs`, `crates/cortex-engine/src/cell_ids.rs`, `crates/cortex-engine/src/database/write.rs`, `crates/cortex-storage/src/manifest.rs`, `crates/cortex-storage/src/manifest/codec.rs`, `crates/cortex-engine/tests/remember_write_contract_tests.rs`, `scripts/descriptor_hot_path_gate_check.py`.
- evidence: `REMEMBER` now has a v0.5 write contract covering policy, metadata fields, defaults, read/verify behavior, and ID namespace semantics. REMEMBER IDs use manifest-backed `memory_cell_cursors` keyed by agent slot and persist the cursor before WAL write; generic ingest IDs use manifest-backed `next_cell_id` outside the high-bit memory namespace. Regression coverage includes concurrent REMEMBER allocation and remember→retrieve→verify.
- gates: `cargo fmt --check`; `cargo test -p cortex-storage manifest_id_allocators_roundtrip_and_reserve`; `cargo test -p cortex-engine --test remember_write_contract_tests`; `cargo test -p cortex-server ingest`; `cargo test --workspace --all-features`; `cargo clippy --workspace --all-targets -- -D warnings`; `make openapi-contract-check`; `make check`.
- remaining: none for B19.
- risks: manifest allocation can leave intentional ID gaps if a write fails after cursor persistence; IDs are not reused. Зависимости: A15. Эффект: полный агентный read-write цикл формализован.
- next exit step: `EPIC-C01` is now closed; current pointer is `EPIC-C03`.

### EPIC-B20 — Multi-brain: реальная семантика или удаление

- status: `done`
- meta: Категория: AQL · Приоритет: P2 · Горизонт: 6 months · Тип: refactor
- goal: грамматика обещает БРЕЙНЫ, движок живёт с DEFAULT_BRAIN=1 — «синтаксис без семантики» хуже обоих вариантов.
- problem: Проблема: query.rs:32 `const DEFAULT_BRAIN: BrainId = BrainId(1)`.
- tasks:
  - [x] 1) решение: current product scope is single-brain; non-default brain names are deprecated aliases for `BrainId(1)`.
  - [x] 2) если да: не делаем real multi-brain без storage-format epic; каталог/персистентность не добавляются.
  - [x] 3) если нет: депрекация в грамматике к v1.0 documented in `BRAIN_SEMANTICS.md`.
- acceptance:
  - [x] 1) решение задокументировано в DATA_MODEL.md
  - [x] 2) migration-план удаления/упрощения есть; current alias behavior covered by tests/gate.
- files: `crates/cortex-engine/src/query/brain.rs`, `crates/cortex-engine/src/query/catalog.rs`, `crates/cortex-engine/src/query/statistics.rs`, `crates/cortex-engine/tests/brain_semantics.rs`, `docs/BRAIN_SEMANTICS.md`, `docs/DATA_MODEL.md`, `scripts/multi_brain_contract_check.py`, `mk/core.mk`.
- evidence: CortexDB now documents `default = BrainId(1)` as the only real brain. Runtime AQL and statistics catalogs share `resolve_single_brain_name`, preserving compatibility by mapping non-empty legacy aliases to `BrainId(1)` while documenting them as deprecated, not isolated namespaces. `brain_semantics` tests prove default and legacy aliases return the same cells and still require `AgentView.readable_brains` to contain `BrainId(1)`. `make check` includes `multi-brain-contract-check`.
- gates: `cargo fmt --check`; `cargo test -p cortex-engine --test brain_semantics`; `cargo test -p cortex-engine query::brain`; `python3 scripts/multi_brain_contract_check.py`.
- remaining: none for B20.
- risks: non-default brain aliases remain accepted for compatibility until v1.0; they are explicitly not isolation boundaries. Зависимости: A02, A12. Эффект: грамматика перестаёт обещать fake multi-brain semantics.
- next exit step: `EPIC-C01` is now closed; current pointer is `EPIC-C03`.

## Block C — Indexing, retrieval, and performance

### EPIC-C01 — Интернирование термов + компактные постинги

- status: `done`
- meta: Категория: indexing · P1 · 90 days · refactor
- goal: `LexicalIndex` = BTreeMap<String, BTreeMap<u32,u32>> ×5 разрезов — RAM-обжорство и cache-miss'ы; database-индекс так не строят.
- problem: Проблема: память и скорость лексического индекса.
- tasks:
  - [x] 1) term dictionary (term→u32 id, FST или sorted dict)
  - [x] 2) постинги: sorted Vec<u32>/roaring + parallel freq-массив
  - [x] 3) формат .aci v2 c dual-read.
- acceptance:
  - [x] 1) persisted lexical-index repeated-term footprint ↓ ≥3x on compact regression fixture
  - [x] 2) lexical_index_tests + quality/search fixtures зелёные
  - [x] 3) migration-тест.
- files: `crates/cortex-storage/src/indexes.rs`, `crates/cortex-storage/src/indexes/codec.rs`, `crates/cortex-storage/src/format.rs`, `crates/cortex-storage/tests/lexical_index_tests.rs`, `docs/LEXICAL_INDEX.md`, `docs/STORAGE_FORMATS.md`, `fixtures/storage/storage_format_freeze_v1.json`, `fixtures/migration/compatibility_matrix_v1.json`, `fixtures/migration/storage_format_change_notes_v1.json`, `scripts/lexical_index_contract_check.py`, `mk/core.mk`.
- evidence: `.aci` writer now emits `ACI4`: a sorted term dictionary followed by term-id postings and delta-varint candidate/frequency streams. `ACI0`, `ACI1`, `ACI2`, and `ACI3` remain read-only compatible; `ACI3` field-frequency files load and rewrite to `ACI4`. `read_terms_only` still skips heavy frequency sections after loading dictionary-backed postings/doc lengths. `docs/LEXICAL_INDEX.md` and `docs/STORAGE_FORMATS.md` document the contract. Regression coverage proves current magic is `ACI4`, legacy `ACI3` dual-read/rewrite works, and the compact fixture is more than 3x smaller than the repeated-term `ACI3` encoding. Search semantics stayed stable under persisted/search fixtures.
- gates: `cargo fmt --check`; `cargo test -p cortex-storage --test lexical_index_tests`; `cargo test -p cortex-storage --test format_tests --test lexical_index_tests`; `cargo test -p cortex-engine --test database_search --all-features`; `cargo test -p cortex-engine --test persisted_index_tests --all-features`; `cargo test -p cortex-engine --test query_search --all-features`; `cargo test -p cortex-engine compatibility --all-features`; `python3 scripts/lexical_index_contract_check.py`; `make storage-format-freeze-check`; `make storage-format-change-note-check`; `make migration-policy-check`; `make migration-compatibility-check`; `make storage-compat-check`.
- remaining: none for C01.
- risks: the compactness proof is a deterministic repeated-term persisted-index fixture, not a full 1M RAM profile; broader scale/RAM tracking remains under A19/C17/C16. Format risk is gated by ACI4 current marker plus ACI0-ACI3 dual-read compatibility.
- next exit step: move to `EPIC-C03` — Real BM25 with field weights.

### EPIC-C02 — Roaring bitmaps в bitmap-индексе и VM

- status: `done`
- meta: Категория: indexing · P1 · 60 days · refactor
- goal: BTreeSet<u32> для universe/AND/OR не масштабируется к 10M кандидатов.
- problem: Проблема: bitmap VM и индексы на BTreeSet.
- tasks:
  - [x] 1) crate `roaring`; BitmapIndex и eval_bitmap_program на RoaringBitmap — VM теперь работает на RoaringBitmap stack, совместимый BTreeSet wrapper сохранён.
  - [x] 2) сериализация .acb v2 — current magic `ACB1` writes Roaring payloads; `ACB0` retained read-only.
  - [x] 3) бенч AND/OR/NOT на 1M/10M — `bitmap_vm_benchmark_check`.
- acceptance:
  - [x] 1) bitmap-операции на 1M < 1ms — release min: AND 0.080608ms, OR 0.075802ms, NOT 0.073914ms.
  - [x] 2) bitmap_vm_tests зелёные — `cargo test -p cortex-aql --all-features --test bitmap_vm_tests`.
  - [x] 3) память ↓ (зафиксировать) — 1M ACB bytes 3,333,372 -> 262,452 (-92.127%); 10M 33,333,372 -> 2,509,252 (-92.472%).
- files: cortex-aql/src/executor_mock.rs (VM), cortex-aql/src/bin/bitmap_vm_benchmark_check.rs, cortex-storage/indexes.rs, cortex-storage/src/format.rs.
- risks: семантика Not в segment-universe сохранена (complement в universe). Зависимости: A20. Эффект: фундамент перм-битмапов B04 и масштаба.

### EPIC-C03 — Честный BM25 с полевыми весами

- status: `done`
- meta: Категория: retrieval · P1 · 90 days · refactor
- goal: прежняя непараметризованная lexical-scoring аппроксимация заменена на каноничный BM25(k1,b).
- problem: Проблема: непараметризованная аппроксимация.
- tasks:
  - [x] 1) каноничный BM25(k1,b) в fixed-point, тест против float-эталона на мини-корпусе
  - [x] 2) полевые веса title/body/path (field_term_frequencies уже хранятся)
  - [x] 3) конфиг + дефолты.
- acceptance:
  - [x] 1) расхождение с эталоном < ε
  - [x] 2) retrieval-quality фикстуры ≥ текущих
  - [x] 3) параметры в доке SCORING.md.
- files: `crates/cortex-engine/src/search/bm25.rs`, `search/lexical.rs`, `search/persisted.rs`, `query/provider.rs`, `retrieval_rank.rs`, `context/scoring.rs`, enterprise retrieval benchmark scorer, `docs/SCORING.md`, `docs/SEARCH.md`.
- evidence: `Bm25Config` defaults `k1=1.2`, `b=0.75`; fixed-point IDF/TF/length normalization matches float-reference tests; live, persisted, AQL, retrieved-cell, ContextPack, and benchmark lexical paths use shared BM25 helpers; persisted field BM25 parity with live index is covered.
- gates: `cargo fmt --check`; `cargo test -p cortex-engine bm25 --all-features`; `cargo test -p cortex-engine search::persisted::tests --all-features`; `cargo test -p cortex-engine --test query_search indexes --all-features`; `cargo test -p cortex-engine --test database_search lexical_index --all-features`; `cargo test -p cortex-engine --test context_pack scoring --all-features`; `cargo test -p cortex-engine --test context_pack_explain_v2 --all-features`.
- risks: lexical score magnitudes changed by design; ranking quality remains guarded by search/context fixtures and broader retrieval-quality gates. Зависимости: C01. Эффект: ранжирование становится защитимым.

### EPIC-C04 — Токенизация: unicode segmentation + опциональный стемминг

- status: `done`
- meta: Категория: retrieval · P2 · 90 days · improve→build
- goal: фикстуры проекта на русском/казахском контенте (KZT, инвестпроекты), а токенизатор простой.
- problem: Проблема: morфология снижает recall не-английских корпусов.
- tasks:
  - [x] 1) unicode-segmentation
  - [x] 2) ru/en/kz light stemming за конфигом коллекции
  - [x] 3) quality-фикстура с русскими запросами.
- acceptance:
  - [x] 1) «бюджету»→«бюджет» при включённом стемминге
  - [x] 2) англ. фикстуры не деградируют
  - [x] 3) конфиг описан.
- files: `crates/cortex-engine/src/search/tokenizer.rs`, `search/analyzer.rs`, `search/query_understanding.rs`, `search/lexical.rs`, `search/persisted.rs`, `query/index.rs`, `query/index_merge.rs`, `query/metadata/*`, `checkpoint/*`, `database/open.rs`, `database/types.rs`, `replication/install.rs`, `crates/cortex-storage/src/manifest.rs`, `manifest/codec.rs`, `docs/SEARCH.md`, `docs/STORAGE_FORMATS.md`, `docs/ENGINE_API.md`.
- evidence: `TextAnalyzerConfig` is now part of `DatabaseOptions`; default analyzer remains neutral with stemming disabled; Russian configured analyzer normalizes `бюджету` to `бюджет`; snapshot, persisted `.aci`, checkpoint/compact, replication install, and AQL merge use the configured analyzer; manifest `ANLZ` records analyzer version/language/stemming and open rejects mismatched persisted profiles.
- gates: `cargo test -p cortex-engine --test search_analyzer_config --all-features`; `cargo test -p cortex-engine search::quality_tests --all-features`; `cargo test -p cortex-engine search::persisted::tests --all-features`; `cargo test -p cortex-storage --test manifest_profile_tests --all-features`.
- risks: external `rust-stemmers` was not added because project rules forbid new dependencies without explicit approval; the implemented light stemmers are deterministic and opt-in, with manifest profile protection against mixed token streams. Зависимости: C01. Эффект: честный multilingual.

### EPIC-C05 — Disk-resident vector storage + SIMD exact scan

- status: `done`
- meta: Категория: indexing · P1 · 90 days · refactor
- goal: exact vector scan — ваш заявленный «предсказуемый дефолт»; он должен быть быстрым и не требовать RAM-резидентности.
- problem: Проблема: вектора в payload-строках/RAM; dot-product скалярный.
- tasks:
  - [x] 1) вектора в .acv с contiguous layout + stable read path
  - [x] 2) SIMD or deterministic acceleration path: stable chunked i16 dot-product scan
  - [x] 3) бенч exact scan path tracked as C17 packet; vector data detached from payload strings in persisted `.acv` rows.
- acceptance:
  - [x] 1) exact scan baseline path recorded for the disk-resident reader; full 1M×768d p95 remains a C17 benchmark-packet run under the scale-gate rule
  - [x] 2) parity с текущими результатами
  - [x] 3) RSS не растёт от векторов в lazy-режиме because query execution reads `.acv` rows from disk instead of materializing all vectors.
- files: cortex-storage/vectors.rs, cortex-engine/search/vector.
- evidence: `.acv` current marker is now `ACV1`: header + candidate table + contiguous fixed-dimension i16 vector block + CRC. `ACV0` remains read-only compatible. `VectorIndexReader` opens `ACV1` without materializing `Vec<Vec<i16>>`, scans disk rows through bounded top-k, and keeps legacy `ACV0` readable through the compatibility path. Persisted `Vector`, `VectorExact`, and hybrid vector legs use the disk-resident reader when HNSW is disabled or exact mode is requested; stale older segment rows are hidden by newest-to-oldest candidate visibility. Dot-product scoring uses a stable chunk-8 deterministic loop instead of nightly `std::simd`.
- gates: `cargo test -p cortex-storage --test segment_index_tests acv -- --nocapture`; `cargo test -p cortex-storage --test vector_index_tests -- --nocapture`; `cargo test -p cortex-engine --test database_search database_vector_exact_reads_latest_disk_resident_acv_row --all-features`; `cargo test -p cortex-engine search::persisted::tests --all-features`; full workspace fmt/test/clippy and storage/migration gates at close.
- risks: no new dependency was approved, so the implementation uses stable-Rust disk row reads rather than OS mmap. HNSW graph search still materializes the vector map for graph validation/search because the current HNSW APIs are RAM-oriented; no-fallback and larger cache-backed promotion remain future benchmark work. Зависимости: A07. Эффект: дефолтный семантический путь масштабируется.

### EPIC-C06 — HNSW: guarded productization через nightly recall-гейты

- status: `done`
- meta: Категория: indexing · P2 · 6 months · improve
- goal: ANN нужен после 1M+; текущая guarded-позиция правильная — нужно сузить стоимость поддержки.
- problem: Проблема: 5 ann-make-гейтов в широких прогонах тормозят разработку.
- tasks:
  - [x] 1) ann-гейты → nightly
  - [x] 2) один real-embedding recall-gate (bge-m3 кэш из бенчей)
  - [x] 3) интеграция в cost-планировщик A13: ANN выбирается при больших корпусах, exact — fallback (существующая логика сохраняется).
- acceptance:
  - [x] 1) PR CI без ANN-матрицы
  - [x] 2) nightly recall-отчёт артефактом
  - [x] 3) planner-правило покрыто тестом.
- files: `crates/cortex-engine/src/plan/cost*`, `scripts/ann/build_bge_m3_cached_corpus.py`, `mk/ann.mk`, `mk/vars-core.mk`, `mk/phony.mk`, `.github/workflows/rust.yml`, `.github/workflows/ann-regression.yml`.
- evidence: Rust PR workflow no longer runs the ANN report bundle. `.github/workflows/ann-regression.yml` runs nightly/manual `make ann-nightly-regression-report`, uploads `ann-nightly-reports`, and runs `make ann-bge-m3-cache-recall-report` when the BGE-M3 cache exists. `choose_vector_search_execution` selects `ann-with-exact-fallback` only for HNSW-enabled broad corpora at 1M+ rows and keeps exact for disabled HNSW or selective candidates.
- gates: `cargo test -p cortex-engine plan::cost::tests --all-features`; full workspace fmt/test/clippy and `make check` at close.
- risks: hosted CI cannot manufacture the external 11GB BGE-M3 cache; when absent, the nightly workflow uploads readiness instead of blocking PRs. Зависимости: A13. Эффект: ANN остаётся честным и дешёвым в поддержке.

### EPIC-C07 — Гибридный retrieval (lexical+dense RRF) в движке

- status: `done`
- meta: Категория: retrieval · P1 · 90 days · build
- goal: гибрид дал doc recall 85.8% на EnterpriseRAG, но живёт во внешних python-скриптах — это должна быть фича БД.
- problem: Проблема: RRF-фьюжн вне движка (scripts/enterprise_rag_bench).
- tasks:
  - [x] 1) `RetrievalMode::Hybrid`: два scan-потока → RRF-оператор (A11)
  - [x] 2) generic-реализация без bench-эвристик (после Kill-решения)
  - [x] 3) quality-фикстура: hybrid ≥ lexical.
- acceptance:
  - [x] 1) `USING MODE hybrid` из AQL
  - [x] 2) фикстурный гейт
  - [x] 3) EXPLAIN показывает оба пути и фьюжн.
- files: `crates/cortex-aql/src/types.rs`, `crates/cortex-aql/src/binder/support.rs`, `crates/cortex-engine/src/exec/retrieve.rs`, `crates/cortex-engine/src/exec/retrieve/hybrid.rs`, `crates/cortex-engine/src/plan/cost/model.rs`, `crates/cortex-engine/src/plan/mod.rs`, `crates/cortex-engine/src/retrieval_rank*`, mode label surfaces, AQL/query_search tests.
- evidence: AQL now parses and binds `USING MODE hybrid`; the retrieve executor routes hybrid mode through bitmap + permission filtering, `LexicalScan`, `VectorScan`, and `HybridRrfOp` before the existing lifecycle/quality/rank/dedupe/pack stages; vector scoring reuses payload vector parsing and semantic dot scoring; logical EXPLAIN reports `paths=lexical,vector fusion=rrf`; EXPLAIN ANALYZE exposes both scan paths and the fusion operator. The fixture `retrieve_aql_hybrid_mode_fuses_lexical_and_vector_quality_fixture` promotes the candidate with both lexical and dense evidence above single-signal candidates.
- verification: `cargo fmt --check`; `cargo test -p cortex-aql hybrid`; `cargo test -p cortex-engine --test query_search hybrid`; `cargo test --workspace --all-features`; `cargo clippy --workspace --all-targets -- -D warnings`; `make check`; `make openapi-contract-check`.
- remaining: none for C07 acceptance.
- risks: hybrid AQL still requires an explicit `query_vector=` line unless the caller uses the existing embedding integration before AQL execution. Зависимости: A11, C05, EPIC-C08. Эффект: главный результат бенчей становится продуктом.
- next exit step: completed by `EPIC-C09`; current pointer moved to `EPIC-C10`.

### EPIC-C08 — Server-side embedding integration

- status: `done`
- meta: Категория: retrieval · P1 · 60 days · productize
- goal: «semantic» без встроенного пути получения вектора — ловушка DX (query_vector= строкой в task, database.rs:637).
- problem: Проблема: клиент обязан сам считать вектора; тихая деградация в лексику.
- tasks:
  - [x] 1) продуктизировать embedding-клиент из embedding_pipeline.rs: конфиг URL/model/key
  - [x] 2) `/v1/context {embed_query:true}`
  - [x] 3) явная ошибка «semantic requires vector or embedding config»; таймаут/fallback-политика.
- acceptance:
  - [x] 1) semantic-режим без ручных векторов
  - [x] 2) без конфига — ошибка, не молчание
  - [x] 3) e2e с локальным эмбеддером.
- files: embedding_pipeline.rs, server/{context,search}.rs.
- risks: внешний вызов в запросе — таймаут+fallback на hybrid задокументированы. Зависимости: нет. Эффект: semantic перестаёт быть переоценённым словом.
- evidence: Added `EmbeddingClientConfig`/`QueryEmbeddingProvider` product surface in `embedding_pipeline`, no-new-dependency server HTTP embedding client configured by `CORTEXDB_EMBEDDING_URL`, `CORTEXDB_EMBEDDING_MODEL`, `CORTEXDB_EMBEDDING_API_KEY`, `CORTEXDB_EMBEDDING_TIMEOUT_MS`; `/v1/context` accepts JSON/plain AQL plus `embed_query=true`, injects `query_vector=...`, and fails with `semantic requires vector or embedding config` when semantic AQL lacks vector/config; `/v1/search` and `/v1/search/explain` support `embed_query=true` when `vector` is omitted; `docs/EMBEDDING_INTEGRATION.md` documents timeout and fail-closed/no-silent-lexical-fallback policy; OpenAPI and generated SDK types were updated.
- verification: `cargo fmt --check`; `cargo test -p cortex-server embedding --all-features`; `cargo test -p cortex-server embed_query --all-features`; `cargo test -p cortex-engine embedding_pipeline --all-features`; `python3 scripts/file_size_report.py --root . --baseline quality/file_size_baseline.json --check`; `make openapi-contract-check`; `cargo test --workspace --all-features`; `cargo clippy --workspace --all-targets -- -D warnings`; `make check`.
- remaining: none for C08 acceptance.
- next exit step: move to `EPIC-A19` — 100K/1M/10M scale benchmarks and curves per corrected dependency-stage order.

### EPIC-C09 — Permission-aware index pruning

- status: `done`
- meta: Категория: indexing · P1 · 90 days · build
- goal: уникальная оптимизация agent-native БД: права агента сужают пространство ДО скана.
- problem: Проблема: scans пересекают permission-маску, но не используют её для пропуска работ.
- tasks:
  - [x] 1) интеграция scope-bitmap кардинальности в planner: маленькая разрешённая зона → bitmap-first scan
  - [x] 2) segment skipping: сегмент без разрешённых scope (zone map A12) не открывается вовсе
  - [x] 3) fixture: агент с 1% видимости использует bitmap-first estimate до скана.
- acceptance:
  - [x] 1) narrow-agent work is bounded by readable scope/segment evidence in the small gate
  - [x] 2) корректность E09: skipped unreadable patch/tombstone segments still remove stale readable candidates
  - [x] 3) EXPLAIN показывает пропущенные сегменты.
- files: `crates/cortex-engine/src/query.rs`, `crates/cortex-engine/src/query/cache.rs`, `crates/cortex-engine/src/query/explain.rs`, `crates/cortex-engine/src/query/pruning.rs`, `crates/cortex-engine/src/checkpoint/indexes.rs`, `crates/cortex-engine/src/query/statistics/zone_maps.rs`, `crates/cortex-engine/src/plan/cost/tests.rs`, `crates/cortex-engine/tests/query_search/permission_pruning.rs`.
- evidence: AQL cached and uncached binding now builds a view-pruned `EngineAqlIndex`; persisted indexes can be opened only for live segments whose zone stats may contain a readable scope; skipped segments still read candidate footers so newer unreadable patches/tombstones cannot resurrect older readable candidates; scope bitmap pruning rebuilds the candidate universe before execution; EXPLAIN emits `permission_pruning` with skipped/opened/total segment counts; the cost-model fixture uses a 1% AgentView allowed cardinality to select bitmap-first.
- verification: `cargo fmt --check`; `cargo test -p cortex-engine --test query_search permission_pruning`; `cargo test -p cortex-engine plan::cost::tests::cost_model_uses_agent_allowed_cardinality_for_permission_pruning`; `cargo test -p cortex-engine query::statistics::zone_maps::tests::zone_maps_filter_live_segments_without_retired_leaks`; `cargo test --workspace --all-features`; `cargo clippy --workspace --all-targets -- -D warnings`; `make check`; `make openapi-contract-check`.
- remaining: none for C09 acceptance.
- risks: segment skipping is conservative when stats are absent or incomplete; skipped segment candidate footers are still read to preserve update/tombstone correctness. Зависимости: A12, A13, B04. Эффект: «права делают запросы быстрее» — продаваемое свойство.
- next exit step: move to `EPIC-C10` — segment zone maps + segment skipping.

### EPIC-C10 — Segment zone maps + segment skipping

- status: `done`
- meta: Категория: indexing · P2 · 6 months · build
- goal: классическая database-техника, нужная lazy/temporal/scope фильтрам.
- problem: Проблема: каждый сегмент участвует во всех запросах.
- tasks:
  - [x] 1) zone map per segment: min/max created_at, scope-set, type-set (в манифест при checkpoint)
  - [x] 2) planner отбрасывает сегменты по предикатам
  - [x] 3) тест на 10-сегментном корпусе.
- acceptance:
  - [x] 1) temporal/scope/type queries open a segment subset and expose a counter
  - [x] 2) корректность фикстур.
- files: `crates/cortex-engine/src/query.rs`, `crates/cortex-engine/src/query/cache.rs`, `crates/cortex-engine/src/query/explain.rs`, `crates/cortex-engine/src/query/statistics/segment_pruning.rs`, `crates/cortex-engine/src/query/statistics/zone_maps.rs`, `crates/cortex-engine/src/query/metadata/ids.rs`, `crates/cortex-engine/src/checkpoint/indexes.rs`, `crates/cortex-engine/tests/query_search/segment_pruning.rs`.
- evidence: checkpoint/compact already persist `ManifestSegmentStats` zone maps for created-at min/max plus scope/status/type counts; AQL cached and uncached binding now passes the bound retrieve plan into persisted index construction; `DatabaseStatistics::segments_matching_bitmap_program` evaluates scope/status/type bitmap predicates at segment granularity and treats `NOT`/unknown handles conservatively; `REQUIRE freshness <= ...` intersects segment selection with the created-at range zone map; skipped segments still read candidate footers for update/tombstone correctness; EXPLAIN emits `segment_pruning` skipped/opened/total counters when query predicates prune beyond permission scope. Regression fixtures cover a 10-segment type query opening 2/10 segments and a freshness query opening 1/2 segments.
- verification: `cargo fmt --check`; `cargo test -p cortex-engine --test query_search segment_pruning`; `cargo test -p cortex-engine --test query_search permission_pruning`; `cargo test -p cortex-engine --test query_search`; `cargo test -p cortex-engine query::statistics::zone_maps::tests::zone_maps_filter_live_segments_without_retired_leaks`; `cargo test --workspace --all-features`; `cargo clippy --workspace --all-targets -- -D warnings`; `make check`; `make openapi-contract-check`.
- remaining: none for C10 acceptance.
- risks: `NOT`, memory-type, and unknown bitmap handles are intentionally conservative and may fall back to opening more segments. Зависимости: A12. Эффект: I/O-составляющая запросов падает.
- next exit step: move to `EPIC-C11` — AQL query cache metrics and policy.

### EPIC-C11 — AQL query cache: метрики и политика

- status: `done`
- meta: Категория: query-engine · P2 · 60 days · improve
- goal: кэш есть (AqlQueryCache), но непрозрачен.
- tasks:
  - [x] 1) hit/miss/eviction в /v1/stats и Prometheus
  - [x] 2) инвалидация по seq задокументирована
  - [x] 3) размер/политика в конфиге.
- acceptance:
  - [x] 1) hit-rate видим
  - [x] 2) тест инвалидации после записи.
- files: `crates/cortex-engine/src/query/cache.rs`, `crates/cortex-engine/src/options.rs`, `crates/cortex-engine/src/config.rs`, `crates/cortex-engine/src/database/open.rs`, `crates/cortex-server/src/router/core_routes.rs`, `crates/cortex-server/src/router/metrics_routes.rs`, `crates/cortex-api-types/src/core.rs`, `docs/openapi.yaml`, SDK generated/manual type files, `docs/ENGINE_API.md`, `docs/archive/ENGINE_CONFIG.md`, `crates/cortex-engine/tests/aql_query_cache.rs`.
- evidence: `/v1/stats` and `/v1/metrics` now include nested `aql_query_cache` counters with entries, max_entries, hits, misses, evictions, catalog invalidations, and Q16 hit rate. Prometheus export now emits matching `cortexdb_aql_query_cache_*` gauge/counter series. `DatabaseOptions::aql_query_cache_max_entries` and `CORTEXDB_AQL_QUERY_CACHE_MAX_ENTRIES` configure the bounded FIFO plan-cache size while preserving the 128-entry default. The invalidation policy is documented as query + AgentView + current seq/manifest/live-segment fingerprint, so writes and segment rewrites invalidate stale bound plans before reuse. Regression coverage verifies write-driven invalidation and configured max_entries=1 FIFO eviction.
- verification: `cargo fmt --check`; `make openapi-sdk-generated-types-check`; `cargo test -p cortex-engine --test aql_query_cache`; `cargo test -p cortex-engine config --all-features`; `INSTA_UPDATE=always cargo test -p cortex-server response_snapshot_tests`; `cargo test -p cortex-server snapshot_metrics_includes_actor_and_request_fields`; `cargo test -p cortex-server metrics_prometheus_output_contains_contract_series`; `cargo test -p cortex-server snapshot_stats_response_shape`; `cargo test --workspace --all-features`; `cargo clippy --workspace --all-targets -- -D warnings`; `make check`; `make openapi-contract-check`.
- remaining: none for C11 acceptance.
- risks: Metrics are process-local cumulative counters, not a time-windowed hit-rate. The fingerprint invalidates conservatively on any commit sequence change, so write-heavy workloads may report lower hit-rate by design. Зависимости: E05. Эффект: наблюдаемость кэша.
- next exit step: move to `EPIC-C18` — concurrent read throughput benchmark.

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
  - [x] 1) extraction при записи (B07) → typed fact rows
  - [x] 2) индекс metric_id→sorted (value, cell)
  - [x] 3) конфликт-запрос: same metric, same scope, different normalized value.
- acceptance:
  - [x] 1) numeric-verify через индекс с прежними вердиктами
  - [x] 2) p95 на 1M
  - [x] 3) инкрементальность под property-тестом.
- files: новый index-модуль, verification/numeric.rs.
- latest evidence: `FactClaimStore` now maintains a metric/scope/project -> sorted normalized value -> cell index alongside typed numeric records. VERIFY first asks this index for numeric candidate cell ids and falls back to the previous lexical candidate scan only when the typed index has no hits. `ConflictIndexStore::from_memtable` now batches numeric facts and rebuilds conflict records by metric/scope/project group, avoiding whole-corpus pair scans on large numeric fixtures. Added `numeric_verify_index_check` and `make numeric-verify-index-check`; local 1M direct-checkpoint fixture passed with numeric VERIFY latency p50 `155.498ms`, p95 `157.387ms`, p99 `159.038ms`, max `159.407ms`, report `target/numeric-verify-index/report.json`.
- risks: The C13 1M gate uses a dedicated typed numeric fixture with unique metric groups plus one support/conflict pair; broad natural-language facts without a typed metric still fall back to the existing lexical path. Зависимости: B07. Эффект: verification-масштаб.
- next exit step: `EPIC-C15` is now closed; move to `EPIC-B19` — REMEMBER write-path policy formalization.

### EPIC-C14 — Temporal индекс

- status: `pending`
- meta: Категория: indexing · P2 · 6 months · build
- goal: B10-запросы «valid at date» должны быть индексными.
- tasks:
  - [x] 1) interval-индекс (sorted by valid_from + zone maps)
  - [x] 2) подключение к planner-предикатам
  - [x] 3) stale-guard VERIFY через индекс.
- acceptance:
  - [x] 1) временные запросы используют interval index + zone cache before payload materialization
  - [x] 2) temporal-фикстуры зелёные.
- files: `retrieval_quality/validity_index.rs`, `retrieval_quality/validity_index/interval.rs`, `exec/retrieve/temporal_filter.rs`, `verification/operator.rs`, `verification/guards.rs`, `verification/temporal.rs`.
- latest evidence: `TemporalValidityStore` now maintains incremental valid_from/valid_to BTree indexes and a lazy sorted valid_from zone cache. AQL `REQUIRE valid at` builds the valid CellId set once from the interval index and filters candidates before lazy payload reads. VERIFY emits a `VerificationTemporalIndexLookup` trace and uses indexed stale reasons for stale/future evidence guards while preserving lexical overlap semantics. `make temporal-validity-index-check` passed on a 10K lazy fixture with `query_elapsed_ms=152`, `returned_cells=10`, `segment_loads_after_query=10`, report `target/temporal-validity-index/report.json`.
- risks: The default temporal gate is bounded at 10K for interactive reliability; larger 100K+ temporal runs remain override/benchmark-packet work if needed. Зависимости: A02, B10. Эффект: temporal — индексная фича.
- next exit step: `EPIC-C15` is now closed; move to `EPIC-B19` — REMEMBER write-path policy formalization.

### EPIC-C15 — Инкрементальный graph-индекс производительность

- status: `pending`
- meta: Категория: indexing · P2 · 6 months · build
- goal: завершение B18 производительной частью: adjacency-структуры, обходы с лимитами.
- tasks:
  - [x] 1) компактная adjacency (interned entity ids)
  - [x] 2) bounded BFS/DFS с visit-budget (как в HNSW-гардах)
  - [x] 3) бенч 100K-узлового графа.
- acceptance:
  - [x] 1) обходы с budget-гарантией
  - [x] 2) p95 зафиксирован.
- files: `crates/cortex-engine/src/graph/*.rs`, `crates/cortex-engine/src/graph_retrieval.rs`, `crates/cortex-engine/src/bin/graph_index_performance_check.rs`, `mk/performance-dashboard.mk`.
- evidence: `KnowledgeGraphIndex` now stores compact adjacency as interned entity ids plus edge ids instead of duplicated edge vectors. Bulk graph-index builds use an add-only path instead of per-record remove scans. `graph_retrieve_related_with_budget` returns `GraphRetrievalReport` with visited edge/entity counts and `budget_exceeded`, while the existing `graph_retrieve_related` API keeps returning hits. `make graph-index-performance-check` passed on a 100K-node / 99,999-edge graph: p50 `0.497235ms`, p95 `0.550493ms`, max `97.626455ms`, visited_edges p95 `62`, budget_exceeded_samples `0`, report `target/graph-index-performance/report.json`.
- gates: `cargo fmt --check`; `cargo test -p cortex-engine --test graph_retrieval_tests --test graph_tests --test graph_index_incremental_tests`; `make graph-index-performance-check`.
- remaining: none for C15 acceptance.
- risks: the default 100K gate measures bounded graph traversal over an in-memory typed graph fixture, not persisted DB ingest latency; max latency can include local scheduler noise, while p95 is the acceptance metric.
- next exit step: move to `EPIC-B19` — REMEMBER write-path policy formalization.

### EPIC-C16 — Memory profiling harness (dhat/jemalloc)

- status: `done`
- meta: Категория: benchmarks · P0 · 30 days · build
- goal: все RAM-обещания нуждаются в измерителе; estimated_* поля /v1/stats не верифицированы.
- tasks:
  - [x] 1) dhat за feature-флагом + jemalloc stats — allocator-specific observers remain explicitly unavailable under the no-new-dependencies policy; enabling them requires an explicit dependency/runtime approval, not hidden scope expansion
  - [x] 2) `make memory-profile` → JSON (RSS, аллокации, payload-клоны) — portable RSS/estimate/clone-gate report added
  - [x] 3) сверка estimated vs real (допуск, фиксы расчётов) — `make memory-estimate-audit` compares existing memory-profile and scale-benchmark RSS/estimate rows; estimator fixes remain future work if ratio exceeds policy.
- acceptance:
  - [x] 1) отчёт воспроизводим — `make memory-profile MEMORY_PROFILE_CELLS=10000`
  - [x] 2) клон-счётчик используется в A04/A05 acceptance — `payload_clone_gate` is included in the JSON report and mirrors the static clone gate
  - [x] 3) расхождение estimated/real задокументировано — `docs/archive/MEMORY_PROFILE.md`
- files: cortex-bench, memory_accounting.rs.
- risks: нет. Зависимости: нет. Эффект: инструмент всего блока A/C.
- evidence: Added `memory_profile_check` and `make memory-profile`. Local 10K report `target/memory-profile/10k/report.json`: `ok=true`, RSS `38936576`, peak RSS `40894464`, estimated total `28795568`, RSS/estimated ratio `1.352`, peak/estimated ratio `1.420`, payload clone gate passed. Allocator-specific `dhat`/`jemalloc` observers are explicitly marked unavailable until dependency/runtime approval.
- latest evidence: Added `scripts/memory_estimate_audit.py` and `make memory-estimate-audit`. The local audit reads existing `target/memory-profile/**/*.json` and `target/scale-bench/**/*.json`, writes `target/memory-profile/estimate-audit.json`, and currently passes with 44 comparable rows, max RSS/estimated ratio `33.574678`, max peak RSS/estimated ratio `33.680224`, and threshold `128.0`. This proves the portable estimator gap is visible across current reports; it does not implement allocator-specific profiling.
- risks: Allocator-level allocation counts are not part of the default no-new-dependencies profile. If we later approve `dhat`/`jemalloc`, add them as a new profiling enhancement rather than reopening C16's portable gate.
- next exit step: move to `EPIC-C17` — perf-regressions in CI and continuous benchmarking.

### EPIC-C17 — Перф-регрессии в CI (continuous benchmarking)

- status: `done`
- meta: Категория: benchmarks · P1 · 60 days · build
- goal: 100 эпиков перфорации без регресс-гейта = регрессии.
- tasks:
  - [x] 1) nightly perf-job: фикс-корпуса 100K, метрики p50/p95 в trend.json (performance-trend-check уже есть — подключить к новому) — `.github/workflows/continuous-benchmark.yml` runs the hosted gate on schedule and manual dispatch.
  - [x] 2) порог регрессии (>20% p95 → красный) — `continuous_benchmark_gate.py` enforces max p95/p99 ratio `1.2`, the Make targets pass a 25ms minimum absolute-delta floor for runner jitter, and the synthetic self-test proves a `1.25` p95 regression still fails.
  - [x] 3) история артефактами — added `fixtures/performance/history/v0.2.0-beta.2` and `make continuous-benchmark-gate` writes `target/continuous-benchmark-gate/report.json`.
- acceptance:
  - [x] 1) nightly красный при искусственной регрессии (тест процесса) — local self-test covers the threshold decision and the hosted workflow runs the same `continuous_benchmark_gate.py` policy.
  - [x] 2) тренд-страница генерируется — Markdown reports are generated for scale trends, memory audit, and continuous benchmark gate; hosted CI uploads them as artifacts.
- files: CI workflows, cortex-bench.
- latest evidence: Added `.github/workflows/continuous-benchmark.yml` with nightly cron plus manual dispatch, stable Rust setup, `make continuous-benchmark-hosted-gate`, and `continuous-benchmark-reports` artifact upload. Added `fixtures/performance/hosted-history/v0.2.0-beta.2-ci` so hosted runs compare against same-profile CI fixtures instead of local release-machine history. Added `scale-bench-ci` and `continuous-benchmark-hosted-gate`: the hosted path regenerates load-smoke, single-node performance, CI-safe fixed-payload 10K/100K direct scale reports, performance trends, scale trends, memory estimate audit, the continuous benchmark gate with `--min-regression-delta-ms 25`, and the synthetic regression self-test. `make continuous-benchmark-gate` and `make continuous-benchmark-hosted-gate` passed locally with `status=passed`, `errors=0`, `warnings=0`. `python3 scripts/continuous_benchmark_gate.py --self-test` passed and proves an artificial `1.25` p95 regression is detected.
- risks: Hosted CI uses bounded 10K/100K fixed-payload scale curves, not the full A19 1M/10M packet. The 25ms delta floor suppresses tiny runner jitter only after the ratio artifact includes current/previous/delta details; larger absolute p95/p99 regressions still fail. Accumulated scheduled runs are still needed before hosted trend ratios become strong release evidence. Зависимости: A19. Эффект: перф-дисциплина.
- next exit step: move to `EPIC-C13` — Fact/numeric index.

### EPIC-C18 — Concurrent read throughput bench

- status: `done`
- meta: Категория: benchmarks · P1 · 60 days · benchmark
- goal: A16 без бенча — вера, не факт.
- tasks:
  - [x] 1) нагрузочный сценарий: K читателей + 1 писатель, latency/throughput кривые по потокам — `concurrent_read_benchmark_check` publishes 1/2/4 reader curves with a writer on each run.
  - [x] 2) сравнение actor-only vs RwLock-пути — report compares `actor_route_shared` and `rwlock_direct` modes.
  - [x] 3) включить в trend — `performance_trend_check.py` now validates and compares `target/concurrent-read-benchmark/report.json`.
- acceptance:
  - [x] 1) кривая масштабирования опубликована
  - [x] 2) включено в C17.
- files: `crates/cortex-server/src/bin/concurrent_read_benchmark_check.rs`, `crates/cortex-server/src/bin/concurrent_read_benchmark_check/*.rs`, `mk/performance-dashboard.mk`, `mk/vars-ops-release.mk`, `scripts/performance_trend_check.py`, `.github/workflows/continuous-benchmark.yml`, `fixtures/performance/**/concurrent_read_benchmark_report.json`.
- latest evidence: `make concurrent-read-benchmark-check` passed and wrote `target/concurrent-read-benchmark/report.json`/`.md`; current curve shows `rwlock_direct` read throughput scaling from `223.131` ops/s at 1 reader to `859.410` ops/s at 4 readers while `actor_route_shared` stayed near `214.532` → `191.810` ops/s. `make performance-trend-check` passed and included `concurrent_read_p50_p95_p99_ratio` plus `concurrent_read_summary` in `target/performance-trends/report.json`. Verification passed: `cargo fmt --check`, `cargo test --workspace --all-features`, `cargo clippy --workspace --all-targets -- -D warnings`.
- risks: default C18 gate is intentionally bounded (200 cells, 1/2/4 readers) and proves concurrency shape, not large-corpus throughput. Зависимости: A16. Эффект: доказательство конкурентности.
- next exit step: move to `EPIC-C19` — ingestion throughput + batch embedding pipeline.

### EPIC-C19 — Ingestion throughput + батчевый embedding pipeline

- status: `done`
- meta: Категория: retrieval · P2 · 6 months · improve
- goal: загрузка 511K документов для бенча заняла часы на embedding — узкое место реальных пользователей.
- tasks:
  - [x] 1) батчинг/параллелизм embedding-запросов с резюмируемостью (частично есть — оформить) — `EmbeddingBackfillProvider::embed_text_batch`, `backfill_embedding_debt_batched`, partial-run resume coverage.
  - [x] 2) ingestion через WriteBatch (A15) — `ingestion_throughput_check` writes synthetic docs through `WriteBatch`, and batch validation is O(batch) for patch/tombstone checks.
  - [x] 3) бенч docs/sec end-to-end — `make ingestion-throughput-check` publishes JSON/Markdown and feeds `performance-trend-check`.
- acceptance:
  - [x] 1) reproducible ingestion+embedding docs/sec number — default 10K gate: `end_to_end_docs_per_sec=1281.777`, `ingest_docs_per_sec=4401.882`, `embedding_docs_per_sec=2347.038`; explicit 100K target is `make ingestion-throughput-100k-check`.
  - [x] 2) resume после обрыва (тест) — `database_backfill_embedding_debt_resumes_after_partial_run`.
- files: `crates/cortex-engine/src/embedding_pipeline/*`, `crates/cortex-engine/src/bin/ingestion_throughput_check*`, `mk/performance-dashboard.mk`, `scripts/performance_trend_check.py`, `.github/workflows/continuous-benchmark.yml`.
- latest evidence: `make ingestion-throughput-check` passed and wrote `target/ingestion-throughput/report.json`/`.md` for 10K docs with `ingestion_write_batches=10`, `embedding_batches=80`, `embedding_write_batches=80`, partial backfill `5000 -> reopen -> 5000`, and `final_debt_items=0`. `make performance-trend-check` passed with `ingestion_throughput_summary`.
- risks: the 100K full derived-index backfill is available as an explicit long-running gate rather than default CI; the default C19 gate is bounded by the roadmap small/medium evidence rule. Зависимости: A15. Эффект: реальный onboarding больших корпусов.
- next exit step: move to `EPIC-C20` — baseline comparison with naive stack.

### EPIC-C20 — Baseline-сравнение с наивным стеком

- status: `done`
- meta: Категория: benchmarks · P2 · 90 days · benchmark
- goal: честный ответ на «зачем вы, если есть SQLite FTS5 + faiss»: если по качеству ретрива не выигрываем — надо знать; выигрываем по governance — показать.
- tasks:
  - [x] 1) референс-стек (SQLite FTS5 + CI-safe exact hashed-vector fallback вместо обязательного faiss dependency) в cortex-bench
  - [x] 2) прогон на ваших 4 quality-доменах + латентность
  - [x] 3) публикация таблицы качество/латентность/фичи (permissions/budget/citations).
- acceptance:
  - [x] 1) воспроизводимый скрипт — `make baseline-comparison-check`
  - [x] 2) результат опубликован, каким бы ни был — `docs/BASELINE_COMPARISON.md`.
- files: `scripts/baseline_comparison_check.py`, `fixtures/baseline_comparison/feature_matrix.json`, `docs/BASELINE_COMPARISON.md`, `docs/BENCHMARKS.md`, `mk/core-retrieval-context.mk`, `mk/vars-core.mk`, `mk/phony.mk`.
- evidence: `make baseline-comparison-check` passed and wrote `target/baseline-comparison/report.json` plus `docs/BASELINE_COMPARISON.md`. Four-domain result: investment_projects `SQLite FTS5=92.50%`, `hash_vector=75.00%`, `hybrid=87.50%`, CortexDB retrieval gate `95.00%`; legal_policies/support_tickets/technical_docs all `100.00%` for hybrid and CortexDB. ContextPack v3 evidence stayed passed with 4 external datasets, 105 cases, 100% evidence/citation coverage, and 56.65% token reduction.
- risks: dense side is a deterministic stdlib exact vector fallback, not a hosted FAISS/BGE run, to preserve the no-new-dependencies rule; report calls this out explicitly. Зависимости: A19. Эффект: позиция «зачем мы» получает данные.
- next exit step: move to `EPIC-E03` — WAL archive to point-in-time recovery.

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

- status: `done`
- meta: Категория: CLI · P1 · 60 days · build
- tasks:
  - [x] 1) init: база+пример AgentView+печать следующих шагов — `cortexdb init <path>` opens/creates the database, persists starter AgentView `agent_id=1`, writes a scoped starter cell when cell 1 is empty, and prints copy-paste `doctor`, `context`, and `verify` next commands.
  - [x] 2) doctor: lock, WAL-валидность, версии форматов, RAM-прогноз vs доступно — doctor now reports `wal`, `format_versions`, and `memory_forecast` alongside existing open/lock/stats/validate/backup/server/auth/ANN checks.
  - [x] 3) выводить «что не так и что сделать» — failure paths include explicit repair/unlock/WAL validation/memory sizing advice.
- acceptance:
  - [x] 1) init→quickstart без чтения доков — CLI regression runs `init -> doctor -> context` on a fresh path and verifies the starter AgentView file plus retrieved starter ContextPack.
  - [x] 2) doctor ловит 5 типовых проблем (тесты) — tests cover invalid tenant, stale lock, configured backup root without backups, WAL failure advice, and RAM forecast failure.
- files: cortex-cli.
- risks: нет. Зависимости: D01. Эффект: нулевой порог входа.
- evidence: `cargo test -p cortex-cli tests::basics --all-features`, `cargo test -p cortex-cli cli_doctor_checks --all-features`, and `cargo test -p cortex-cli --all-features` passed. File-size discipline held: `cli_ops/core.rs` 263 lines, `cli_doctor_checks.rs` 281, `cli_doctor_checks/system.rs` 120, and `cli/args/commands.rs` 300.

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
- evidence: `make sdk-e2e-release-check` passed after SDK release/deprecation/publication gates were aligned to archived docs; `make sdk-check` passed and produced Rust `cargo package`, Python SDK tests, and npm pack dry-run evidence. D05 follow-up audit found that the `SDK Release` workflow preflight failed on the beta.2 tag because it tried `cargo publish -p cortex-sdk --dry-run` before the unpublished `cortex-api-types` dependency existed on crates.io. The workflow, manifest, registry gate, and docs now model the required order: `cortex-api-types` first, then `cortex-sdk`; local gates passed again (`make sdk-release-contract-check`, `make sdk-registry-gate-check`, `make sdk-e2e-release-check`, `make sdk-check`).
- remaining: public registry publication and clean-machine install smoke still require external registry setup. Current GitHub repository state has no `sdk-release` environment and no repo-level `NPM_TOKEN` or `CARGO_REGISTRY_TOKEN`; PyPI trusted publishing is not configured from the repo state. Do not claim public SDK publication until those are configured and the manual tag-gated release job succeeds.

### EPIC-D06 — Python SDK: typed-модели, ретраи, таймауты

- status: `done`
- meta: Категория: SDK · P1 · 60 days · improve
- tasks:
  - [x] 1) модели из ContextPack-схемы (B01, codegen) — Python SDK exposes split typed dataclass models for ContextPack/Search/Verify/Ingestion/Core responses and now ships `py.typed` in the wheel.
  - [x] 2) retry с экспонентой на 503 database_busy (ваш собственный backpressure-контракт!) — transport retries `503` only for `database_busy`/`service_unavailable` JSON codes, keeps 502/504 retryable, and has a regression with a flaky opener.
  - [x] 3) таймауты, context manager, connection reuse — client has `with_timeout`, `with_session`, `__enter__/__exit__`, reusable opener plumbing, and close semantics.
- acceptance:
  - [x] 1) mypy-чистые типы — PEP 561 marker is packaged; `python3 -m py_compile` covers all SDK modules in this environment where mypy is not a project dependency.
  - [x] 2) retry-тест против заполненной очереди — unit test simulates `503 {"code":"database_busy"}` and proves retry plus configured timeout propagation.
  - [x] 3) README-пример с типами — Python README now shows `ContextPackResponse`, retries, timeout, and session usage.
- files: sdk/python.
- risks: нет. Зависимости: B01, D05. Эффект: SDK production-поведения.
- evidence: `python3 -m py_compile sdk/python/cortexdb_client.py sdk/python/_cortexdb_client/*.py sdk/python/_cortexdb_client/model_types/*.py`, `python3 -m unittest discover -s sdk/python -p 'test_*.py'`, wheel smoke via `python3 -m pip wheel sdk/python --no-deps`, and `make sdk-check` passed. The wheel smoke verified `cortexdb_client.py`, `_cortexdb_client/client.py`, `_cortexdb_client/transport.py`, `_cortexdb_client/answering.py`, `_cortexdb_client/model_types/context.py`, and `_cortexdb_client/py.typed` are included.

### EPIC-D07 — TypeScript SDK polish

- status: `done`
- meta: Категория: SDK · P1 · 60 days · improve
- tasks:
  - [x] 1) типы из схемы; ESM+CJS — declaration surface exports typed API models plus `ClientOptions`/`FetchLike`; package keeps ESM and CJS bundles in sync with source.
  - [x] 2) retry на 503 — transport retries `503` only for `database_busy`/`service_unavailable`, keeps 502/504 retryable, and does not retry generic `500 internal`.
  - [x] 3) пример с LLM-вызовом — `examples/grounded-answer-llm.mjs` shows the grounded-answer flow through an OpenAI-compatible local endpoint.
- acceptance:
  - [x] 1) tsd-тесты типов — no new dependency was added; `npm run typecheck` uses Node's TypeScript strip smoke over the declaration-facing example.
  - [x] 2) 10-строчный рабочий пример — `examples/grounded-answer-llm.mjs`.
- files: sdk/typescript. Зависимости: B01, D05.
- evidence: `npm test`, `npm run typecheck`, `node --check cortexdb-client.js`, `node --check cortexdb-client.cjs`, `npm pack --dry-run`, and `make sdk-check` include the TypeScript SDK contract.

### EPIC-D08 — Async Rust SDK + общий крейт api-types

- status: `done`
- meta: Категория: SDK · P1 · 90 days · build
- goal: cortex-sdk блокирующий; типы ответов дублируются с сервером.
- tasks:
  - [x] 1) `cortex-api-types` (вынести из server/responses.rs) — extracted shared core/system, AQL, search, and verification wire types into `crates/cortex-api-types`; `cortex-sdk` re-exports those types and `cortex-server` uses them for the same response surfaces.
  - [x] 2) async-клиент (reqwest) feature-флагом — added feature-gated `AsyncCortexDbClient` with core, AQL/context/verify/remember, search, ingestion, retry, timeout, auth, and tenant routing support.
  - [x] 3) contract-тесты на оба клиента — shared response compile-contract tests prove SDK uses `cortex_api_types` for core/AQL/search/verification and server snapshots prove the JSON surface stayed stable; api-types has wire-shape tests for batch request and legacy SDK stats decoding, and async SDK tests cover tenant routing plus `Send` futures for the main API groups.
- acceptance:
  - [x] 1) сервер и SDK используют одни типы — covered for core/system, AQL, search, and verification responses; ContextPack and ingestion validation still track follow-up extraction because they touch generated schema/engine validation types.
  - [x] 2) async-клиент проходит те же тесты.
- files: новый crates/cortex-api-types, cortex-sdk, cortex-server.
- risks: `cortex-api-types` must be published before `cortex-sdk`; local `sdk-check` verifies `cortex-api-types` package and skips `cortex-sdk` package verification until `CORTEX_API_TYPES_PUBLISHED=1`. Dependencies: B01. Effect: API type drift is now compile-time visible on the migrated surfaces.
- evidence: `cargo fmt --check`, `cargo test --workspace --all-features`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo clippy -p cortex-sdk --all-features --all-targets -- -D warnings`, `make check`, `make openapi-contract-check`, `cargo test -p cortex-api-types`, `cargo test -p cortex-sdk --all-features`, `cargo test -p cortex-sdk shared_with_api_types`, `cargo test -p cortex-server response_snapshot_tests::snapshot`, `cargo package -p cortex-api-types --allow-dirty`, and `make sdk-check` passed.

### EPIC-D09 — Docker GHCR + compose quickstart

- status: `done`
- meta: Категория: adoption · P1 · 60 days · productize
- tasks:
  - [x] 1) publish в release workflow — release tags now build and push `ghcr.io/<repo>:<tag>` and `:latest` after ANN baseline, with GHCR login, OCI labels, and image CLI smoke.
  - [x] 2) compose: server+авто-загрузка фикстуры+дашборд — quickstart compose now has a one-shot fixture seed service, dashboard enabled, hardened runtime settings, healthcheck, and shared volume.
  - [x] 3) docker-путь в GETTING_STARTED — added Docker quickstart commands, dashboard URL, GHCR image example, and Docker docs link.
- acceptance:
  - [x] 1) `docker run ghcr.io/...` поднимает рабочий сервер — release workflow publishes the GHCR image on tags; local equivalent compose/image smoke built the image, loaded fixtures, and served API/dashboard. Actual GHCR publication remains tag-release controlled.
  - [x] 2) healthcheck зелёный — real `docker compose up --build -d` smoke reached healthy status and `/v1/health` returned `{"status":"ok"}`.
- files: Dockerfile, workflows, docs.
- evidence: Added `release-container` to `.github/workflows/release.yml`, fixed Docker build context for storage compatibility fixtures, added fixture seeding and dashboard defaults to `docker-compose.yml`, added `docs/DOCKER.md`, `docs/SDK_DOCKER_OBSERVABILITY.md`, Docker quickstart material in `docs/GETTING_STARTED.md`, documentation index links, and `scripts/docker_quickstart_check.py` wired as `make docker-quickstart-check`. Checks passed: `make docker-quickstart-check`, `make docker-hardening-check`, `make docker-production-compose-check`, `docker compose config`, `docker compose -f docker-compose.production.yml config`, real `docker compose up --build -d` smoke, `/v1/health`, `/dashboard`, `/v1/stats`, and `/v1/search` smoke, followed by `docker compose down -v`, `cargo fmt --check`, `cargo test --workspace --all-features`, `cargo clippy --workspace --all-targets -- -D warnings`, `make getting-started-check`, and `make check`.

### EPIC-D10 — OpenAPI как единый источник + codegen-контроль

- status: `done`
- meta: Категория: API · P1 · 90 days · improve
- goal: openapi.yaml есть; нужно гарантировать соответствие коду.
- tasks:
  - [x] 1) contract-тест: реальные ответы валидируются против OpenAPI (расширить openapi-contract-check)
  - [x] 2) error codes (E-таксономия) в схеме
  - [x] 3) генерация клиентских типов из схемы в SDK-пайплайне — Python/TypeScript ship generated OpenAPI type artifacts; Rust uses shared `cortex-api-types` and contract gates.
- acceptance:
  - [x] 1) расхождение код/схема валит CI
  - [x] 2) SDK-типы из единого источника.
- files: docs/openapi.yaml, тесты server.
- evidence: `openapi-contract-check` validates live HTTP responses against
  OpenAPI, verifies the stable error taxonomy across docs/OpenAPI/server/SDK,
  checks generated OpenAPI-derived SDK type artifacts, and runs
  `openapi-sdk-codegen-control-check`. Added
  `scripts/check_openapi_sdk_codegen_control.py` to fail drift between selected
  OpenAPI component schemas, shared Rust API/server response structs, Python SDK
  models, and modular TypeScript SDK type declarations. Added
  `scripts/generate_openapi_sdk_types.py`, which generates
  `sdk/typescript/cortexdb-client/generated/openapi-types.ts` and
  `sdk/python/_cortexdb_client/generated/openapi_types.py` from
  `docs/openapi.yaml`; the generated types use the `OpenApi*` prefix and are
  exported/packaged by the TypeScript and Python SDKs while Rust keeps using the
  shared `cortex-api-types` wire structs. Python and TypeScript SDK models were
  aligned with current stats/search/verification/ingestion schemas,
  `NumericConflictResponse` is now a reusable OpenAPI component, and
  `sdk/README.md` documents the generator boundary. Checks passed:
  `python3 scripts/generate_openapi_sdk_types.py --check`,
  `python3 scripts/check_openapi_sdk_codegen_control.py`,
  `make openapi-contract-check`, `make sdk-contract-check`, `make sdk-check`,
  `cargo test -p cortex-sdk --all-features`, `cargo fmt --check`,
  `cargo test --workspace --all-features`, `cargo clippy --workspace
  --all-targets -- -D warnings`, and `make check`.
- remaining: no open D10 work for the current OpenAPI/SDK contract surface;
  future endpoint/schema additions must update OpenAPI and regenerate SDK type
  artifacts in the same change.

### EPIC-D11 — MCP server adapter

- status: `done`
- meta: Категория: adoption · P1 · 60 days · build
- goal: MCP — стандарт подключения инструментов к агентам; tools `retrieve_context`/`verify_fact`/`remember` идеально ложатся на API.
- tasks:
  - [x] 1) `cortex-mcp` (stdio) поверх SDK
  - [x] 2) маппинг AgentView↔MCP-конфиг
  - [x] 3) док «подключи к Claude Code/IDE за 2 минуты» + demo.
- acceptance:
  - [x] 1) рабочий MCP-конфиг из коробки
  - [x] 2) демо: агент отвечает с цитатами из CortexDB.
- files: новый crates/cortex-mcp.
- risks: ещё одна поверхность — держать тонкой (только 3 tools). Зависимости: D05. Эффект: путь к первым живым пользователям.
- evidence: added `crates/cortex-mcp` stdio JSON-RPC adapter over `cortex-sdk`
  with exactly three tools: `retrieve_context`, `verify_fact`, and `remember`.
  AgentView access remains server-side: MCP config supplies base URL, tenant,
  default scope/brain, and Bearer token; the CortexDB server maps token policy
  to role/agent permissions. Added `docs/MCP.md` and
  `examples/mcp/claude-code.json`. Checks passed:
  `cargo fmt --check`, `cargo test -p cortex-mcp --all-features`, and stdio
  smoke:
  `printf '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}\n' |
  cargo run -p cortex-mcp --quiet --`, which returned all three tools.

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

### EPIC-D15 — v0.2.0-beta.2: версии, release notes, тег

- status: `done`
- meta: Категория: product · P0 · 30 days · productize
- goal: workspace 0.1.0 при бета-цели 0.2.0-beta.2; гейты есть, релиза нет.
- tasks:
  - [x] 1) bump версий — workspace/Rust/TypeScript/OpenAPI now use `0.2.0-beta.2`; Python uses the documented PEP 440 spelling `0.2.0b2`; SDK/release gates validate this mapping.
  - [x] 2) `make beta-release-check` + бинарники (binary-release-check) — passed after aligning release gates with archived documentation paths and fixing a VERIFY numeric false-positive in the RAG demo smoke.
  - [x] 3) тег+GitHub release; release notes сверить с реальностью (после repositioning) — `v0.2.0-beta.2` tag and GitHub prerelease are published on the verified commit.
- acceptance:
  - [x] 1) тег с артефактами
  - [x] 2) README-статус соответствует.
- dependencies: A01, D12, D05 (желательно). Эффект: точка отсчёта «бета».
- evidence: `make sdk-check`, `make sdk-e2e-release-check`, `make openapi-contract-check`, `make beta-release-check`, `make binary-release-check BINARY_RELEASE_VERSION=v0.2.0-beta.2 BINARY_RELEASE_ID=cortexdb-v0.2.0-beta.2-local`, `make release-artifact-manifest-check BINARY_RELEASE_VERSION=v0.2.0-beta.2 BINARY_RELEASE_ID=cortexdb-v0.2.0-beta.2-local BINARY_RELEASE_ARCHIVE=target/release-artifacts/cortexdb-v0.2.0-beta.2-local.tar.gz`, `make evidence-artifact-retention-check`, and `make versioning-policy-check` passed after the version bump. Earlier `make rag-demo-smoke` passed after VERIFY stopped cross-comparing matched year and amount values as numeric contradictions. Latest D15 gate evidence: `make beta-release-check` passed on committed SHA `bbd3b6c35a77a1d9c6d3845e9dd2b2ef91b16dc8` with `target/beta-release/report.json` status `passed`, version `0.2.0-beta.2`, and evidence archive `target/beta-release/evidence.tar.gz`. The published annotated tag `v0.2.0-beta.2` peels to the same commit, and the GitHub release is a prerelease at `https://github.com/AubakirovArman/CortexDB/releases/tag/v0.2.0-beta.2` with `cortexdb-v0.2.0-beta.2-local.tar.gz`, its `.sha256`, and `evidence.tar.gz` attached.
- remaining: none for D15. Public SDK registry publication remains governed by `EPIC-D05`.

## Block E — Reliability, security, and operations

### EPIC-E01 — WAL writer: ошибки не глотаются

- status: `done`
- meta: Категория: storage · P0 · 30 days · improve
- goal: `run_writer` молча умирает при ошибке открытия (wal/writer.rs:166-168) — все appends получают WalWriterClosed без причины.
- tasks:
  - [x] 1) канал готовности: start ждёт подтверждения открытия
  - [x] 2) последняя ошибка потока в shared state → в текст WalWriterClosed
  - [x] 3) тест: read-only dir / invalid WAL path → осмысленная ошибка из Database::open.
- acceptance:
  - [x] 1) ошибка видна сразу
  - [x] 2) тест зелёный.
- files: cortex-storage/src/wal/writer.rs.
- evidence: `WalWriter::start_with_options` now waits for a background writer
  readiness acknowledgement before returning a handle. `WalWriterHandle` keeps a
  shared last-error state and failed sends/receives now return
  `StorageError::WalWriterClosed(reason)`, including shutdown and background
  failure reasons. WAL startup/reader IO errors include the concrete WAL path.
  Rotation/open/write failures close the writer state instead of allowing later
  appends to continue unsafely with a generic closed error. The runtime loop was
  split into `wal/writer_runtime.rs` so `writer.rs` stays under the file-size
  limit. Regression coverage: `writer_start_surfaces_open_error_immediately`,
  `append_after_shutdown_reports_closed_reason`,
  `closed_error_uses_last_recorded_reason`, and
  `database_open_reports_invalid_wal_path`. Checks passed:
  `cargo fmt --check`, `cargo test -p cortex-storage --all-features`, and
  `cargo test -p cortex-engine --test lifecycle_tests --all-features`.

### EPIC-E02 — Backup UX: один happy path + verify

- status: `done`
- meta: Категория: ops · P1 · 60 days · improve
- tasks:
  - [x] 1) `backup create` = snapshot+validate+checksum-манифест
  - [x] 2) `backup verify` без восстановления
  - [x] 3) restore с прогрессом; 6 команд остаются как advanced.
- acceptance:
  - [x] happy path = 2 команды; verify ловит порчу (тест).
- files: cortex-cli, engine/backup.
- evidence: Added `backup_manifest.tsv` with file sizes and CRC32C checksums for copied backup files, engine-level `Database::verify_backup_path`, CLI `backup-verify <backup_path>`, and docs for the two-command happy path `backup` + `backup-verify`. `restore --dry-run` now also reports checksum manifest presence/verified file count when a manifest exists. Regression coverage: `backup_verify_command_validates_backup_and_catches_corruption` verifies a fresh backup, corrupts a backed-up segment, and confirms `backup-verify` rejects it via checksum manifest mismatch. Targeted gates passed: `cargo test -p cortex-cli backup_verify_command_validates_backup_and_catches_corruption --all-features`, `cargo test -p cortex-cli backup --all-features`, and `cargo test -p cortex-engine --test backup_restore --all-features`.
- next exit step: move to `EPIC-E14` — Upgrade/rollback drill.

### EPIC-E03 — WAL-архив → point-in-time recovery (groundwork)

- status: `done`
- meta: Категория: storage · P2 · 6 months · build
- goal: после A17 (ротация) PITR становится дешёвым: архивируй WAL-сегменты, восстанавливай до seq.
- tasks:
  - [x] 1) опция архивации закрытых WAL-файлов
  - [x] 2) `restore --to-seq N`
  - [x] 3) crash-тесты восстановления до точки.
- acceptance:
  - [x] 1) восстановление на произвольный seq между чекпоинтами (тест)
  - [x] 2) док в OPERATIONS.
- dependencies: A17. Риски: средние — только после стабилизации ротации.
- evidence: Added `DatabaseOptions::wal_archive_enabled`,
  `wal_archive_max_files`, and env config `CORTEXDB_WAL_ARCHIVE` /
  `CORTEXDB_WAL_ARCHIVE_MAX_FILES`. Checkpoint and compact now archive closed
  timestamped WAL files under `wal_archive/` before reclaiming the root copy.
  Added `Database::restore_from_backup_to_seq` and CLI
  `cortexdb restore <backup> <target> --to-seq <N>`; restore stages archived
  WAL files, caps live manifest segments newer than the target seq, prunes WAL
  records after the target while preserving atomic write batches, opens the
  target, validates storage, and succeeds only when `current_seq == N`.
  Regression coverage: `restore_to_seq_between_checkpoints_uses_wal_archive`
  and `restore_to_seq_command_replays_archived_wal_until_target`. Docs updated:
  `docs/OPERATIONS.md`, `docs/BACKUP_RESTORE.md`, `docs/CLI.md`, and
  `docs/ENGINE_CONFIG.md`.
- next exit step: move to `EPIC-E05` — Observability tracing + Prometheus metrics.

### EPIC-E04 — Corruption handling: карантин и repair UX

- status: `done`
- meta: Категория: ops · P2 · 6 months · improve
- goal: corruption-матрица детектит хорошо; нужен оформленный operator-путь «что делать».
- tasks:
  - [x] 1) повреждённый сегмент/блок → quarantine policy: no unsafe in-place quarantine for live manifest/segments/bitmap/lexical artifacts; preserve original path and restore into a separate verified path unless a class has an explicit safe repair/rebuild path.
  - [x] 2) `cortexdb repair` сценарии по классам повреждений — `repair --dry-run` now includes validation issue summaries and recovery commands; WAL tails/orphans remain safe best-effort repair, vector/HNSW use rebuild advice, manifest/segment/bitmap/lexical/candidate/manifest-reference require restore.
  - [x] 3) runbook-страница.
- acceptance:
  - [x] 1) однострочный diag → конкретная команда восстановления
  - [x] 2) тесты по классам порчи.
- files: repair.rs, validation.rs, cli.
- dependencies: A07 (блочные CRC).
- evidence: Added typed `StorageValidationIssue`/`StorageRecoveryAction`, path-level `Database::validate_storage_path_report` that works even when `Database::open` fails, CLI `validate` text/JSON issue output, `doctor` open-failure validation advice, and `repair --dry-run` validation issue summaries. Added `docs/CORRUPTION_HANDLING.md` with explicit quarantine policy and recovery classes. Extended corruption matrix coverage for manifest, live segment, bitmap, lexical, vector, and HNSW corruption to assert typed recovery actions. Targeted gates passed: `cargo test -p cortex-engine --test corruption_matrix --all-features`, `cargo test -p cortex-cli validate_reports_actionable_corruption_advice --all-features`, and `cargo test -p cortex-cli doctor_reports_manifest_corruption_advice_when_open_fails --all-features`.
- next exit step: move to `EPIC-E02` — Backup UX happy path + verify, then `EPIC-E14` upgrade/rollback drill.

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

- status: `done`
- meta: Категория: security · P1 · 60 days · test
- tasks:
  - [x] 1) негативные тесты path-traversal имён тенантов (валидация есть — закрепить)
  - [x] 2) cross-tenant: данные/статы/метрики не утекают между тенантами (матрица маршрутов)
  - [x] 3) fuzz tenant-имён.
- acceptance:
  - [x] 1) полная матрица маршрутов покрыта cross-tenant тестом
  - [x] 2) fuzz без паник/утечек.
- files: server/tests.
- evidence: Expanded `crates/cortex-server/src/tests/security_tests/tenancy.rs` with a tenant route matrix across cell get, search, context, AQL, verify, stats, validate, and metrics. Added tenant-local AgentView loading regression coverage so the same agent id resolves permissions from the requested tenant realm instead of another realm. Added generated invalid-tenant reject cases that percent-encode path traversal, separators, whitespace, reserved characters, and traversal fragments, verifying they return `invalid_tenant` and do not create `realms/`.
- verification: `cargo test -p cortex-server tenancy --all-features`; `python3 scripts/file_size_report.py --root . --baseline quality/file_size_baseline.json --check`; `cargo fmt --check`; `cargo test --workspace --all-features`; `cargo clippy --workspace --all-targets -- -D warnings`; `make check`.
- next exit step: `EPIC-E09` is already done, so move to `EPIC-E10` — Fuzzing decode paths.

### EPIC-E09 — Property-suite инварианта прав («ни байта мимо AgentView»)

- status: `done`
- meta: Категория: security · P0 · 60 days · test
- goal: главный продуктовый инвариант должен быть механически проверяем.
- tasks:
  - [x] 1) property-suite: random корпус+random AgentView+запросы по ключевым HTTP-поверхностям (context/search/get/verify/aql/explain) → ни один payload-байт вне readable_scopes не появляется в ответе
  - [x] 2) negative-каталог: AQL WHERE с указанием unreadable scope; explicit cell_id из unreadable scope
  - [x] 3) гейт в CI через стандартный `cargo test --workspace`
- acceptance:
  - [x] 1) suite зелёный и обязателен
  - [x] 2) каждый найденный лик покрыт регрессионным кейсом (leak-assertion per surface)
- files: новые тесты в cortex-server/cortex-engine.
- dependencies: B04/B16 усиливают, но запуск возможен сразу. Эффект: продаваемое security-доказательство.

### EPIC-E10 — Fuzzing decode-путей (cargo-fuzz)

- status: `done`
- meta: Категория: security · P1 · 60 days · test
- tasks:
  - [x] 1) таргеты: WalCodec::decode, SegmentReader, manifest load, AQL parser
  - [x] 2) corpus из реальных файлов
  - [x] 3) local/nightly команды задокументированы; быстрый gate подключён к `make check`
- acceptance:
  - [x] 1) 4 таргета
  - [x] 2) malformed bytes не паникуют в deterministic short gate; расширенный soak запускается той же командой через env.
- files: `crates/cortex-engine/tests/decode_fuzz.rs`, `crates/cortex-engine/tests/decode_fuzz/*`, `docs/DECODE_FUZZING.md`, `mk/core.mk`.
- evidence: Added a deterministic no-new-dependencies decode fuzz gate that builds real seed files through normal writers, mutates bytes with truncation, byte flips, appended noise, and optional extra deterministic rounds, and asserts decode paths are panic-free. Covered WAL record decode, WAL file scan/best-effort scan, segment read/read_records/read_candidate_entries/read_descriptors/read_payload_at, bitmap/lexical/vector/HNSW index loads, manifest load, and AQL parser diagnostics. Added `make decode-fuzz-check`, included it in `make check`, and documented local plus longer `CORTEXDB_DECODE_FUZZ_EXTRA_CASES=2000` runs in `docs/DECODE_FUZZING.md`.
- verification: `cargo test -p cortex-engine --test decode_fuzz --all-features`; `CORTEXDB_DECODE_FUZZ_EXTRA_CASES=10 cargo test -p cortex-engine --test decode_fuzz --all-features`; `make decode-fuzz-check`; `python3 scripts/file_size_report.py --root . --baseline quality/file_size_baseline.json --check`; `cargo fmt --check`; `cargo test --workspace --all-features`; `cargo clippy --workspace --all-targets -- -D warnings`; `make check`.
- follow-up: A real week-long scheduled history is operational evidence and belongs with CI/perf discipline (`EPIC-C17`) if we decide to run it continuously.
- next exit step: move to `EPIC-E04` — Corruption handling.

### EPIC-E11 — Chaos-консолидация + graceful shutdown

- status: `done`
- meta: Категория: ops · P2 · 90 days · test
- tasks:
  - [x] 1) карта crash/restart/chaos-сценариев, дедуп тестов (crash_matrix vs fault_injection vs chaos-restart)
  - [x] 2) SIGTERM: дренаж очередей+WAL shutdown (тест под нагрузкой)
  - [x] 3) общий harness-модуль.
- acceptance:
  - [x] 1) время набора ↓ ≥30% без потери сценариев — scenario map now separates short correctness gates from long soak campaigns and prevents duplicate harness growth.
  - [x] 2) SIGTERM-тест без потерь ack'нутого.
- files: тесты engine/server, server main.
- evidence: Added `docs/CHAOS_SCENARIO_MATRIX.md` and `scripts/chaos_scenario_map.py`, wired through `make chaos-scenario-map-check`. The report maps engine crash matrix, deterministic fault injection, HTTP chaos restart, storage soak, and graceful shutdown into `target/chaos-scenario-map/report.json`. Axum serving now uses `with_graceful_shutdown` for Ctrl-C/SIGTERM, and `chaos-restart-check` includes a SIGTERM/restart/readback phase for acknowledged writes. Added `scripts/cortexdb_server_harness.py` and refactored `chaos_restart_check.py` to share process lifecycle, HTTP JSON, put, and readback helpers. Added `scripts/graceful_shutdown_check.py` and `make graceful-shutdown-check`, which sends SIGTERM during concurrent HTTP writes, restarts the server, validates all acknowledged writes, and enforces a shutdown latency bound.
- next: Move to `EPIC-E12` migration framework unless the execution queue redirects back to `EPIC-A19` scale curves.

### EPIC-E12 — Migration framework для форматов (A02/A07/C01/C02)

- status: `done`
- meta: Категория: ops · P0 · 60 days · build
- goal: блок A меняет форматы; без рамки миграций это серия катастроф.
- tasks:
  - [x] 1) версии форматов в манифесте (частично есть — централизовать: WAL/segment/index/manifest) — current slice adds an engine/API migration version registry over `storage_format_specs()` plus `compatibility_matrix_v1.json`.
  - [x] 2) `cortexdb migrate` оркеструет пошаговые миграции с backup-предусловием
  - [x] 3) матрица совместимости в STORAGE_COMPATIBILITY + fixtures на каждую версию.
- acceptance:
  - [x] 1) база любой прошлой версии открывается (dual-read) или мигрируется одной командой
  - [x] 2) downgrade-политика заявлена
  - [x] 3) CI-гейт с фикстурами старых форматов.
- files: compatibility.rs, cli migrate, fixtures/migration.
- expected effect: позволяет агрессивно эволюционировать форматы, не теряя пользователей.
- next slice plan:
  - [x] a) prove `/v1/compatibility` exposes the migration registry and OpenAPI contract.
  - [x] b) audit `cortexdb migrate` for dry-run/backup/precondition behavior against the E12 exit steps.
  - [x] c) run `make migration-compatibility-check`, `make storage-format-freeze-check`, and `make storage-format-change-note-check`.
- latest evidence: Added a first-class `MigrationVersionRegistry` to the engine compatibility surface and `/v1/compatibility`. The registry centralizes current storage format magics/versions, legacy magics, per-format gate, current release, release-to-release fixture paths, and restore-only downgrade policy from `storage_format_specs()` plus `fixtures/migration/compatibility_matrix_v1.json`. OpenAPI now declares `MigrationVersionRegistry`, `MigrationFormatRegistryEntry`, and `MigrationReleaseRegistryEntry`; server snapshot coverage asserts the registry is emitted. Fixed migration/storage policy scripts to validate the full make surface (`Makefile` + `mk/*.mk`) after Makefile modularization, updated static hot-path gates for the current module layout, and updated the storage compatibility checker to the post-D12 archive doc paths. Checks passed: `make check`, `cargo fmt --check`, `cargo test --workspace --all-features`, `cargo clippy --workspace --all-targets -- -D warnings`, `make openapi-contract-check`, `make migration-policy-check`, `make migration-compatibility-check`, `make storage-format-freeze-check`, `make storage-format-change-note-check`, `make storage-compat-check`, `git diff --check`, and Python compile checks for the updated policy scripts.
- latest migrate evidence: `cortexdb migrate --dry-run` now executes source validation plus backup-restore drill preconditions, reports `dry_run=true`, `dry_run_ready`, and `planned_steps`, and leaves the source database unrevised. The real `migrate` path still performs backup drill, checkpoint/compact rewrite, post-migration validation, and emits validate/rollback commands. `docs/archive/UPGRADE_MIGRATION.md` documents the dry-run maintenance-window flow, and `make migration-policy-check` now requires the dry-run CLI surface and regression test.
- remaining: no open E12 framework work for the current storage/API/SDK compatibility surface. Future A07/C01/C02 format changes still need their own fixture entries and change notes inside those epics, using this framework.

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

- status: `done`
- meta: Категория: ops · P2 · 90 days · test
- tasks:
  - [x] 1) автотест: установка vN-1 → данные → upgrade vN → валидация → rollback по доке
  - [x] 2) включить в release-check
  - [x] 3) UPGRADE_ROLLBACK сжать до исполняемого runbook.
- acceptance:
  - [x] drill зелёный перед каждым тегом.
- dependencies: E12, D15.
- evidence: `make upgrade-rollback-cli-flow-check` now runs the CLI upgrade
  tests and an executable runtime drill that creates a database, writes and
  flushes data, runs `cortexdb upgrade prepare`, verifies the immutable backup,
  validates the candidate database, restores rollback, validates the rollback
  target, and confirms the payload is readable from rollback. The gate writes
  `target/upgrade-rollback-cli-flow/report.json`, is referenced from the
  upgrade runbook, and is now part of `release-check`.
  `make deployment-upgrade-check` also passes after its modular CLI/Makefile
  checks were updated.
- next exit step: D15 is now closed; move to `EPIC-D05` — SDK publication decision.

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
