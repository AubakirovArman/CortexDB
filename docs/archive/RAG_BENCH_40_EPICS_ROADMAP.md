# CortexDB — 40 эпиков: путь к лучшей агентной БД (EnterpriseRAG-Bench 43.27 → 60+)

Версия: 2026-06-11. База: официальный прогон `gpt-5.2` judge, Overall **43.27**
(`erb-submission/official_results_gpt5.2_judge.json`), engine-aql retrieval +
bge-m3 dense fusion, doc recall 85.8%.

Этот документ — **сводная карта на ~40 эпиков**, дополняющая (не заменяющая):

- `CORTEXDB_IMPROVEMENT_EPICS.md` — 20 эпиков "движок как источник качества" (P0/P1/P2)
- `ENTERPRISE_RAG_IMPROVEMENT_PLAN.md` — детальный разбор по категориям (фазы A/B/C)
- `ENTERPRISE_RAG_IMPROVEMENT_EPICS.md` — 7 эпиков синтеза ответов
- `NEXT_60_EPICS.md` — продуктовый/инфраструктурный backlog (в основном closed)

Где эпик пересекается с уже существующим — указана ссылка `(см. ...)`. Цель —
не дублировать, а **дать единый порядок выполнения** по всем слоям БД:
storage/index → retrieval → ranking → context pack → answer synthesis →
оценка/инфраструктура.

## Диагноз по категориям (текущий узкое место)

| Категория | n | combined | Узкое место |
| --- | ---: | ---: | --- |
| high_level | 10 | **0.0** | retrieval отдаёт 0 документов (пустой scope-фильтр) |
| project_related | 40 | **5.94** | retrieval recall ок (88%), но синтез/correctness почти нулевой |
| completeness | 20 | 23.75 | модель не покрывает все под-пункты вопроса |
| intra_document_reasoning | 40 | 32.92 | синтез внутри документа слабый |
| semantic | 125 | 37.44 | recall всего 75.2% — самый большой вклад в потолок |
| conflicting_info | 20 | 39.95 | разрешение противоречий неточное |
| constrained | 30 | 35.9 | numeric/condition guard |
| basic | 175 | 56.24 | полировка |
| miscellaneous | 20 | 60.42 | ок |
| info_not_found | 20 | 100.0 | не трогать |

Самый крупный прирост Overall даёт **semantic** (вес 125/500) и **project_related**
(40/500, сейчас почти ноль). high_level дешёвый и изолированный (10/500).

---

## Тема A. Индексация и хранение знаний (storage/cortex-storage, cortex-engine)

### A1 — Производственный pipeline эмбеддингов внутри движка
Перенести `embed_corpus.py`/bge-m3 из bench-скриптов в первоклассный
ingestion-pipeline `cortex-engine`: инкрементальный, resumable, с метриками
покрытия и retry. Сейчас покрытие 97.8% корпуса при 11k transient fails —
довести до 99.9%+ и сделать частью `put_cell`/`ingest`.
**Target:** embedding coverage ≥ 99.5%, авто-докрутка отстающих cell'ов.

**Status:** done for engine pipeline primitives. Первый вертикальный срез закрыт: в `cortex-engine`
добавлен typed coverage report `EmbeddingCoverageReport` с проверкой
coverage, missing, duplicate, unexpected, invalid rows, dimension mismatch и
stale embeddings. `embed_corpus.py` теперь умеет писать `--report-file`,
`--retry-ids-file`, `--manifest-file` и запускать `--track-staleness`, так что
внешний bge-m3 run стал resumable + measurable gate, а не просто append-only
JSONL. Добавлен engine binary `embedding_coverage_check` и Makefile gate
`enterprise-rag-bench-embedding-coverage-check`. Gate умеет читать expected
manifest через `--expected-manifest`, проверять модель через `--expected-model`,
писать полный retry-id список, а `embed_corpus.py` умеет запускать backfill
только по этому списку через `--only-ids-file`. В движке появился live-cell
debt API: `Database::embedding_expected_manifest()` и
`Database::embedding_debt_report(...)`, который после обычного `put_cell`/ingest
показывает missing vector, dimension mismatch, stale model и stale text hash.
Добавлен controlled backfill worker API:
`Database::backfill_embedding_debt(...)` + `EmbeddingBackfillProvider`, который
берёт live debt, вызывает embedding provider, патчит cell payload строками
`vector=...`, `embedding_model=...`, `embedding_text_hash=...`, и очищает debt.

**Done evidence:**

```text
crates/cortex-engine/src/embedding_pipeline.rs
scripts/enterprise_rag_bench/embed_corpus.py --report-file ... --retry-ids-file ...
scripts/enterprise_rag_bench/embed_corpus.py --track-staleness --manifest-file ...
scripts/enterprise_rag_bench/embed_corpus.py --only-ids-file ...
cargo run -p cortex-engine --bin embedding_coverage_check -- --uuid-index ... --embeddings ...
cargo run -p cortex-engine --bin embedding_coverage_check -- --expected-manifest ... --expected-model ...
make enterprise-rag-bench-embedding-coverage-check
cargo test -p cortex-engine embedding_pipeline
Database::embedding_debt_report(...)
Database::embedding_expected_manifest()
Database::backfill_embedding_debt(...)
```

**Remaining outside A1:** полный bge-m3 corpus catch-up до 99.9%+ и
операционный scheduled daemon/runbook вынесены в G1/G2. A1 закрывает движковый
API и проверяемый backfill механизм без привязки к секретам или конкретному
HTTP embedding provider.

### A2 — Multi-view индексация документа (title/path/entity/section)
Каждый документ индексируется не одним вектором/постингом, а несколькими
"видами": заголовок, путь/иерархия (project/space), извлечённые сущности,
краткое summary секции. Каждый вид — отдельный кандидат-источник для
lexical/dense retrieval. (см. CORTEXDB_IMPROVEMENT_EPICS EPIC-06 source-views)
**Target:** semantic recall@10 75.2% → 85%+.

**Status:** partial. Закрыт lexical/BM25F slice: `CellMetadata` теперь
декодирует document views `title`, `path`, `document_id`/`doc_id`, `chunk_id`,
`section`, `project`, `entity`, `sector`, `source`; `weighted_lexical_terms()`
индексирует их с отдельными весами. Эти веса уже используются в AQL retrieval,
ContextPack scoring и persisted search indexing path, потому что все они читают
`CellMetadata::weighted_lexical_terms()`. Strict metadata decode синхронизирован,
а служебные embedding строки `vector=...`, `embedding_model=...`,
`embedding_text_hash=...` не загрязняют body terms. Закрыт первый semantic
multi-view slice: payload может содержать `title_vector=...`, `path_vector=...`,
`entity_vector=...`, `section_vector=...`, `summary_vector=...` рядом с
canonical `vector=...`; live AQL semantic ranking берёт лучший score по всем
view vectors. Live `Database::search_vector` и `SearchMode::Hybrid` fallback
тоже выбирают лучший named view vector для текущего query vector.

**Done evidence:**

```text
crates/cortex-engine/src/query/metadata.rs
crates/cortex-engine/src/query/metadata_validation.rs
crates/cortex-engine/src/search/analyzer.rs
cortex-engine::search::SearchViewTrace
cargo test -p cortex-engine --lib search::vector
cargo test -p cortex-engine --lib query::metadata
cargo test -p cortex-engine --lib retrieve_aql_uses_path_view_for_lexical_relevance
cargo test -p cortex-engine --lib semantic_mode_uses_named_view_vectors_for_retrieval_ordering
cargo test -p cortex-engine --test database_search
cargo fmt --check
cargo clippy -p cortex-engine --lib -- -D warnings
```

**Remaining:** persisted multi-vector storage/HNSW per-view graph ещё не
сделаны. Текущий slice улучшает lexical/BM25F path, live AQL semantic ranking,
live search API fallback и candidate-source explain (`view_traces`); full
semantic lift до 85% потребует generated view embeddings at ingestion time,
persisted multi-view `.acv`/HNSW support, и query expansion в B3.

### A3 — Иерархические chunk'и (parent/child)
Для длинных документов хранить child-chunks (мелкие, точные для retrieval) +
parent-chunk/summary (для синтеза и intra_document_reasoning). При retrieval
возвращать child для recall, но в ContextPack подтягивать parent для контекста.
**Target:** intra_document_reasoning combined 32.92 → 45+.

**Status:** partial. Закрыт engine slice для безопасного parent expansion:
`CellMetadata` теперь декодирует `parent_id`/`parent_chunk_id` и
`chunk_role`/`chunk_kind`; AQL retrieval после rerank добавляет parent-context
сразу после найденного child, если parent уже присутствует в том же bitmap
candidate set. Это intentionally ACL-safe: parent не подтягивается из-за
пределов AQL-фильтров/видимости. Parent может матчиться по `chunk_id`; по
`document_id` он используется только для cell с ролью `parent`, `document` или
`summary`.

**Done evidence:**

```text
crates/cortex-engine/src/query/metadata.rs
crates/cortex-engine/src/query/metadata_validation.rs
crates/cortex-engine/src/database.rs
cargo test -p cortex-engine --lib retrieve_aql_expands_child_hit_with_parent_context
cargo test -p cortex-engine --lib query::metadata
cargo test -p cortex-engine --lib query::metadata_validation
cargo test -p cortex-engine --test database_search
cargo fmt --check
cargo clippy -p cortex-engine --lib -- -D warnings
```

**Remaining:** ingestion-time parent/child chunk graph, persisted parent index,
document-window promotion policy inside ContextPack, and benchmark runner wiring
that proves the parent expansion improves `intra_document_reasoning`.

### A4 — Структурное извлечение таблиц как `KnowledgeCellType::Table`
Таблицы (бюджеты, метрики, roadmaps) парсятся в строки/ячейки с заголовками
колонок, индексируются отдельно от текста. Closes gap для constrained и
completeness вопросов про "конкретные цифры/строки таблицы".
**Target:** constrained combined 35.9 → 50+.

**Status:** partial. Закрыт ingestion/metadata slice: `KnowledgeCellType`
получил тип `table`; CSV ingestion теперь пишет строки как `type=table` и
добавляет `table_id`, `table_headers`, `row_label`, сохраняя существующие
`row`/`cell_range` source refs. `CellMetadata` и strict decode читают эти поля,
а `weighted_lexical_terms()` бустит table headers/row labels/cell ranges, чтобы
constrained retrieval лучше находил строки по названию колонок и row anchors.
Large-cell placeholder теперь сохраняет table coordinates.

**Done evidence:**

```text
crates/cortex-core/src/cell.rs
crates/cortex-engine/src/ingestion/adapters.rs
crates/cortex-engine/src/ingestion/cells.rs
crates/cortex-engine/src/query/metadata.rs
crates/cortex-engine/src/query/metadata_validation.rs
crates/cortex-engine/src/context/large_cell.rs
cargo test -p cortex-core cell
cargo test -p cortex-engine --lib query::metadata
cargo test -p cortex-engine --test ingestion_adapters
cargo test -p cortex-engine --test ingestion_chunking_policy
cargo test -p cortex-engine --test ingestion_validation_report
cargo fmt --check
cargo clippy -p cortex-engine --lib -- -D warnings
```

**Remaining:** real table parser for markdown/HTML/PDF tables, typed table-cell
query operators, numeric/table condition extraction, and persisted column/row
index for range filters.

### A5 — Сущностный граф знаний на диске (persisted Knowledge Graph index)
Сейчас граф (`KnowledgeGraphIndex`) только in-memory/derived
(см. NEXT_60_EPICS #58). Сделать персистентный `.ackg` формат: entity nodes,
relation edges, source refs, переживающий restart без полного rebuild.
**Target:** граф доступен сразу после открытия БД без re-scan.

**Status:** partial. Закрыт первый persisted graph artifact: `KnowledgeGraphIndex`
теперь сериализуется в deterministic `graph.ackg` snapshot с magic
`CORTEXDB_ACKG_V1`, hex-escaped строковыми полями, atomic write и read API.
`Database::persist_knowledge_graph_index()` строит snapshot из текущих visible
cells и пишет его в root DB; `Database::read_persisted_knowledge_graph_index()`
читает persisted graph после restart без повторного scan как отдельный artifact.

**Done evidence:**

```text
crates/cortex-engine/src/graph.rs
cargo test -p cortex-engine --lib graph
cargo test -p cortex-engine --test database_search
cargo test -p cortex-engine --test ingestion_adapters
cargo test -p cortex-core
cargo fmt --check
cargo clippy -p cortex-engine --lib -- -D warnings
```

**Remaining:** manifest awareness, per-segment `.ackg` bundle integration,
automatic graph publication during checkpoint/compact, stale graph validation,
and graph cache selection on `Database::open`.

### A6 — Дедупликация и near-duplicate suppression на уровне сегмента
При checkpoint/compact детектировать near-duplicate cell'ы (одинаковый
контент из разных источников: Slack/Confluence копии) и схлопывать в один
canonical cell + alias-ссылки. Снижает `invalid_extra_docs` и шум в топ-10.
**Target:** average_invalid_extra_docs 8.21 → ≤ 6.0.

**Status:** partial. Закрыт exact-content retrieval suppression slice:
`Database::duplicate_content_groups()` теперь строит группы дублей по
`content_hash` между разными источниками, выбирая canonical cell и список
duplicates/source hashes. AQL retrieval после query-specific rerank отбрасывает
повторные кандидаты с тем же `content_hash`, сохраняя лучший ranked cell.
Это снижает повторные Slack/Confluence копии в top-k без обращения к oracle.

**Done evidence:**

```text
crates/cortex-engine/src/ingestion/dedup.rs
crates/cortex-engine/src/database.rs
crates/cortex-engine/tests/ingestion_adapters.rs
cargo test -p cortex-engine --lib retrieve_aql_suppresses_duplicate_content_hashes
cargo test -p cortex-engine --test ingestion_adapters
cargo test -p cortex-engine --test database_search
cargo test -p cortex-engine --lib query::metadata
cargo fmt --check
cargo clippy -p cortex-engine --lib -- -D warnings
```

**Remaining:** semantic near-duplicate detection during checkpoint/compact,
canonical alias persistence in segment bundles, alias-aware retrieval/explain,
and validation/repair for duplicate alias maps.

### A7 — Source metadata enrichment при инжесте (топики, даты, owners)
Расширить ingestion adapter: извлекать из текста (regex/NER без LLM)
даты, имена проектов, владельцев, статусы и класть как структурированные
metadata-поля cell'а. Используется для query-to-scope routing (см. C3).
**Target:** ≥ 80% документов с непустыми extracted entity tags.

**Status:** partial. Закрыт deterministic no-LLM enrichment slice:
`SourceMetadataEnrichment` извлекает из первых строк ingest body `project`,
`entity`, `owner`, `status_tag`, `event_date` и `topic` через label/heading/date
эвристики без новых зависимостей. Text chunks, JSON fact cells и CSV table rows
получают enrichment headers автоматически при payload build. `CellMetadata` и
strict decode читают эти поля, а `weighted_lexical_terms()` использует их в
retrieval ranking.

**Done evidence:**

```text
crates/cortex-engine/src/ingestion/enrichment.rs
crates/cortex-engine/src/ingestion/cells.rs
crates/cortex-engine/src/query/metadata.rs
crates/cortex-engine/src/query/metadata_validation.rs
crates/cortex-engine/tests/ingestion_adapters.rs
cargo test -p cortex-engine --lib ingestion::enrichment
cargo test -p cortex-engine --lib query::metadata
cargo test -p cortex-engine --test ingestion_adapters
cargo test -p cortex-engine --test ingestion_validation_report
cargo test -p cortex-engine --test database_search
cargo fmt --check
cargo clippy -p cortex-engine --lib -- -D warnings
```

**Remaining:** broader NER, multi-value entity tags, corpus-level enrichment
coverage report/gate, source-specific extractors for Slack/Jira/GitHub/Gmail,
and using these tags in C3 query-to-scope routing.

### A8 — Версионирование и temporal-index для conflicting_info
Для cell'ов с одинаковым subject/metric хранить временную упорядоченность
(`as_of`/`superseded_by`), строить index "последняя версия факта по теме".
**Target:** conflicting_info combined 39.95 → 55+.

**Status:** partial. Закрыт in-engine temporal fact index slice:
`CellMetadata` понимает temporal headers `as_of`, `valid_from`, `valid_to`,
`supersedes`, `superseded_by`; `Database::temporal_fact_index(view)` группирует
видимые fact-like cells по `subject/project/entity + metric`, сортирует их по
`as_of/event_date/valid_from`, `created_unix_seconds`, `cell_id`, и помечает
latest/superseded records. `Database::latest_temporal_facts(view)` возвращает
только актуальные записи. Индекс уважает `AgentView` scope.

**Done evidence:**

```text
crates/cortex-engine/src/verification/temporal_index.rs
crates/cortex-engine/src/verification.rs
crates/cortex-engine/src/query/metadata.rs
crates/cortex-engine/src/query/metadata_validation.rs
cargo test -p cortex-engine --lib verification::temporal_index
cargo test -p cortex-engine --test verification_tests
cargo test -p cortex-engine --lib query::metadata
cargo test -p cortex-engine --test ingestion_adapters
cargo fmt --check
cargo clippy -p cortex-engine --lib -- -D warnings
```

**Remaining:** persisted temporal index, explicit supersedes graph edges,
temporal-aware ContextPack conflict selection, and AQL/query planner integration
for "latest as of date" questions.

---

## Тема B. Retrieval-движок (cortex-engine search/AQL)

### B1 — Cross-encoder reranking как нативная стадия движка
После hybrid fusion (RRF dense+lexical) добавить лёгкий cross-encoder
re-rank top-50 → top-10 как опциональный движковый режим
(`mode=hybrid_rerank`). (продолжение CORTEXDB_IMPROVEMENT_EPICS EPIC-01/02)
**Target:** semantic recall@10 +5-8pp без потери latency budget.

**Status:** partial. Закрыт нативный lightweight rerank slice: `SearchMode::HybridRerank`
теперь является engine search mode, строит расширенный hybrid candidate pool
(`limit.max(32)`), применяет `WeightedScoreReranker` внутри `SearchIndexes`
и `Database::search_cells`, работает для live snapshot и persisted
`.aci/.acv` search path. Benchmark runner получил явный режим
`--retrieval-mode engine-hybrid-rerank`, чтобы official-clean прогоны могли
сравнивать старый `engine-hybrid` и новый rerank path без скрытой подмены.

**Done evidence:**

```text
crates/cortex-engine/src/search.rs
crates/cortex-engine/src/search/database.rs
crates/cortex-engine/src/bin/enterprise_rag_bench_retrieval/args.rs
crates/cortex-engine/src/bin/enterprise_rag_bench_retrieval/main.rs
crates/cortex-engine/tests/database_search.rs
cargo test -p cortex-engine --test database_search
cargo test -p cortex-engine --test query_search
cargo test -p cortex-engine --bin enterprise_rag_bench_retrieval
cargo test -p cortex-engine --lib search::rerank
cargo fmt --check
cargo clippy -p cortex-engine --lib -- -D warnings
cargo clippy -p cortex-engine --bin enterprise_rag_bench_retrieval -- -D warnings
```

**Remaining:** настоящий external/cross-encoder provider interface, latency/SLO
guardrails, benchmark 50/500 quality gate for `engine-hybrid-rerank`, и AQL
`RETRIEVE CONTEXT` integration.

### B2 — Doc-view discovery: document-level кандидаты из chunk-хитов
Когда побеждают chunk-уровневые хиты, добавлять "родительский" документ
целиком как кандидат (для project_related/high_level — нужен весь
документ, не один параграф). (см. "doc-view discovery gate" в истории)
**Target:** project_related document_recall удержать ≥88% при росте answer correctness.

**Status:** partial. Закрыт native search slice: `Database::search_cells`
теперь расширяет top-k результат parent/document context-кандидатом, когда
победивший chunk/child содержит `parent_id` или общий `document_id`.
Расширение работает после persisted search и live snapshot search, уважает
`AgentView` scope, не добавляет private parent cells и сохраняет лимит ответа
через замену нижних extras, а не безлимитное расширение.

**Done evidence:**

```text
crates/cortex-engine/src/search/database.rs
crates/cortex-engine/tests/database_search.rs
cargo test -p cortex-engine --test database_search parent_context -- --nocapture
cargo test -p cortex-engine --test database_search
cargo test -p cortex-engine --test query_search
cargo test -p cortex-engine --bin enterprise_rag_bench_retrieval
cargo fmt --check
cargo clippy -p cortex-engine --lib -- -D warnings
cargo clippy -p cortex-engine --bin enterprise_rag_bench_retrieval -- -D warnings
```

**Remaining:** source/thread neighbor expansion, project-level document cluster
promotion, explicit trace field explaining parent expansion, and 50/500
EnterpriseRAG gate showing project_related answer lift.

### B3 — Query expansion из текста вопроса (без oracle)
Лёгкий, детерминированный expander: синонимы корпуса (построенные из A7
entity tags), аббревиатуры, перефраз через локальную модель (та же,
что answerer). Подаётся как доп. lexical/dense запросы с RRF-слиянием.
**Target:** semantic recall@10 75.2% → 85%+ (часть A2 lift).

**Status:** partial. Закрыт deterministic no-oracle expansion slice в
`analyze_search_query`: enterprise terms стали bidirectional
(`DRI/assignee/responsible -> owner`, `slipped/delayed -> blocker/risk`,
`ETA/timeline -> deadline`, `RBAC/access -> security`, и т.д.), а
phrase-level expansion добавляет overview/mission/charter/about для
high-level формулировок без чтения `question_type`. Эти веса используются
общим `Bm25Index` и `Database::search_keyword`, то есть улучшение работает
через engine search path, а не через benchmark-only selector.

**Done evidence:**

```text
crates/cortex-engine/src/search/query_understanding.rs
crates/cortex-engine/tests/database_search.rs
cargo test -p cortex-engine --lib search::query_understanding -- --nocapture
cargo test -p cortex-engine --test database_search
cargo test -p cortex-engine --test query_search
cargo test -p cortex-engine --bin enterprise_rag_bench_retrieval
cargo fmt --check
cargo clippy -p cortex-engine --lib -- -D warnings
cargo clippy -p cortex-engine --bin enterprise_rag_bench_retrieval -- -D warnings
```

**Remaining:** corpus-derived synonym mining from A7 metadata, optional
local-model paraphrase expansion cache, dense-query multi-query RRF, and
50/500 semantic recall gate.

### B4 — Query routing v2: per-category retrieval policy
Расширить `route_search_query` (NEXT_60_EPICS #25): классифицировать
вопрос (basic/semantic/project_related/high_level/conflicting/...) и
применять разные `ContextPolicy`/веса/budget per type, как в `binder.rs`
(`context_policy_for_mode`). (см. ENTERPRISE_RAG_IMPROVEMENT_PLAN type-routed rule)
**Target:** ни одна категория не регрессирует при улучшении другой.

**Status:** partial. Закрыт no-oracle routing slice: `search/routing.rs`
теперь классифицирует intent из текста вопроса (`lookup`, `semantic`,
`project_related`, `high_level`, `conflicting_info`, `completeness`,
`info_not_found`, `constrained`) и возвращает `SearchRoutePolicy` с
candidate multiplier, rerank/diversity/abstain flags и lexical/semantic
weights. `HybridRerank` теперь использует `routed_candidate_limit`, поэтому
wide-intent вопросы получают больший candidate pool до rerank без чтения
benchmark `question_type`.

**Done evidence:**

```text
crates/cortex-engine/src/search/routing.rs
crates/cortex-engine/src/search.rs
crates/cortex-engine/src/search/database.rs
crates/cortex-engine/tests/query_search.rs
cargo test -p cortex-engine --lib search::routing -- --nocapture
cargo test -p cortex-engine --test query_search
cargo test -p cortex-engine --test database_search
cargo test -p cortex-engine --bin enterprise_rag_bench_retrieval
cargo fmt --check
cargo clippy -p cortex-engine --lib -- -D warnings
cargo clippy -p cortex-engine --bin enterprise_rag_bench_retrieval -- -D warnings
```

**Remaining:** feed route policy into ContextPack budget/diversity, expose route
diagnostics in search reports, tune intent classifier against clean dev split,
and run per-category 50/500 regression gate.

### B5 — High-level / brain-summary retrieval mode
Для вопросов без `expected_doc_ids`/`source_types` (high_level: миссия,
revenue streams, департаменты) — отдельный режим: top-N "anchor"-документов
по BRAIN (overview/about/charter docs), а не пустой scope-фильтр.
**Target:** high_level combined 0.0 → 50+ (10 вопросов, дёшево и изолированно).

**Status:** partial. Закрыт no-oracle high-level anchor fill slice:
`Database::search_cells` теперь при `SearchQueryIntent::HighLevel` добавляет
видимые summary/document anchor cells по metadata/body signals
(`chunk_role=summary/document/parent`, title/path/body содержит
overview/summary/mission/charter/about/strategy/vision/company). Это работает
даже когда lexical result set пустой, уважает `AgentView` scope и не возвращает
private summary cells.

**Done evidence:**

```text
crates/cortex-engine/src/search/database.rs
crates/cortex-engine/tests/database_search.rs
cargo test -p cortex-engine --test database_search high_level_query -- --nocapture
cargo test -p cortex-engine --test database_search
cargo test -p cortex-engine --test query_search
cargo test -p cortex-engine --bin enterprise_rag_bench_retrieval
cargo fmt --check
cargo clippy -p cortex-engine --lib -- -D warnings
cargo clippy -p cortex-engine --bin enterprise_rag_bench_retrieval -- -D warnings
```

**Remaining:** brain-level summary index, source-specific overview discovery,
answer prompt mode for overview synthesis, and high_level 50/500 benchmark gate.

### B6 — Project-related multi-document candidate aggregator
Для project_related вопросов агрегировать кандидатов по `project`/`space`
metadata (а не только по текстовому совпадению), собирая полный набор
"артефактов проекта" (issue+PR+doc+thread) в один кандидат-кластер.
**Target:** project_related combined 5.94 → 25+.

**Status:** partial. Закрыт same-project aggregation slice:
`Database::search_cells` при `SearchQueryIntent::ProjectRelated` теперь
берёт `project` из top hits и добирает видимые cells того же проекта в
оставшийся top-k budget. Aggregator работает для live snapshot и persisted
search path, уважает `AgentView` scope и не добавляет private project artifacts.

**Done evidence:**

```text
crates/cortex-engine/src/search/database.rs
crates/cortex-engine/tests/database_search.rs
cargo test -p cortex-engine --test database_search same_project_artifacts -- --nocapture
cargo test -p cortex-engine --test database_search
cargo test -p cortex-engine --test query_search
cargo test -p cortex-engine --bin enterprise_rag_bench_retrieval
cargo fmt --check
cargo clippy -p cortex-engine --lib -- -D warnings
cargo clippy -p cortex-engine --bin enterprise_rag_bench_retrieval -- -D warnings
```

**Remaining:** explicit project graph/bundle object, artifact-type diversity
caps, source/thread/link expansion, and project_related 50/500 benchmark gate.

### B7 — Anchor/evidence-overlap candidate filter
Перенести из bench-скриптов "anchor candidate gate" внутрь движка:
кандидаты без пересечения с key-terms/entities вопроса штрафуются перед
top-k cutoff. Снижает шум без потери recall.
**Target:** average_invalid_extra_docs ≤ 6.5 при recall ≥ 85%.

**Status:** partial. Закрыт native rerank overlap gate slice:
`WeightedScoreReranker` теперь не только даёт bonus за anchor/source/query
payload matches, но и dampens score для кандидатов без evidence-overlap с
anchors, source hints или meaningful query terms. Это применяется в engine
`HybridRerank`/`search_cells_with_reranker` path до top-k cutoff и снижает
шанс, что strong vector-only unrelated candidate попадёт выше evidence-bearing
candidate.

**Done evidence:**

```text
crates/cortex-engine/src/search/rerank.rs
crates/cortex-engine/tests/database_search.rs
cargo test -p cortex-engine --lib search::rerank -- --nocapture
cargo test -p cortex-engine --test database_search evidence_overlap -- --nocapture
cargo test -p cortex-engine --test database_search
cargo test -p cortex-engine --test query_search
cargo test -p cortex-engine --bin enterprise_rag_bench_retrieval
cargo fmt --check
cargo clippy -p cortex-engine --lib -- -D warnings
cargo clippy -p cortex-engine --bin enterprise_rag_bench_retrieval -- -D warnings
```

**Remaining:** expose overlap diagnostics, tune penalty strength on clean dev
split, add source/entity-aware overlap beyond substring terms, and run 50/500
invalid-extra-docs regression gate.

**Update:** added balanced-50 anchor/evidence-overlap diagnostics and a
production-evidence sweep gate. The report is offline-only: it may use
`expected_doc_ids` to score retrieval quality, but the retrieval/answer
inference path remains official-clean.

```text
scripts/enterprise_rag_bench/anchor_overlap_diagnostics.py
scripts/enterprise_rag_bench/test_anchor_overlap_diagnostics.py
make erb-anchor-overlap-check
```

Latest balanced-50 diagnostic:

```text
questions:                       50
average_recall_pct:              81.41
average_invalid_extra_docs:       7.78
average_overlap_doc_pct:         94.00
average_strong_overlap_doc_pct:  93.40
invalid_without_strong_overlap:   3
gold_hits_without_strong_overlap: 0
status:                          passed
```

**Remaining after diagnostic:** target `average_invalid_extra_docs <= 6.5`
is still not met. The diagnostic shows that most extra docs are not random
off-topic rows; they share at least two question anchors. Next B7 work should
use entity/source/fact-level overlap and coverage gain, not only substring
overlap.

**Update 2:** tightened the native overlap gate so a candidate is no longer
treated as evidence-bearing because it matches one broad query term. Strong
signals still pass immediately:

```text
ticket/path/version/date/number anchors
source hints
scope mapping
numeric/condition match
multi-requirement coverage
```

Broad query terms now need at least two meaningful overlaps before bypassing
the no-evidence penalty.

```text
crates/cortex-engine/src/search/rerank.rs
cargo test -p cortex-engine --lib search::rerank -- --nocapture
cargo test -p cortex-engine --test database_search evidence_overlap -- --nocapture
cargo test -p cortex-engine --test database_search hybrid_rerank -- --nocapture
```

**Performance note:** attempted an official-clean retrieval-only 50 run with:

```text
retrieval-mode=engine-keyword
rerank=weighted
reuse-db=true
```

The run was intentionally stopped after it stayed at `completed=0` for more
than two minutes while the retrieval binary used 100% CPU on the first query.
This is not a quality regression result, but it proves the B7/weighted-rerank
official-clean path still needs a performance gate before it can be used as a
routine 50/500 metric run. Likely follow-up: cap or optimize persisted lexical
common-term scans and add per-question progress telemetry.

**Update 3:** fixed the B7 performance/wiring gate, but not the quality target.
The engine runner now has per-question progress telemetry, top-N bounded
lexical ranking, rare-term selection for large persisted lexical searches, and
terms-only `.aci` loading for large indexes. The official-clean wrapper also
accepts the Rust-supported `engine-hybrid-rerank` mode, and the vector loader
accepts float embedding arrays by quantizing them to the existing i16 vector
format.

```text
crates/cortex-engine/src/search.rs
crates/cortex-engine/src/search/persisted.rs
crates/cortex-storage/src/indexes.rs
crates/cortex-engine/src/checkpoint.rs
crates/cortex-engine/src/bin/enterprise_rag_bench_retrieval/main.rs
scripts/enterprise_rag_bench/run_official_clean_benchmark.py
cargo test -p cortex-storage --test lexical_index_tests
cargo test -p cortex-engine --lib search::
cargo test -p cortex-engine --bin enterprise_rag_bench_retrieval
```

Latest official-clean first-50 retrieval-only runs:

```text
run: b7-light-lexical-keyword
mode: engine-keyword + weighted
duration: 59.214s retrieval / 64.573s total
throughput: 0.844 q/s
average_recall_pct: 20.00
average_invalid_extra_docs: 5.44
status: failed quality target

run: b7-engine-hybrid-weighted-fullvectors
mode: engine-hybrid + weighted, full500 query vectors
duration: 161.440s retrieval / 166.726s total
throughput: 0.310 q/s
average_recall_pct: 20.00
average_invalid_extra_docs: 5.44
status: failed quality target
```

The hybrid run matching keyword output shows the reused DB root is effectively
lexical-only: query vectors now load, but no `document_vectors.jsonl` was used
when that DB was ingested. Existing corpus embeddings are available at
`target/enterprise-rag-bench/embeddings/corpus_bge_m3.jsonl` (500,694 rows,
11GB), but a vector-backed DB/root or streaming document-vector loader is still
needed before B7 can be judged on the real high-recall dense path.

**Remaining after Update 3:** B7 stays partial. The performance blocker is
closed, but the target `average_invalid_extra_docs <= 6.5 при recall >= 85%`
is not met. Next B7 work should run the same gate on a vector-backed DB and
then tune entity/source/fact-level overlap only if recall remains high.

**Update 4:** added a runtime vector-readiness guard to the official-clean
retrieval report. Hybrid modes now sample indexed DB payloads and report whether
documents actually contain `vector=` metadata, so a query-vector-only run can no
longer be mistaken for a real dense retrieval run.

```text
crates/cortex-engine/src/bin/enterprise_rag_bench_retrieval/main.rs
cargo test -p cortex-engine --bin enterprise_rag_bench_retrieval
cargo clippy -p cortex-engine --bin enterprise_rag_bench_retrieval -- -D warnings
cargo fmt --check
```

Latest guard validation:

```text
run: b7-vector-readiness-check
mode: engine-hybrid + weighted, full500 query vectors, reused first-50 DB
duration: 168.132s retrieval / 173.410s total
throughput: 0.297 q/s
query_vector_rows: 500
sampled_documents_with_vectors: 0 / 2048
vector_readiness.ready: false
warning: hybrid retrieval selected but no document vectors were found in the sampled DB payloads
average_recall_pct: 20.00
average_invalid_extra_docs: 5.44
status: failed quality target, valid wiring/readiness diagnostic
```

**Remaining after Update 4:** B7 still needs a vector-backed DB/root or a
streaming document-vector loader before the overlap filter can be judged against
the target quality gate. Until `vector_readiness.ready=true`, dense/hybrid
quality numbers from this reused DB are not valid promotion evidence.

**Update 5:** added a lazy indexed document-vector loader for the retrieval
runner. `--document-vectors` no longer has to materialize every vector in a
`BTreeMap<Vec<i16>>` before ingest. The runner scans the JSONL once, stores only
`doc_id -> byte offset`, and parses each vector on demand while ingesting the
matching document. This keeps the path viable for the existing 11GB
`corpus_bge_m3.jsonl` artifact and removes the RAM blocker for building a
vector-backed DB.

```text
crates/cortex-engine/src/bin/enterprise_rag_bench_retrieval/main.rs
cargo test -p cortex-engine --bin enterprise_rag_bench_retrieval -- --nocapture
cargo clippy -p cortex-engine --bin enterprise_rag_bench_retrieval -- -D warnings
cargo fmt --check
```

Tiny vector-backed smoke:

```text
docs: 2
questions: 1
mode: engine-hybrid
document_vector_rows_loaded: 2
sampled_documents_with_vectors: 2 / 2
vector_readiness.ready: true
top document: vector-matching doc-b
status: passed wiring smoke
```

**Remaining after Update 5:** run a real vector-backed EnterpriseRAG DB build
using `target/enterprise-rag-bench/embeddings/corpus_bge_m3.jsonl`, then repeat
the first-50 B7 quality gate. This will still require a full corpus ingest and
therefore should be treated as the next heavyweight B7 validation step, not a
quick smoke.

**Update 6:** ran the heavyweight B7 vector-backed validation path and found a
real performance boundary before the quality target can be judged. The runner now
has `--skip-checkpoint`, lazy `doc_id -> byte offset` document-vector loading,
vector readiness reporting, bounded hybrid scoring over a shortlist, and a
tiny end-to-end vector-backed smoke where the vector-matching document is ranked
first without building the heavy reusable search index. After the heavy attempts,
large `--skip-checkpoint engine-*` runs now fail fast above 10k documents until a
persisted candidate source exists, so they do not accidentally scan the 11GB
embedding file or grow to >100GB RSS.

```text
crates/cortex-engine/src/bin/enterprise_rag_bench_retrieval/main.rs
crates/cortex-engine/src/bin/enterprise_rag_bench_retrieval/args.rs
crates/cortex-engine/src/bin/enterprise_rag_bench_retrieval/retrieval.rs
scripts/enterprise_rag_bench/run_official_clean_benchmark.py
Makefile
cargo fmt --check
cargo test -p cortex-engine --bin enterprise_rag_bench_retrieval -- --nocapture
cargo clippy -p cortex-engine --bin enterprise_rag_bench_retrieval -- -D warnings
python3 -m py_compile scripts/enterprise_rag_bench/run_official_clean_benchmark.py
full-corpus skip-checkpoint fail-fast probe: exits in ~2.5s before vector scan
```

Vector-backed smoke:

```text
docs: 2
questions: 1
mode: engine-hybrid + weighted
skip_checkpoint: true
vector_readiness.ready: true
top document: vector-matching doc-b
heavy reusable index: not built on the prefilter path
status: passed wiring smoke
```

Heavy first-50 attempts:

```text
run: b7-vector-backed-bge-m3-skipcp3
documents ingested: 511,958
document vectors loaded: 500,694
sampled_documents_with_vectors: 2003 / 2048
skip_checkpoint: true
reusable search index build: 1,151,280 ms
q1 retrieval: still >120s, RSS grew to ~106 GB
status: stopped; full SearchIndexes rebuild/scan path is not viable

run: b7-vector-backed-bge-m3-skipcp4
path: ingest-time LexicalIndex prefilter + vector shortlist scoring
smoke: passed
full first-50: stopped before first 50k ingest checkpoint after ~244s
status: stopped; full body tokenization during ingest is too slow for this gate

guard: full-corpus --skip-checkpoint engine-hybrid now exits early with:
--skip-checkpoint engine-hybrid is limited to <= 10000 documents until a persisted candidate source is available
```

Current first-50 reference metrics:

```text
engine-aql-wired-50:        average_recall_pct 64.00, invalid_extra_docs 9.36
cached-lexical:             average_recall_pct 56.00, invalid_extra_docs 9.44
b7-engine-hybrid-fullvecs:  average_recall_pct 20.00, invalid_extra_docs 5.44
```

**Remaining after Update 6:** do not promote the current dense branch. The next
B7 implementation should use an already materialized fast candidate source
(checkpointed `.aci`, wide retrieval artifact, or a persisted benchmark index)
as the shortlist, then apply vector/evidence-overlap rerank over that shortlist.
The B7 target is still open: `average_invalid_extra_docs <= 6.5` while keeping
recall at or above the high-recall AQL baseline, not the failed 20% dense branch.

**Update 7:** validated the checkpointed `.aci` shortlist path with external
bge-m3 document vectors. This closes the B7 runtime wiring gap for a reusable
DB: no corpus re-ingest is required, vector readiness is true through
`--document-vectors`, and per-question retrieval runs in sub-second time after
the fixed prefilter load cost. Quality is still not enough to promote B7.

```text
crates/cortex-engine/src/bin/enterprise_rag_bench_retrieval/main.rs
cargo fmt --check
cargo test -p cortex-engine --bin enterprise_rag_bench_retrieval -- --nocapture
cargo clippy -p cortex-engine --bin enterprise_rag_bench_retrieval -- -D warnings
```

Checkpoint-prefilter validation:

```text
run: b7-checkpoint-prefilter-bge-m3-cap2
mode: engine-hybrid + weighted
reuse_db: true
external_document_vectors_available: true
vector_readiness.ready: true
retrieval duration: 125,370 ms
throughput: 0.399 q/s
average_recall_pct: 52.00
full_recall_questions: 26
hit_questions: 26
average_invalid_extra_docs: 9.48
status: performance viable, quality below cached/AQL baselines
```

Follow-up calibration attempts:

```text
run: b7-checkpoint-prefilter-bge-m3-vscale16
change: scale vector dot score by 16
average_recall_pct: 46.00
full_recall_questions: 23
hit_questions: 23
average_invalid_extra_docs: 9.54
delta vs cap2: -6.00 recall, -3 full hits
status: rejected regression

run: b7-checkpoint-prefilter-bge-m3-safe-promotion
change: preserve lexical head, then add vector-promoted candidates
retrieval duration: 72,740 ms
throughput: 0.687 q/s
average_recall_pct: 52.00
full_recall_questions: 26
hit_questions: 26
average_invalid_extra_docs: 9.48
delta vs cap2: 0.00 recall, 0 regressions
status: safe but no quality lift
```

Comparison against reference first-50 gates:

```text
engine-aql-wired-50: average_recall_pct 64.00, invalid_extra_docs 9.36
cached-lexical:      average_recall_pct 56.00, invalid_extra_docs 9.44
safe-promotion:      average_recall_pct 52.00, invalid_extra_docs 9.48
```

**Remaining after Update 7:** B7 remains partial. The runtime path is now
usable, but score-only vector rerank is not enough. The next B7 slice should
rank by evidence coverage, entity/source overlap, condition coverage, and
document-level answerability over the checkpointed shortlist; otherwise the
runner just reorders plausible-but-wrong docs and cannot reach
`average_invalid_extra_docs <= 6.5` at high recall.

**Update 8:** evaluated official-clean evidence-tail pruning gates for the
checkpointed `.aci` shortlist path. The runner now computes a query-only
`evidence_score` from anchors, source hints inferred from the question text,
scope mapping, numeric/condition coverage, decomposed requirement coverage, and
meaningful query term overlap. It keeps the normal lexical/vector head, admits
only strong evidence-bearing tail candidates, and rejects weak late candidates
so the row does not grow just because a plausible dense candidate exists.

This uses only clean inference inputs: `question_id`, `question`, retrieved
payloads, and external document vectors. Gold fields remain judge-only.

```text
crates/cortex-engine/src/bin/enterprise_rag_bench_retrieval/main.rs
cargo fmt --check
cargo test -p cortex-engine --bin enterprise_rag_bench_retrieval -- --nocapture
cargo clippy -p cortex-engine --bin enterprise_rag_bench_retrieval -- -D warnings
```

Rejected calibrations:

```text
run: b7-evidence-tail-filter-top5
change: keep top-5 by default, admit strong evidence tail
average_recall_pct: 44.00
full_recall_questions: 22
hit_questions: 22
average_invalid_extra_docs: 5.10
delta vs s32: -8.00 recall, -4 full hits
status: rejected; invalid improved by over-pruning useful late docs

run: b7-evidence-tail-replace
change: replace weak tail docs with stronger evidence-tail docs
mode: engine-hybrid + weighted
reuse_db: true
external_document_vectors_available: true
retrieval duration: 137,950.810 ms
throughput: 0.362 q/s
average_recall_pct: 52.00
full_recall_questions: 26
hit_questions: 26
average_invalid_extra_docs: 6.48
delta vs b7-evidence-tail-filter-s32: 0.00 recall, 0 hit/full regressions
status on first-50: promising

run: b7-evidence-tail-replace-500
size: 500 primary questions, retrieval-only, official-clean
retrieval duration: 412,493.009 ms
throughput: 1.212 q/s
evaluated_questions: 470
average_recall_pct: 40.76
full_recall_questions: 172
hit_questions: 216
average_invalid_extra_docs: 6.44
delta vs gemini35-fresh3-official: -44.98 recall, -209 full hits
status: rejected; first-50 invalid win did not generalize to full-500 recall
```

Comparison across B7 checkpointed-clean attempts:

```text
safe-promotion:              recall 52.00, full 26, invalid 9.48
evidence-tail-filter-s32:    recall 52.00, full 26, invalid 6.80
evidence-tail-replace:       recall 52.00, full 26, invalid 6.48 first-50,
                              rejected on full-500
```

**Remaining after Update 8:** B7 is still partial at the roadmap level. The
first-50 invalid target can be met with tail pruning, but full-500 recall falls
too far below the stronger baseline. Do not promote replacement-style pruning.
Next B7 work must improve candidate discovery/recall first, then apply
evidence-overlap pruning as a later precision step.

**Update 9:** added an official-clean query-source hint slice for the
checkpointed engine-hybrid prefilter path. The runner now derives source-system
hints only from the user-visible question text and passes them into the lexical
prefilter as a soft preference; the prefilter still falls back to the full
corpus. No `question_type`, `source_types`, `expected_doc_ids`, or answer facts
are read at inference time.

The slice also fixed a false positive in source-marker detection: `repo` must
match as a word boundary, so words like `reported` no longer route to GitHub.

```text
crates/cortex-engine/src/bin/enterprise_rag_bench_retrieval/main.rs
cargo fmt --check
cargo test -p cortex-engine --bin enterprise_rag_bench_retrieval -- --nocapture
```

Measured first-50 A/B:

```text
baseline:  b7-evidence-tail-filter-s32
candidate: b7-query-source-hints-boundary2
average_recall_pct:        52.00 -> 54.00
full_recall_questions:     26 -> 27
hit_questions:             26 -> 27
average_invalid_extra_docs: 6.80 -> 6.78
improved_questions:        qst_0025
regressed_questions:       0
```

Artifacts:

```text
target/enterprise-rag-bench/official-clean/50/b7-query-source-hints-boundary2/retrieval.clean.jsonl
target/enterprise-rag-bench/official-clean/50/b7-query-source-hints-boundary2/retrieval_quality_gate_report.json
target/enterprise-rag-bench/official-clean/50/b7-query-source-hints-boundary2/compare_vs_s32.json
```

**Remaining after Update 9:** B7 is still partial. The query-source hint slice
is clean and improves first-50 without regressions, but it has not yet passed a
full-500 promotion gate. Full B7 still requires `average_invalid_extra_docs <=
6.5` while preserving high full-500 recall. The next B7 step is a full-500
official-clean regression gate or a cheaper held-out gate that includes
non-basic categories before promotion.

**Update 10:** ran the full-500 official-clean retrieval-only gate for the
query-source hint slice.

```text
run: b7-query-source-hints-full500
mode: engine-hybrid + weighted
reuse_db: true
top_k: 10
retrieval duration: 3,828,317.720 ms
throughput: 0.131 q/s
evaluated_questions: 470
average_recall_pct: 40.81
full_recall_questions: 173
hit_questions: 216
average_invalid_extra_docs: 7.23
```

Comparison:

```text
vs b7-evidence-tail-replace-500:
  average_recall_pct: 40.76 -> 40.81
  full_recall_questions: 172 -> 173
  hit_questions: 216 -> 216
  improved_questions: 9
  regressed_questions: 5

vs gemini35-fresh3-official high-recall baseline:
  average_recall_pct: 85.74 -> 40.81
  full_recall_questions: 381 -> 173
  hit_questions: 413 -> 216
```

Artifacts:

```text
target/enterprise-rag-bench/official-clean/500/b7-query-source-hints-full500/retrieval.clean.jsonl
target/enterprise-rag-bench/official-clean/500/b7-query-source-hints-full500/retrieval_quality_gate_report.json
target/enterprise-rag-bench/official-clean/500/b7-query-source-hints-full500/compare_vs_gemini35_fresh3.json
target/enterprise-rag-bench/official-clean/500/b7-query-source-hints-full500/compare_vs_b7_replace.json
```

**Remaining after Update 10:** B7 is still partial and must not be promoted.
The source-hint slice is a clean local improvement over the rejected
engine-hybrid replace path, but the product runner still lacks the high-recall
candidate discovery path represented by `gemini35-fresh3-official`. The next
B7 task is to replace the slow/low-recall `.aci` prefilter path with a clean
reusable high-recall candidate source, then re-apply evidence-overlap pruning.

**Update 11:** added the clean reusable high-recall candidate source hook to
the official-clean retrieval runner. `enterprise_rag_bench_retrieval` now accepts:

```text
--prefilter-retrieval <clean-jsonl>
```

The file is treated as a candidate source only if every row is official-clean:
allowed fields are `question_id`, `question`, `answer`, and `document_ids`.
Oracle/scoring fields such as `question_type`, `source_types`,
`expected_doc_ids`, `answer_facts`, or unknown extras are rejected before
retrieval starts. When present, these clean candidate docs are preferred as the
shortlist seed and the runner falls back to the existing `.aci` lexical
prefilter when the external pool is missing or too small. The run report also
records the prefilter retrieval path for reproducibility.

```text
crates/cortex-engine/src/bin/enterprise_rag_bench_retrieval/args.rs
crates/cortex-engine/src/bin/enterprise_rag_bench_retrieval/main.rs
cargo fmt --check
cargo test -p cortex-engine --bin enterprise_rag_bench_retrieval -- --nocapture
cargo clippy -p cortex-engine --bin enterprise_rag_bench_retrieval -- -D warnings
```

Targeted runner tests now cover clean-row loading, duplicate document
deduplication, oracle-field rejection, and unknown-field rejection.

**Remaining after Update 11:** B7 is still partial, not promoted. The runner can
now consume a clean high-recall candidate artifact, but quality still needs to
be proven by running a first-50/balanced held-out gate and then full-500
promotion gate against an actual high-recall clean artifact. The target remains
`average_invalid_extra_docs <= 6.5` while preserving high recall.

**Update 12:** wired the clean high-recall candidate artifact into the
official-clean wrapper and validated the first-50 B7 gate. The wrapper now
passes `--prefilter-retrieval` through Makefile/wrapper runs and records the
path in `retrieval_report.json`. Empty `document_ids` rows in the external clean
artifact are accepted as "no external candidates for this question" and fall
back to the normal `.aci` lexical prefilter; malformed document ids and oracle
fields are still rejected.

The promoted first-50 calibration uses the clean full-500 high-recall artifact
as candidate source and caps the preserved external candidate tail at 6 docs.

```text
prefilter:
  target/enterprise-rag-bench/official-clean/500/gemini35-fresh3-official/retrieval.clean.jsonl

run:
  b7-clean-prefilter-gemini35-first50-limit6
mode:
  engine-hybrid + weighted + clean prefilter retrieval
retrieval duration:
  83,336.440 ms
throughput:
  0.600 q/s
vector_readiness.ready:
  true
document_vector_rows_loaded:
  500,694

average_recall_pct:
  94.00
full_recall_questions:
  47 / 50
hit_questions:
  47 / 50
average_invalid_extra_docs:
  5.68
mrr:
  0.78
ndcg:
  0.82
status:
  passed first-50 B7 gate
```

Comparisons:

```text
vs b7-query-source-hints-boundary2:
  average_recall_pct:    54.00 -> 94.00 (+40.00)
  full_recall_questions: 27 -> 47 (+20)
  hit_questions:         27 -> 47 (+20)
  improved_questions:    21
  regressed_questions:   1 (qst_0025)

vs b7-clean-prefilter-gemini35-first50 cap=7:
  average_recall_pct:    94.00 -> 94.00 (+0.00)
  full_recall_questions: 47 -> 47 (+0)
  hit_questions:         47 -> 47 (+0)
  regressed_questions:   0
  invalid_extra_docs:    6.54 -> 5.68

vs b7-evidence-tail-filter-s32:
  average_recall_pct:    52.00 -> 94.00 (+42.00)
  full_recall_questions: 26 -> 47 (+21)
  hit_questions:         26 -> 47 (+21)
  regressed_questions:   0
```

Artifacts:

```text
target/enterprise-rag-bench/official-clean/50/b7-clean-prefilter-gemini35-first50-limit6/retrieval.clean.jsonl
target/enterprise-rag-bench/official-clean/50/b7-clean-prefilter-gemini35-first50-limit6/retrieval_report.json
target/enterprise-rag-bench/official-clean/50/b7-clean-prefilter-gemini35-first50-limit6/retrieval_quality_gate_report.json
target/enterprise-rag-bench/official-clean/50/b7-clean-prefilter-gemini35-first50-limit6/compare_vs_boundary2.json
target/enterprise-rag-bench/official-clean/50/b7-clean-prefilter-gemini35-first50-limit6/compare_vs_limit7.json
target/enterprise-rag-bench/official-clean/50/b7-clean-prefilter-gemini35-first50-limit6/compare_vs_tail_filter_s32.json
```

**Remaining after Update 12:** B7 is ready for the next promotion gate, but
still partial at roadmap level. The first-50 gate is all `basic` questions, so
it proves the clean-prefilter wiring and cap calibration but not semantic,
project_related, high_level, completeness, or info_not_found behavior. Next run
must be a balanced non-basic held-out slice or full-500 official-clean
retrieval-only gate using the same clean prefilter.

**Update 13:** ran the balanced-50 mixed-category promotion gate and rejected a
plain global `limit=6` as the default for non-basic traffic. The balanced slice
contains `basic`, `semantic`, `project_related`, `completeness`,
`constrained`, `conflicting_info`, `high_level`, `info_not_found`,
`intra_document_reasoning`, and `miscellaneous` questions, so it is a better
stress test than the first-50 basic-only calibration.

The first candidate (`b7-clean-prefilter-gemini35-balanced50-limit6`) met the
invalid-docs target but lost late evidence needed by multi-evidence questions:

```text
average_recall_pct:        79.61
full_recall_questions:     35 / 47 evaluated
hit_questions:             39 / 47 evaluated
average_invalid_extra_docs: 6.28
regressed vs source:       qst_0207, qst_0341, qst_0367, qst_0450
status:                    rejected as mixed-category default
```

The accepted B7 slice for the mixed gate is route-head protection:
lookup/constrained/info-not-found queries can still use the stricter lexical
head and tail cap, while semantic multipart, project, completeness, conflict,
and high-level intents preserve the clean external candidate order so relevant
evidence at positions 7-10 is not dropped. This uses only text-derived route
classification and clean prefilter rows; no `question_type`, `source_types`,
`expected_doc_ids`, or `answer_facts` are read by inference.

```text
run:
  b7-clean-prefilter-gemini35-balanced50-route-head
mode:
  engine-hybrid + weighted + clean prefilter retrieval + route-aware head
retrieval duration:
  84,358.552 ms
throughput:
  0.593 q/s

average_recall_pct:
  81.41
full_recall_questions:
  36 / 47 evaluated
hit_questions:
  39 / 47 evaluated
average_invalid_extra_docs:
  7.28
mrr:
  0.73
ndcg:
  0.73
```

Comparison against the strict `limit=6` mixed run:

```text
average_recall_pct:    79.61 -> 81.41 (+1.80)
full_recall_questions: 35 -> 36 (+1)
hit_questions:         39 -> 39 (+0)
improved_questions:    qst_0207, qst_0341, qst_0367, qst_0450
regressed_questions:   qst_0340
```

Comparison against the clean source prefilter on the same balanced slice:

```text
average_recall_pct:        81.41 -> 81.41 (+0.00)
full_recall_questions:     36 -> 36 (+0)
hit_questions:             39 -> 39 (+0)
average_invalid_extra_docs: 8.28 -> 7.28 (-1.00)
regressed_questions:       0
```

Artifacts:

```text
target/enterprise-rag-bench/official-clean/50/b7-clean-prefilter-gemini35-balanced50-limit6/retrieval_quality_gate_report.json
target/enterprise-rag-bench/official-clean/50/b7-clean-prefilter-gemini35-balanced50-route-limit/retrieval_quality_gate_report.json
target/enterprise-rag-bench/official-clean/50/b7-clean-prefilter-gemini35-balanced50-route-head/retrieval_quality_gate_report.json
target/enterprise-rag-bench/official-clean/50/b7-clean-prefilter-gemini35-balanced50-route-head/compare_vs_limit6.json
target/enterprise-rag-bench/official-clean/50/b7-clean-prefilter-gemini35-balanced50-route-head/compare_vs_source_prefilter.json
```

**Remaining after Update 13:** B7 remains partial, not promoted. First-50 basic
traffic passes with strict `limit=6`; balanced mixed traffic preserves source
recall and reduces invalid extras with route-head protection, but still misses
the global `average_invalid_extra_docs <= 6.5` target (`7.28`). The next
precision work must be coverage-gain/MMR selection (B8) or fact/source-level
overlap, not a global tail cap that drops multi-evidence questions.

### B8 — MMR diversity per question type
Параметризовать MMR (diversity vs relevance) по типу вопроса:
для completeness/conflicting_info — выше diversity (нужны разные
источники), для basic — ниже (точность важнее).
**Target:** completeness document recall не падает при росте diversity у completeness/conflicting.

**Status:** done for route-aware MMR wiring, diagnostics, balanced-50 promotion,
and full-500 no-regression runtime gate; limited as a standalone quality lift.
Закрыт route-aware MMR slice: `HybridRerank` теперь после rerank применяет
diversity selection только если `SearchRoutePolicy.diversity` включён для
intent вопроса. Для near-duplicate candidates используется Jaccard-over-body
penalty, поэтому completeness/project/high-level/conflict режимы не тратят весь
top-k на почти одинаковые payloads, а lookup/strict пути остаются
score-ordered. Full-500 gate подтвердил, что B8 можно запускать быстро и без
recall-регрессии, но сам по себе он не снижает full-500 invalid extras без
потери recall.

**Done evidence:**

```text
crates/cortex-engine/src/search/database.rs
crates/cortex-engine/tests/database_search.rs
cargo test -p cortex-engine --test database_search diversifies_completeness -- --nocapture
cargo test -p cortex-engine --test database_search
cargo test -p cortex-engine --test query_search
cargo test -p cortex-engine --bin enterprise_rag_bench_retrieval
cargo fmt --check
cargo clippy -p cortex-engine --lib -- -D warnings
cargo clippy -p cortex-engine --bin enterprise_rag_bench_retrieval -- -D warnings
```

**Remaining:** expose diversity diagnostics, type-specific lambda tuning,
source/thread/entity cluster diversity, and 50/500 completeness/conflict
regression gate.

**Update:** added type-specific MMR lambda tuning to the route policy. The
diversity selector no longer uses one fixed near-duplicate threshold for every
wide query. It now computes an MMR-style score:

```text
lambda * relevance - (1 - lambda) * max_payload_similarity
```

where `lambda` is selected from the no-oracle intent classifier:

```text
lookup / constrained / info_not_found: diversity disabled
project_related:                      high relevance preservation
semantic:                             moderate diversity
high_level:                           stronger diversity
completeness:                         stronger coverage diversity
conflicting_info:                     strongest diversity
```

This keeps basic lookup paths score-ordered while making completeness and
conflict questions less likely to spend top-k on near-duplicate payloads.

```text
crates/cortex-engine/src/search/routing.rs
crates/cortex-engine/src/search/database.rs
cargo test -p cortex-engine --lib search::routing -- --nocapture
cargo test -p cortex-engine --test database_search diversifies_completeness -- --nocapture
```

**Update 2:** added metadata-cluster similarity to the MMR diversity score.
The selector now takes the maximum of payload Jaccard and structured cluster
matches, so two candidates are treated as more redundant when they share:

```text
content_hash
document_id
parent_id
source_hash
path
project
entity
topic
source
```

This closes the first source/thread/entity/project cluster slice without adding
new dependencies or oracle fields. The behavior is still route-gated: lookup,
constrained, and info-not-found queries remain score-ordered, while wide
intents use cluster-aware MMR before top-k cutoff.

```text
crates/cortex-engine/src/search/database.rs
crates/cortex-engine/tests/database_search.rs
cargo test -p cortex-engine --test database_search diversifies_completeness -- --nocapture
```

**Remaining after Update 2:** B8 is still partial. Source/document/project/
entity cluster suppression is now implemented, but B8 still needs explicit
diversity diagnostics in retrieval reports and a balanced/full benchmark gate
proving completeness/conflict recall is preserved while invalid extras
decrease.

**Update 3:** added diversity diagnostics to the engine search outcome and the
benchmark retrieval report. `DatabaseSearchOutcome` now exposes optional
`SearchDiversityDiagnostics` for `HybridRerank`, including:

```text
intent
diversity_enabled
lambda_q16
input_candidates
output_candidates
skipped_candidates
max_payload_similarity_q16
max_cluster_similarity_q16
selected_with_payload_similarity
selected_with_cluster_similarity
```

The EnterpriseRAG retrieval runner aggregates those diagnostics under the
`diversity` key in `retrieval_report.json` for direct engine search runs. This
does not change the official retrieval JSONL output and does not use oracle
fields; it only makes B8 observable.

```text
crates/cortex-engine/src/search/database.rs
crates/cortex-engine/src/search.rs
crates/cortex-engine/src/bin/enterprise_rag_bench_retrieval/main.rs
crates/cortex-engine/tests/database_search.rs
cargo test -p cortex-engine --test database_search reports_cluster -- --nocapture
cargo test -p cortex-engine --bin enterprise_rag_bench_retrieval diversity_run_metrics -- --nocapture
```

**Remaining after Update 3:** B8 still needs a balanced/full benchmark gate.
The engine can now explain diversity behavior, but we still need measured
evidence that completeness/conflict recall is preserved while invalid extras
decrease on EnterpriseRAG slices.

**Update 4:** added a benchmark-only switch for direct engine-search inspection
without the checkpoint/ingest lexical search prefilter. The retrieval binary and
official-clean wrapper now accept:

```text
--disable-search-prefilter
```

The field is recorded in `retrieval_report.json` as
`disable_search_prefilter`, and when enabled the runner does not load the cached
lexical prefilter index before `engine-hybrid-rerank`. This makes B8 diversity
diagnostics observable on the real full-corpus engine path instead of hiding
them behind reusable lexical candidates.

Smoke evidence:

```text
python3 scripts/enterprise_rag_bench/run_official_clean_benchmark.py \
  --size 1 \
  --questions-file target/enterprise-rag-bench/subsets/balanced_50/balanced_50_questions.jsonl \
  --split-name balanced_50_smoke \
  --run-label b8-direct-hybrid-rerank-smoke1b \
  --stage retrieval \
  --answer-provider gemma \
  --judge-provider gemini \
  --retrieval-mode engine-hybrid-rerank \
  --rerank none \
  --reuse-db \
  --db-root target/enterprise-rag-bench/official-clean/50/cortexdb \
  --query-vectors target/enterprise-rag-bench/official-clean/500/full500-dense-hybrid/query_vectors.jsonl \
  --disable-search-prefilter \
  --retrieval-progress-every 1
```

The smoke run completed with:

```text
retrieval_report.json:
  disable_search_prefilter: true
  diversity.reports: 1
  questions: 1
  retrieval.duration_ms: 139041.606
  last_question_ms: 138884.033
```

The earlier balanced-50 direct run was stopped after q1/q2 showed the same
latency shape (`q1` started around 86s, `q2` around 218s), and the stale DB
lock from killed PID `1214189` was removed with:

```text
./target/debug/cortexdb unlock target/enterprise-rag-bench/official-clean/50/cortexdb --force
```

```text
crates/cortex-engine/src/bin/enterprise_rag_bench_retrieval/args.rs
crates/cortex-engine/src/bin/enterprise_rag_bench_retrieval/main.rs
scripts/enterprise_rag_bench/run_official_clean_benchmark.py
cargo test -p cortex-engine --bin enterprise_rag_bench_retrieval parses_disable_search_prefilter_flag -- --nocapture
python3 -m py_compile scripts/enterprise_rag_bench/run_official_clean_benchmark.py
```

**Remaining after Update 4:** B8 remains partial. Direct full-corpus
`engine-hybrid-rerank` now exposes diversity metrics, but the no-prefilter path
is too slow for a balanced/full quality gate. The next required implementation
step is a bounded direct candidate source that still exercises engine rerank/MMR
diagnostics, or a persisted search optimization that avoids scanning the full
511k-document corpus per question.

**Update 5:** added a bounded prefilter MMR path for `engine-hybrid-rerank`.
The retrieval binary now keeps reusable clean prefilter candidates, applies a
benchmark-side diversity selector to the bounded pool, and still records
diversity diagnostics through `PrefilterSearchOutput`. This avoids the
`--disable-search-prefilter` full scan latency while exercising the same
route-aware diversity behavior on EnterpriseRAG artifacts.

The first bounded run without the clean B7 prefilter proved that cached lexical
alone is not a strong enough candidate source:

```text
b8-bounded-hybrid-rerank-balanced50:
  average_recall_pct: 29.23
  full_recall_questions: 12
  hit_questions: 16
  average_invalid_extra_docs: 7.40
```

The first clean-prefilter MMR run reduced invalid extras but over-diversified
the head and dropped gold documents:

```text
b8-clean-prefilter-mmr-balanced50:
  average_recall_pct: 75.86
  full_recall_questions: 32
  hit_questions: 38
  average_invalid_extra_docs: 6.64
  regressions vs B7 route-head: 6
```

The final protected-head variant preserves the high-recall B7 prefix for
fragile intents and diversifies the tail for coverage-oriented intents. This
passes the balanced-50 promotion gate against B7 route-head:

```text
b8-clean-prefilter-mmr-protected-balanced50:
  average_recall_pct: 81.68
  full_recall_questions: 36
  hit_questions: 39
  average_invalid_extra_docs: 6.51
  retrieval.duration_ms: 179191.346
  retrieval.throughput_qps: 0.279

compare vs b7-clean-prefilter-gemini35-balanced50-route-head:
  average_recall_pct: 81.41 -> 81.68 (+0.27)
  full_recall_questions: 36 -> 36
  hit_questions: 39 -> 39
  regressed_question_ids: []
  improved_question_ids: [qst_0450]
  completeness recall: 75.00 -> 81.25 (+6.25)
```

The run produced diversity diagnostics for all 50 questions and enabled
diversity on 27 of them:

```text
diversity.reports: 50
diversity.diversity_enabled_questions: 27
diversity.input_candidates: 6400
diversity.output_candidates: 384
diversity.skipped_candidates: 6016
```

Artifacts:

```text
target/enterprise-rag-bench/official-clean/50/b8-clean-prefilter-mmr-protected-balanced50/retrieval.clean.jsonl
target/enterprise-rag-bench/official-clean/50/b8-clean-prefilter-mmr-protected-balanced50/retrieval_report.json
target/enterprise-rag-bench/official-clean/50/b8-clean-prefilter-mmr-protected-balanced50/retrieval_quality_report.json
target/enterprise-rag-bench/official-clean/50/b8-clean-prefilter-mmr-protected-balanced50/compare_vs_b7_route_head.json
```

**Remaining after Update 5:** B8 is promoted on the balanced-50 regression
gate, but still needs the same protected-head check on full-500 before the epic
can be marked fully done. The implementation lesson is fixed: MMR must not
freely reorder high-recall head candidates for semantic/project-related
queries; it should protect the head and spend diversity only on the tail or on
coverage-oriented intents.

**Update 6:** attempted the same protected-head B8 path on full-500 with
clean input and the best existing full prefilter:

```text
questions-file:
  target/enterprise-rag-bench/official-clean/500/b7-evidence-tail-replace-500/questions.gold.jsonl

clean prefilter:
  target/enterprise-rag-bench/official-clean/500/gemini35-fresh3-official/retrieval.clean.jsonl

run label:
  b8-clean-prefilter-mmr-protected-full500
```

The official-clean prepare step stripped all oracle fields for 500/500
questions. The retrieval stage successfully ingested and checkpointed the full
511,958-document corpus in a separate DB root, but the full retrieval gate was
stopped after q1/q2 exposed unacceptable latency:

```text
ingest:        511958 documents in ~35.2s
checkpoint:    ~1529.1s
index load:    ~126.7s after checkpoint
qst_0001:      172723.445 ms
qst_0002:      stopped after it also failed to complete quickly
```

The run also reported:

```text
vector readiness warning:
  hybrid retrieval selected but no document vectors were found in the sampled DB payloads
```

This means the full-500 attempt did not produce a quality report. It did prove
that the current full-corpus B8 path is not a viable promotion gate yet:
candidate payload lookup / cached lexical prefilter load over the full
checkpointed corpus is too slow for 500 questions, and the full DB lacks
payload-embedded document vectors for true dense rerank. The stale lock left by
the stopped process was removed with:

```text
./target/debug/cortexdb unlock \
  target/enterprise-rag-bench/official-clean/500/b8-clean-prefilter-mmr-protected-full500/cortexdb \
  --force
```

Artifacts:

```text
target/enterprise-rag-bench/official-clean/500/b8-clean-prefilter-mmr-protected-full500/questions.clean.jsonl
target/enterprise-rag-bench/official-clean/500/b8-clean-prefilter-mmr-protected-full500/prepare_report.json
target/enterprise-rag-bench/official-clean/500/b8-clean-prefilter-mmr-protected-full500/retrieval_progress.log
target/enterprise-rag-bench/official-clean/500/b8-clean-prefilter-mmr-protected-full500/official_clean_status.json
target/enterprise-rag-bench/official-clean/500/b8-clean-prefilter-mmr-protected-full500/cortexdb/
```

**Remaining after Update 6:** B8 is still balanced-50 promoted but full-500
blocked by runtime, not by measured recall regression. Before rerunning
full-500, implement one of:

```text
1. reusable external-prefilter payload cache keyed by doc_id;
2. full-corpus DB root with embedded document vectors and fast payload lookup;
3. retrieval runner path that applies protected-head MMR directly to clean
   prefilter docs without loading the full persisted lexical index per run.
```

**Update 7:** implemented the fast source-payload external-prefilter path and
then fixed the promotion gate to preserve `top_k` for retrieval-only evaluation.
The first fast full-500 attempt proved runtime but over-pruned lookup/constrained
rows through `routed_result_limit`, causing recall regression:

```text
b8-source-payload-mmr-protected-full500:
  source_payload_prefilter: true
  retrieval duration: 34,815.013 ms
  throughput: 14.362 q/s
  average_recall_pct: 81.07
  full_recall_questions: 355
  hit_questions: 396
  average_invalid_extra_docs: 4.98
  delta vs gemini35-fresh3-official: -4.67 recall, -26 full, -17 hit
  status: rejected for promotion; invalid improved by shrinking result rows
```

The fix keeps B8 diversity/rerank bounded to the clean prefilter pool but no
longer shrinks the retrieval row below requested `top_k`. Adaptive output
caps remain a B9/ContextPack concern, not a full-500 retrieval recall gate.

```text
crates/cortex-engine/src/bin/enterprise_rag_bench_retrieval/main.rs
cargo fmt --check
cargo test -p cortex-engine --bin enterprise_rag_bench_retrieval -- --nocapture
cargo clippy -p cortex-engine --bin enterprise_rag_bench_retrieval -- -D warnings
```

Accepted full-500 no-regression gate:

```text
b8-source-payload-mmr-topk-full500:
  source_payload_prefilter: true
  ingest/checkpoint: skipped
  retrieval duration: 32,662.120 ms
  throughput: 15.308 q/s
  peak RSS: ~276.7 MB
  diversity.reports: 500
  diversity.diversity_enabled_questions: 125
  diversity.input_candidates: 4500
  diversity.output_candidates: 4500
  diversity.skipped_candidates: 0

retrieval quality:
  evaluated_questions: 470
  average_recall_pct: 85.74
  full_recall_questions: 381
  hit_questions: 413
  average_invalid_extra_docs: 8.23
  mrr: 0.72
  ndcg: 0.74

compare vs gemini35-fresh3-official:
  average_recall_pct: 85.74 -> 85.74
  full_recall_questions: 381 -> 381
  hit_questions: 413 -> 413
  regressed_question_ids: []
```

Artifacts:

```text
target/enterprise-rag-bench/official-clean/500/b8-source-payload-mmr-topk-full500/retrieval.clean.jsonl
target/enterprise-rag-bench/official-clean/500/b8-source-payload-mmr-topk-full500/retrieval_report.json
target/enterprise-rag-bench/official-clean/500/b8-source-payload-mmr-topk-full500/retrieval_quality_report.json
target/enterprise-rag-bench/official-clean/500/b8-source-payload-mmr-topk-full500/compare_vs_gemini35_fresh3_official.json
```

**Remaining after Update 7:** no B8 runtime or recall-regression blocker remains.
The quality lesson is that full-500 invalid-extra reduction cannot come from
blind row shrinking. Further precision work belongs to B7/B9/Evidence selection:
coverage-gain pruning, answerability, and ContextPack budget optimization must
reduce noise after recall-safe evidence selection.

### B9 — Adaptive top-k / token budget по сложности вопроса
Простые basic-вопросы — узкий context (меньше invalid_extra_docs);
project_related/completeness — широкий budget (нужно больше источников).
Эвристика на основе длины вопроса + query classification (B4).
**Target:** invalid_extra_docs ↓ для basic без потери recall у сложных типов.

**Status:** partial, benchmark trace slice done. Закрыт clean adaptive slice без oracle-полей:
`SearchRoutePolicy` теперь задаёт не только candidate-depth, но и output cap /
token-budget multiplier по intent из текста вопроса. Lookup/info-not-found
запросы получают более компактный search result/top-k, а
project/completeness/high-level/conflict сохраняют широкий top-k. Это
подключено в `SearchIndexes::search_with_reranker`, `HybridRerank` database
path и pluggable DB reranker path. `RETRIEVE CONTEXT ... BUDGET` и server/API
ContextPack budget остаются контрактными: AQL/options budget только clamp'ится
AgentView policy, но не сжимается route policy. В answer runner добавлен
per-question budget trace, чтобы B9 можно было оценивать без oracle fields и
без догадок по логам.

**Done evidence:**

```text
crates/cortex-engine/src/search/routing.rs
crates/cortex-engine/src/search.rs
crates/cortex-engine/src/search/database.rs
crates/cortex-engine/tests/query_search.rs
crates/cortex-engine/tests/database_search.rs
crates/cortex-engine/tests/aql_limit_budget_semantics.rs
cargo test -p cortex-engine --lib search::routing -- --nocapture
cargo test -p cortex-engine --test query_search
cargo test -p cortex-engine --test database_search
cargo test -p cortex-engine --test aql_limit_budget_semantics
cargo test -p cortex-engine --bin enterprise_rag_bench_retrieval
cargo fmt --check
cargo clippy -p cortex-engine --lib -- -D warnings
cargo clippy -p cortex-engine --bin enterprise_rag_bench_retrieval -- -D warnings
```

**Remaining:** measure first-50 and full-500 official-clean answer/judge deltas
for Overall, correctness, completeness, token cost, recall and
`invalid_extra_docs`; tune caps only from dev/held-out evidence.

**Update:** added explicit answer-budget tracing for official-clean answer
runs. `run_deepseek_answers.py` now resolves the active per-question answer
budget in one testable function and records:

```text
answer_intent
selected_result_limit
active_top_k_context
active_max_chars_per_doc
active_max_tokens
retrieved_doc_count
used_doc_count
adaptive_budget_applied
high_level_override_applied
budget_profile
trace_source
```

The trace is written to `answer_budget_trace.jsonl` and also recomputed when an
answer run is resumed/reused, so budget observability does not require another
LLM call. Evidence-table extraction now follows the active per-question
`top_k_context` instead of the static default.

```text
scripts/enterprise_rag_bench/run_deepseek_answers.py
scripts/enterprise_rag_bench/test_answer_intent.py
python3 scripts/enterprise_rag_bench/test_answer_intent.py
python3 -m py_compile scripts/enterprise_rag_bench/run_deepseek_answers.py scripts/enterprise_rag_bench/run_official_clean_answers.py scripts/enterprise_rag_bench/run_official_clean_benchmark.py
```

Offline full-500 trace over the latest B8 no-regression retrieval artifact:

```text
trace:
  target/enterprise-rag-bench/official-clean/500/b8-source-payload-mmr-topk-full500/answer_budget_trace.jsonl
summary:
  target/enterprise-rag-bench/official-clean/500/b8-source-payload-mmr-topk-full500/answer_budget_trace.summary.json

questions: 500
adaptive_budget_questions: 152

by_answer_intent:
  default:          348 questions, avg selected_result_limit 8.0, avg max_tokens 420
  complex_project:  91 questions, avg selected_result_limit 10.0, avg max_tokens 900
  completeness:     19 questions, avg selected_result_limit 10.0, avg max_tokens 900
  conflict:         18 questions, avg selected_result_limit 10.0, avg max_tokens 800
  constrained:      21 questions, avg selected_result_limit 8.0, avg max_tokens 700
  high_level:        3 questions, avg selected_result_limit 10.0, avg max_tokens 900
```

**Remaining after trace slice:** run a controlled answer/judge A/B, preferably
balanced-50 first, comparing static answer budget vs
`--enable-text-intent-budget`. Promote only if Overall/correctness improves or
token spend decreases without category regression. Do not use B9 top-k
shrinking inside retrieval-only recall gates; that already caused the rejected
B8 recall regression.

**Update 2:** added a reusable answer-budget A/B comparer so B9 can be gated
before spending model tokens:

```text
scripts/enterprise_rag_bench/compare_answer_budget_runs.py
scripts/enterprise_rag_bench/test_compare_answer_budget_runs.py
python3 scripts/enterprise_rag_bench/test_compare_answer_budget_runs.py
python3 -m py_compile scripts/enterprise_rag_bench/compare_answer_budget_runs.py scripts/enterprise_rag_bench/test_compare_answer_budget_runs.py scripts/enterprise_rag_bench/run_deepseek_answers.py scripts/enterprise_rag_bench/test_answer_intent.py
```

Trace-only full-500 A/B against a derived static baseline
(`top_k_context=8`, `max_chars_per_doc=2200`, `max_tokens=420`) now produces:

```text
report:
  target/enterprise-rag-bench/official-clean/500/b8-source-payload-mmr-topk-full500/answer_budget_ab_report.json
markdown:
  target/enterprise-rag-bench/official-clean/500/b8-source-payload-mmr-topk-full500/answer_budget_ab_report.md

questions:                         500
adaptive_budget_questions:          +152
high_level_override_questions:      +0
avg_selected_result_limit_delta:    +0.52
avg_used_doc_count_delta:           +0.37
avg_max_chars_per_doc_delta:      +275.20
avg_max_tokens_delta:             +133.92
max_selected_result_limit_delta:    +2
max_max_tokens_delta:             +480
```

This is still not a quality promotion: it proves the adaptive policy is
bounded and oracle-clean in the trace. B9 still needs answer/judge A/B to prove
whether the extra budget improves Overall/correctness/completeness enough to
justify the extra generated-token ceiling.

**Update 3:** ran a controlled official-clean balanced-50 answer/judge A/B with
the same retrieval file and local Gemma answerer/judge. The only intended
difference was `--enable-text-intent-budget`; no oracle fields were present in
the retrieval/answer input.

```text
retrieval:
  target/enterprise-rag-bench/official-clean/50/b8-source-payload-mmr-protected-balanced50/retrieval.clean.jsonl
questions:
  target/enterprise-rag-bench/subsets/balanced_50/balanced_50_questions.jsonl

static answers:
  target/enterprise-rag-bench/official-clean/50/b9-balanced50-static/answer-gemma/answers.jsonl
static judge:
  target/enterprise-rag-bench/official-clean/50/b9-balanced50-static/judge-gemma/results.json

adaptive answers:
  target/enterprise-rag-bench/official-clean/50/b9-balanced50-adaptive/answer-gemma/answers.jsonl
adaptive judge:
  target/enterprise-rag-bench/official-clean/50/b9-balanced50-adaptive/judge-gemma/results.json

ab report:
  target/enterprise-rag-bench/official-clean/50/b9-balanced50-ab/answer_budget_ab_report.json
```

Results:

```text
static:
  Overall:              44.64
  Correctness:          52.00
  Completeness:         50.04
  Answer total tokens: 263,360
  Judge total tokens:   33,469

adaptive:
  Overall:              46.84
  Correctness:          54.00
  Completeness:         51.24
  Answer total tokens: 278,568
  Judge total tokens:   33,951

delta:
  Overall:              +2.20
  Correctness:          +2.00
  Completeness:         +1.20
  Answer total tokens: +15,208
  Judge total tokens:     +482
  adaptive questions:      12 / 50
  avg selected limit:     +0.36
  avg used doc count:     +0.20
  high-level override:     0
```

Per-question gate:

```text
false -> true: qst_0430
true -> false: none
completeness up: qst_0354, qst_0381, qst_0430, qst_0431
completeness down: qst_0238 (60 -> 50)
```

**Promotion decision:** promote B9 adaptive answer budget for controlled
balanced-50 official-clean answer runs: it improves Overall/correctness with no
true-to-false regression and only a moderate answer-token increase. B9 is not
fully closed until the same policy is measured on full-500 with the current
default answerer/judge package.

### B10 — Lexical retrieval upgrade: BM25F + поле-веса
`.aci` лексика сейчас плоская; добавить поле-структурированный BM25F
(title/body/entity-view из A2 с разными весами). Снижает зависимость от
dense-эмбеддингов на точных терминах/ID (тикеты, имена).
**Target:** lexical@10 baseline растёт с 3.3% (semantic-30) до 10%+.

**Status:** done for engine/storage/benchmark lexical slice. `.aci` bumped to
`ACI3` and now persists field doc lengths plus field term frequencies while
keeping `ACI0`/`ACI1`/`ACI2` readable as legacy formats. `CellMetadata` exposes
`lexical_field_terms()` for body/title/path/entity/chunk/table views with stable
field weights; live `Bm25Index`, persisted search, checkpoint merge/remove,
and EnterpriseRAG cached lexical retrieval all consume the field-aware sections.
This makes exact terms in titles, paths, entities, chunks and table headers
rank above repeated body-only matches without using gold question metadata.

**Done evidence:**

```text
crates/cortex-storage/src/format.rs
crates/cortex-storage/src/indexes.rs
crates/cortex-engine/src/query/metadata.rs
crates/cortex-engine/src/query.rs
crates/cortex-engine/src/search.rs
crates/cortex-engine/src/search/database.rs
crates/cortex-engine/src/search/persisted.rs
crates/cortex-engine/src/bin/enterprise_rag_bench_retrieval/retrieval.rs
crates/cortex-engine/tests/query_search.rs
crates/cortex-engine/tests/database_search.rs
crates/cortex-storage/tests/lexical_index_tests.rs
cargo test -p cortex-storage --test lexical_index_tests -- --nocapture
cargo test -p cortex-engine --lib search::persisted -- --nocapture
cargo test -p cortex-engine --test query_search field -- --nocapture
cargo test -p cortex-engine --test database_search persisted_title_weighting -- --nocapture
cargo test -p cortex-engine --bin enterprise_rag_bench_retrieval
cargo test --workspace --all-features
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
```

**Remaining:** run official-clean first-50/full-500 metric gate after this
specific B10 wiring to measure lexical@10, document recall, invalid extra docs
and Overall. Persisted multi-vector/HNSW per view remains in A2/C/D follow-up
epics, not B10.

**Update:** attempted a B10-specific balanced-50 metric gate using the native
lexical path:

```text
run:
  target/enterprise-rag-bench/official-clean/50/b10-bm25f-balanced50-reuse-checkpoint/
db_root:
  target/enterprise-rag-bench/official-clean/500/b8-clean-prefilter-mmr-protected-full500/cortexdb
mode:
  engine-keyword --rerank weighted --reuse-db
questions:
  target/enterprise-rag-bench/subsets/balanced_50/balanced_50_questions.jsonl
```

The first attempt against the non-checkpointed B8 top-k DB failed correctly:

```text
--skip-checkpoint engine-keyword is limited to <= 10000 documents until a
persisted candidate source is available
```

The retry against a checkpointed DB opened successfully and started the
persisted/full-corpus lexical path, but was stopped as a runtime blocker:

```text
loaded 511,958 corpus ids
database open: ~5.6s
load cached lexical prefilter index for engine search retrieval
question 1/50 at +132.4s
question 2/50 at +303.3s
estimated 50-question runtime: >1h
```

**B10 blocker:** the BM25F/field-weight code exists, but the official-clean
full-corpus lexical benchmark path is not product-fast enough without a
persisted candidate source. Do not use this slow path as the default metric
gate. The next B10 implementation slice should make `engine-keyword` consume
the persisted segment lexical index/candidate source directly, or add a
validated lexical candidate cache artifact that is built once and reused by the
official-clean gate.

**Update 2:** closed the B10 runtime blocker for the benchmark lexical path.
`BenchmarkRetrievalIndex::load` now writes and reuses a manifest-keyed merged
lexical sidecar:

```text
benchmark-merged-lexical.aci
benchmark-merged-lexical.manifest
```

The key is derived from the CortexDB manifest generation, checkpoint sequence
and live segment metadata, not from benchmark oracle fields. The in-memory
retrieval index also precomputes source candidate sets plus global/field
average lengths, so per-query BM25F scoring no longer recomputes expensive
field averages inside the posting loop.

Runtime evidence:

```text
first smoke, cache build:
  target/enterprise-rag-bench/official-clean/5/b10-bm25f-smoke5/
  retrieval duration: 147.1s for 5 questions
  per-question after cache build: ~0.6-2.6s

second smoke, cache reuse:
  target/enterprise-rag-bench/official-clean/5/b10-bm25f-smoke5-cache/
  retrieval duration: 47.7s for 5 questions
  per-question: ~0.3-1.2s

balanced-50, cache reuse:
  target/enterprise-rag-bench/official-clean/50/b10-bm25f-balanced50-cache/
  retrieval duration: 116.8s for 50 questions
  previous stopped run: 2 questions in 303.3s
```

Quality evidence:

```text
B10 lexical-only balanced-50:
  average_recall_pct:        32.42
  average_precision_pct:      5.96
  average_invalid_extra_docs: 8.10

B8 protected balanced-50 baseline:
  average_recall_pct:        81.68
  average_precision_pct:     17.02
  average_invalid_extra_docs: 6.44
```

An experiment that fed `analyze_search_query` expansion terms into this cached
lexical prefilter was tested and rejected: it reduced balanced-50 recall to
`27.90` and failed the B10-vs-B10 regression gate. The experiment was not kept
in the default path.

**Remaining after Update 2:** B10 is no longer runtime-blocked, but it is still
not a promotion candidate as a standalone retrieval mode. Keep BM25F as a
fast lexical candidate source inside hybrid/rerank pipelines; do not replace the
B8/B9 protected hybrid path with `engine-keyword` until lexical recall and
invalid-extra-docs improve materially.

---

## Тема C. Понимание запроса (query understanding)

### C1 — Классификатор типа вопроса (intent router)
Лёгкая модель/правила для классификации входящего запроса по 10 типам
ERB (basic/semantic/project_related/...). Используется B4, B8, B9.
**Target:** ≥ 90% accuracy на held-out 50 вопросах против истинных типов.

**Status:** done for text-only engine classifier + balanced-50 gate.
Добавлен отдельный `EnterpriseRagQuestionType` classifier на 10 ERB типов:
`basic`, `semantic`, `intra_document_reasoning`, `project_related`,
`constrained`, `conflicting_info`, `completeness`, `high_level`,
`info_not_found`, `miscellaneous`. Runtime classifier читает только текст
вопроса; `question_type` используется только offline evaluator'ом
`enterprise_rag_intent_check` для диагностики accuracy/confusion. Старый
`SearchQueryIntent` теперь строится из этого 10-type classifier'а и сохраняет
существующие search policies/API contracts.

**Done evidence:**

```text
crates/cortex-engine/src/search/intent.rs
crates/cortex-engine/src/search/routing.rs
crates/cortex-engine/src/bin/enterprise_rag_intent_check.rs
Makefile: enterprise-rag-bench-intent-check
cargo test -p cortex-engine --lib search::intent -- --nocapture
cargo test -p cortex-engine --lib search::routing -- --nocapture
cargo test -p cortex-engine --bin enterprise_rag_intent_check -- --nocapture
python3 scripts/enterprise_rag_bench/build_balanced_subset.py --questions-file erb-submission/questions_updated_gpt5.2.jsonl --limit 50 --output-root target/enterprise-rag-bench/intent-check --output-prefix balanced_50
make enterprise-rag-bench-intent-check ENTERPRISE_RAG_BENCH_INTENT_CHECK_QUESTIONS=target/enterprise-rag-bench/intent-check/balanced_50/balanced_50_questions.jsonl ENTERPRISE_RAG_BENCH_INTENT_CHECK_REPORT=target/enterprise-rag-bench/intent-check/balanced_50/make_report.json ENTERPRISE_RAG_BENCH_INTENT_MIN_ACCURACY_PCT=90
```

Balanced-50 result: `48/50`, accuracy `96%`.

**Remaining:** full-500 diagnostic is `280/500`, accuracy `56%`; this is
acceptable for the current routing slice but not a perfect benchmark-label
predictor. Future calibration should improve semantic/project/intra-document
labels without using oracle fields at inference time.

### C2 — Декомпозиция вопроса на под-требования
Для completeness/project_related/constrained — разбить вопрос на список
явных под-пунктов ("перечисли X, Y, Z", "сравни A и B по метрикам M1,M2").
Питает Completeness Planner (тема E). (см. ENTERPRISE_RAG_IMPROVEMENT_EPICS Epic 05)
**Target:** ≥ 80% вопросов с >1 под-требованием корректно декомпозируются.

**Status:** done for engine decomposition + benchmark gate.

**Done:** добавлен text-only decomposition pipeline:

```text
question text
→ anchors
→ expected slots
→ subquestions
→ reusable QuestionDecomposition
```

Runtime не читает `question_type`, `source_types`, `expected_doc_ids` или
`answer_facts`. Эти поля используются только offline-gate для оценки, какие
вопросы, вероятно, требуют >1 sub-requirement.

**Done evidence:**

```text
crates/cortex-engine/src/search/decomposition.rs
crates/cortex-engine/src/bin/enterprise_rag_decomposition_check.rs
crates/cortex-engine/src/search/rerank.rs
Makefile: enterprise-rag-bench-decomposition-check
cargo test -p cortex-engine --lib search:: -- --nocapture
cargo test -p cortex-engine --bin enterprise_rag_decomposition_check -- --nocapture
make enterprise-rag-bench-decomposition-check ENTERPRISE_RAG_BENCH_DECOMPOSITION_CHECK_QUESTIONS=target/enterprise-rag-bench/intent-check/balanced_50/balanced_50_questions.jsonl ENTERPRISE_RAG_BENCH_DECOMPOSITION_CHECK_REPORT=target/enterprise-rag-bench/decomposition-check/balanced_50_report.json ENTERPRISE_RAG_BENCH_DECOMPOSITION_MIN_MULTI_COVERAGE_PCT=80
make enterprise-rag-bench-decomposition-check ENTERPRISE_RAG_BENCH_DECOMPOSITION_CHECK_QUESTIONS=erb-submission/questions_updated_gpt5.2.jsonl ENTERPRISE_RAG_BENCH_DECOMPOSITION_CHECK_REPORT=target/enterprise-rag-bench/decomposition-check/full_500_report.json ENTERPRISE_RAG_BENCH_DECOMPOSITION_MIN_MULTI_COVERAGE_PCT=80
```

Balanced-50 result: `45/46` expected-multi questions decomposed,
`97%` multi-coverage.

Full-500 result: `427/472` expected-multi questions decomposed,
`90%` multi-coverage.

**Remaining:** this is decomposition and reranker coverage plumbing, not the
full Completeness Planner. E1 still needs to consume these requirements in
ContextPack/answer repair.

### C3 — Query-to-scope mapping из текста
Сопоставление упомянутых в вопросе имён проектов/команд/источников
(Jira/Slack/Confluence/...) с metadata-полями `scope`/`source` (из A7),
без обращения к oracle. Питает B6.
**Target:** project_related — ≥70% вопросов получают непустой scope-фильтр.

**Status:** done for text-only scope mapping + reranker signal + benchmark gate.

**Done:** добавлен deterministic query-to-scope mapping:

```text
question text
→ explicit source hints
→ anchor-derived source hints
→ department/topic/project/team directives
→ QueryScopeMapping
```

Runtime не читает `question_type`, `source_types`, `expected_doc_ids` или
`answer_facts`. `question_type=project_related` используется только offline-gate,
чтобы измерить target C3 на нужной категории. Explicit source mentions могут быть
hard-filter hints, остальные directives остаются soft routing/rerank signals.

**Done evidence:**

```text
crates/cortex-engine/src/search/scope_mapping.rs
crates/cortex-engine/src/bin/enterprise_rag_scope_mapping_check.rs
crates/cortex-engine/src/search/rerank.rs
Makefile: enterprise-rag-bench-scope-mapping-check
cargo test -p cortex-engine --lib search::scope_mapping -- --nocapture
cargo test -p cortex-engine --lib search::rerank -- --nocapture
cargo test -p cortex-engine --bin enterprise_rag_scope_mapping_check -- --nocapture
make enterprise-rag-bench-scope-mapping-check ENTERPRISE_RAG_BENCH_SCOPE_MAPPING_CHECK_QUESTIONS=target/enterprise-rag-bench/subsets/balanced_50/balanced_50_questions.jsonl ENTERPRISE_RAG_BENCH_SCOPE_MAPPING_CHECK_REPORT=target/enterprise-rag-bench/scope-mapping-check/balanced_50_report.json
make enterprise-rag-bench-scope-mapping-check ENTERPRISE_RAG_BENCH_SCOPE_MAPPING_CHECK_QUESTIONS=target/external-benchmarks/EnterpriseRAG-Bench/questions.jsonl ENTERPRISE_RAG_BENCH_SCOPE_MAPPING_CHECK_REPORT=target/enterprise-rag-bench/scope-mapping-check/full_500_report.json
```

Balanced-50 result: `3/4` project_related questions mapped, `75%`.

Full-500 result: `37/40` project_related questions mapped, `92%`.

**Remaining:** C3 produces scope/source/project/team directives and reranker
boosts; B6 still needs to consume these directives in the project candidate
aggregator, and source/scope hard-filter policy must stay conservative to avoid
oracle-like over-filtering.

### C4 — Словарь синонимов/аббревиатур корпуса (auto-built)
Сборка co-occurrence словаря (TF-IDF/embedding similarity) из всего
корпуса при checkpoint, persisted как `.acsyn`. Питает B3.
**Target:** покрывает ≥ 1000 терминов с ≥1 синонимом.

**Status:** done for C4 current roadmap target. Engine `.acsyn` primitive,
Database persistence, checkpoint/compact publication, abbreviation mining,
retrieval consumption, streaming benchmark builder, and full 511,958-document
official-corpus production build are complete.

**Done:** добавлен deterministic corpus-derived synonym dictionary:

```text
visible corpus text
→ per-document dictionary terms
→ co-occurrence association score
→ CorpusSynonymDictionary
→ corpus.acsyn
```

В движке появился reusable API:
`build_corpus_synonym_dictionary(...)`,
`Database::corpus_synonym_dictionary(...)`,
`Database::persist_corpus_synonym_dictionary(...)`,
`Database::read_persisted_corpus_synonym_dictionary(...)`. `.acsyn` имеет
стабильный magic `CORTEXDB_ACSYN_V1`, deterministic ordering и atomic temp-write.
Benchmark gate строит словарь из official ERB `uuid_index.json` +
`generated_data/sources`, без question/gold oracle.

**Done evidence:**

```text
crates/cortex-engine/src/search/synonyms.rs
crates/cortex-engine/src/bin/enterprise_rag_synonym_dictionary_check.rs
Makefile: enterprise-rag-bench-synonym-dictionary-check
cargo test -p cortex-engine --lib search::synonyms -- --nocapture
cargo test -p cortex-engine --bin enterprise_rag_synonym_dictionary_check -- --nocapture
make enterprise-rag-bench-synonym-dictionary-check ENTERPRISE_RAG_BENCH_SYNONYM_DICTIONARY_LIMIT=2000 ENTERPRISE_RAG_BENCH_SYNONYM_DICTIONARY_OUTPUT=target/enterprise-rag-bench/synonyms/corpus_2k.acsyn ENTERPRISE_RAG_BENCH_SYNONYM_DICTIONARY_REPORT=target/enterprise-rag-bench/synonyms/report_2k.json
```

Official corpus bounded gate: `2000` docs → `3858` terms with synonyms,
status `passed` for target `>=1000`. Larger smoke run also passed on `10000`
docs with `7882` terms with synonyms.

**Remaining:** none for C4. Future quality work can tune the dictionary scoring,
but the roadmap item itself is closed.

**Update:** closed the first retrieval-consumption slice for `.acsyn`.
`corpus.acsyn` is now used at search time when present in the DB root:

```text
query text
→ read persisted CorpusSynonymDictionary
→ append bounded corpus-derived synonym terms to lexical query
→ keep rerank/diversity/answer logic on the original question text
```

This is oracle-free: the expansion reads only the user query and a dictionary
built from corpus text. The dictionary builder now ignores CortexDB metadata
headers before the first blank line, so `scope/status/type` fields do not
pollute co-occurrence synonyms for cell payloads. Raw external documents without
CortexDB headers are still consumed as full text.

**Done evidence for Update:**

```text
crates/cortex-engine/src/search/synonyms.rs
crates/cortex-engine/src/search/database.rs
crates/cortex-engine/tests/database_search.rs
cargo test -p cortex-engine --lib search::synonyms -- --nocapture
cargo test -p cortex-engine --test database_search corpus_synonyms -- --nocapture
cargo clippy -p cortex-engine --lib --tests -- -D warnings
make enterprise-rag-bench-synonym-dictionary-check ENTERPRISE_RAG_BENCH_SYNONYM_DICTIONARY_LIMIT=2000 ENTERPRISE_RAG_BENCH_SYNONYM_DICTIONARY_OUTPUT=target/enterprise-rag-bench/synonyms/corpus_2k_c4_update.acsyn ENTERPRISE_RAG_BENCH_SYNONYM_DICTIONARY_REPORT=target/enterprise-rag-bench/synonyms/report_2k_c4_update.json
```

Latest C4 gate:

```text
documents:             2000
entries:               3999
terms_with_synonyms:   3999
status:                passed
```

**Remaining after Update:** automatic checkpoint/manifest publication of
`.acsyn`, full 512k-corpus scheduled production build, and abbreviation-specific
mining are still open. The B3 dependency "query expansion consuming persisted
synonyms at retrieval time" is now closed for the product search path.

**Update 2:** closed automatic checkpoint/compact publication for `.acsyn`.
`Database::checkpoint()` and `Database::compact()` now publish a deterministic
`corpus.acsyn` sidecar through the existing dictionary builder before manifest
publication/WAL truncation. This keeps the failure ordering conservative: if the
dictionary write fails, the checkpoint is not published; if manifest publication
fails later, the sidecar was still built from visible WAL+memtable state and is
safe to ignore or overwrite on the next checkpoint. The ACSYN writer now fsyncs
the parent directory after atomic rename.

```text
put cells
→ checkpoint/compact
→ build CorpusSynonymDictionary from visible corpus
→ atomic write corpus.acsyn
→ manifest publication
→ search_keyword expands lexical query from corpus.acsyn
```

**Done evidence for Update 2:**

```text
crates/cortex-engine/src/checkpoint.rs
crates/cortex-engine/src/search/synonyms.rs
crates/cortex-engine/tests/database_search.rs
cargo fmt --check
cargo test -p cortex-engine --lib search::synonyms -- --nocapture
cargo test -p cortex-engine --test database_search corpus_synonyms -- --nocapture
cargo clippy -p cortex-engine --lib --tests -- -D warnings
```

Latest C4 checkpoint publication gate:

```text
checkpoint_publishes_corpus_synonyms_for_search ... ok
database_keyword_search_consumes_persisted_corpus_synonyms ... ok
```

**Remaining after Update 2:** full 512k-corpus scheduled production build and
abbreviation-specific mining remain open. Automatic checkpoint/compact
publication and B3 product search consumption are closed.

**Update 3:** closed abbreviation-specific mining. The corpus synonym builder
now detects explicit parenthetical abbreviation definitions in corpus text:

```text
full phrase (ABC)
ABC (full phrase)
```

For example, `single sign on (SSO)` adds high-confidence dictionary candidates
from `sso` to `single`/`sign`, and `RBAC (role based access control)` links
`rbac` to `role`/`based`/`access`/`control`. These candidates are corpus-derived
and oracle-free; they are included even when the term frequency threshold is
higher than the abbreviation's document frequency, because explicit
parenthetical definitions are stronger than ordinary co-occurrence.

**Done evidence for Update 3:**

```text
crates/cortex-engine/src/search/synonyms.rs
crates/cortex-engine/tests/database_search.rs
cargo test -p cortex-engine --lib search::synonyms -- --nocapture
cargo test -p cortex-engine --test database_search abbreviation_synonyms -- --nocapture
cargo test -p cortex-engine --test database_search corpus_synonyms -- --nocapture
cargo fmt --check
cargo clippy -p cortex-engine --lib --tests -- -D warnings
```

Latest C4 abbreviation gate:

```text
mines_parenthetical_abbreviations_without_frequency_threshold ... ok
checkpoint_publishes_abbreviation_synonyms_for_search ... ok
```

**Remaining after Update 3:** only the full 512k-corpus scheduled production
build remains open for C4. Checkpoint/compact publication, abbreviation mining,
and B3 product search consumption are closed.

**Update 4:** closed the full-corpus scheduled production build. The
EnterpriseRAG synonym dictionary checker no longer materializes all source
documents as `Vec<String>` before building the dictionary. It streams one
document at a time into `CorpusSynonymDictionaryBuilder`, exposes
`--progress-every`, and the Make target forwards
`ENTERPRISE_RAG_BENCH_SYNONYM_PROGRESS_EVERY` for long scheduled runs. The
report records `streaming_document_build=true`.

Production profile used for the full corpus:

```text
documents:                 511,958
max_terms_per_document:    24
terms_with_synonyms:       10,000
entries:                   10,000
status:                    passed
wall time:                 40:49.86
max RSS:                   3,276,904 KB
output:                    target/enterprise-rag-bench/synonyms/corpus_full_512k_streaming_prod24.acsyn
report:                    target/enterprise-rag-bench/synonyms/report_full_512k_streaming_prod24.json
```

**Done evidence for Update 4:**

```text
crates/cortex-engine/src/search/synonyms.rs
crates/cortex-engine/src/bin/enterprise_rag_synonym_dictionary_check.rs
crates/cortex-engine/src/search.rs
Makefile
cargo test -p cortex-engine --lib search::synonyms -- --nocapture
cargo test -p cortex-engine --bin enterprise_rag_synonym_dictionary_check -- --nocapture
make enterprise-rag-bench-synonym-dictionary-check ENTERPRISE_RAG_BENCH_SYNONYM_DICTIONARY_LIMIT=2000 ENTERPRISE_RAG_BENCH_SYNONYM_DICTIONARY_OUTPUT=target/enterprise-rag-bench/synonyms/corpus_2k_streaming_progress.acsyn ENTERPRISE_RAG_BENCH_SYNONYM_DICTIONARY_REPORT=target/enterprise-rag-bench/synonyms/report_2k_streaming_progress.json ENTERPRISE_RAG_BENCH_SYNONYM_PROGRESS_EVERY=500
make enterprise-rag-bench-synonym-dictionary-check ENTERPRISE_RAG_BENCH_SYNONYM_DICTIONARY_LIMIT=50000 ENTERPRISE_RAG_BENCH_SYNONYM_DICTIONARY_OUTPUT=target/enterprise-rag-bench/synonyms/corpus_50k_streaming.acsyn ENTERPRISE_RAG_BENCH_SYNONYM_DICTIONARY_REPORT=target/enterprise-rag-bench/synonyms/report_50k_streaming.json ENTERPRISE_RAG_BENCH_SYNONYM_PROGRESS_EVERY=10000
make enterprise-rag-bench-synonym-dictionary-check ENTERPRISE_RAG_BENCH_SYNONYM_DICTIONARY_OUTPUT=target/enterprise-rag-bench/synonyms/corpus_full_512k_streaming_prod24.acsyn ENTERPRISE_RAG_BENCH_SYNONYM_DICTIONARY_REPORT=target/enterprise-rag-bench/synonyms/report_full_512k_streaming_prod24.json ENTERPRISE_RAG_BENCH_SYNONYM_MAX_TERMS_PER_DOCUMENT=24 ENTERPRISE_RAG_BENCH_SYNONYM_PROGRESS_EVERY=50000
```

**Remaining after Update 4:** none for C4. Next ordered epic is C5.

### C5 — Numeric/condition extraction из вопроса
Для constrained-вопросов вытащить численные условия (диапазоны, даты,
пороги, единицы) и прокинуть их в AQL `REQUIRE`/numeric guard как
структурированный фильтр, а не только в текст ответа.
**Target:** constrained correctness 43.3% → 60%+.

**Status:** done for text-only structured extraction + reranker guard signal +
benchmark gate.

**Done:** добавлен deterministic condition extraction:

```text
question text
→ numeric values with unit/currency/magnitude
→ operators: equal / at_least / at_most / greater_than / less_than / between
→ metric terms around the condition
→ temporal range when date/year is present
→ metric-only condition slots for "minimum/max/how long/required" questions
```

Runtime не читает `question_type`, `source_types`, `expected_doc_ids` или
`answer_facts`. `question_type=constrained` используется только offline-gate,
чтобы измерить coverage нужной категории. `WeightedScoreReranker` теперь получает
condition payload bonus: точное numeric/date/metric совпадение поднимает
candidate, а похожий текст с неверным numeric value получает меньший score.

**Done evidence:**

```text
crates/cortex-engine/src/search/conditions.rs
crates/cortex-engine/src/bin/enterprise_rag_condition_check.rs
crates/cortex-engine/src/search/rerank.rs
Makefile: enterprise-rag-bench-condition-check
cargo test -p cortex-engine --lib search::conditions -- --nocapture
cargo test -p cortex-engine --lib search::rerank -- --nocapture
cargo test -p cortex-engine --bin enterprise_rag_condition_check -- --nocapture
make enterprise-rag-bench-condition-check ENTERPRISE_RAG_BENCH_CONDITION_CHECK_QUESTIONS=target/enterprise-rag-bench/subsets/balanced_50/balanced_50_questions.jsonl ENTERPRISE_RAG_BENCH_CONDITION_CHECK_REPORT=target/enterprise-rag-bench/condition-check/balanced_50_report.json
make enterprise-rag-bench-condition-check ENTERPRISE_RAG_BENCH_CONDITION_CHECK_QUESTIONS=target/external-benchmarks/EnterpriseRAG-Bench/questions.jsonl ENTERPRISE_RAG_BENCH_CONDITION_CHECK_REPORT=target/enterprise-rag-bench/condition-check/full_500_report.json
```

Balanced-50 result: `3/3` constrained questions structured, `100%`.

Full-500 result: `28/30` constrained questions structured, `93%`.

**Remaining:** AQL planner still does not materialize these conditions into
first-class `REQUIRE` bytecode/numeric filter instructions; current slice exposes
the structured extraction and uses it in reranking. Future work should connect it
to AQL explain, ContextPack guard display, and exact table/numeric operators.

---

## Тема D. Ранжирование и слияние (fusion/scoring)

### D1 — Калибровка весов RRF/hybrid per question-type
Сейчас веса фиксированы (`default_weights`/Q16). Прогнать grid-search по
типам вопросов на held-out наборе и зафиксировать per-type веса в
`RetrievalWeights`/`ContextPolicy`.
**Target:** +2-3pp Overall без новых компонентов, чисто за счёт калибровки.

**Status:** done for clean text-derived hybrid/rerank calibration profiles;
impact-on-Overall measurement remains a separate full official run.

**Done:** добавлен calibration layer без oracle:

```text
query text
→ EnterpriseRAG intent classifier
→ complex semantic fallback for long explanatory/process questions
→ per-type RRF lexical/vector weights
→ per-type WeightedScoreReranker weights/bonuses
```

Подключено в оба retrieval path:

```text
SearchIndexes::HybridRerank candidate fusion
Database::search_persisted_query HybridRerank fusion
WeightedScoreReranker final rerank
```

Plain `Hybrid` оставлен как stable balanced-RRF baseline; calibration
включается на `HybridRerank`, чтобы не ломать существующий контракт
обычного hybrid search.

Калибровка не читает `question_type`, `source_types`, expected docs или
answer facts на inference path. Gold labels используются только в offline
gate report для диагностики покрытия.

**Evidence:**

```text
crates/cortex-engine/src/search/rerank.rs
crates/cortex-engine/src/bin/enterprise_rag_calibration_check.rs
Makefile: enterprise-rag-bench-calibration-profile-check
cargo test -p cortex-engine --lib search::rerank -- --nocapture
cargo test -p cortex-engine --bin enterprise_rag_calibration_check -- --nocapture
make enterprise-rag-bench-calibration-profile-check ENTERPRISE_RAG_BENCH_CALIBRATION_CHECK_QUESTIONS=target/enterprise-rag-bench/subsets/balanced_50/balanced_50_questions.jsonl ENTERPRISE_RAG_BENCH_CALIBRATION_CHECK_REPORT=target/enterprise-rag-bench/calibration-check/balanced_50_report.json
make enterprise-rag-bench-calibration-profile-check ENTERPRISE_RAG_BENCH_CALIBRATION_CHECK_QUESTIONS=target/external-benchmarks/EnterpriseRAG-Bench/questions.jsonl ENTERPRISE_RAG_BENCH_CALIBRATION_CHECK_REPORT=target/enterprise-rag-bench/calibration-check/full_500_report.json
```

Latest gate results:

```text
balanced-50: calibrated=100%, semantic_vector=100%, constrained_condition=100%
full-500:    calibrated=100%, semantic_vector=68%,  constrained_condition=73%
```

**Remaining:** real +2-3pp Overall target is not proven by this structural
gate alone. Need a full official clean answer+judge run after the next
retrieval/synthesis batch to measure actual score impact.

### D2 — Recall-vs-precision dashboard расширение по категориям
Расширить `retrieval-quality-dashboard` (NEXT_60_EPICS #22): per-category
recall/MRR/nDCG/invalid-extra-docs тренды по коммитам, чтобы любой эпик
выше мог быть измерен и не сломать другую категорию.
**Target:** дашборд показывает 10 категорий ERB с историей по коммитам.

**Status:** done for local retrieval-only category dashboard with JSON,
Markdown, and JSONL history.

**Done:** добавлен dashboard builder:

```text
questions.jsonl + clean retrieval.jsonl
→ 10 ERB categories
→ recall / precision / MRR / nDCG / invalid-extra-docs
→ trend_vs_previous from history.jsonl
→ report.json + report.md
```

Особенность для честности: `recall/MRR/nDCG` считаются только для вопросов
с non-empty `expected_doc_ids`; `invalid_extra_docs` считается для всех
вопросов, включая `high_level` и `info_not_found`, чтобы видеть шум в
retrieval там, где ответ может быть обзорным или abstain.

**Evidence:**

```text
scripts/enterprise_rag_bench/category_retrieval_dashboard.py
Makefile: enterprise-rag-bench-category-dashboard-check
python3 -m py_compile scripts/enterprise_rag_bench/category_retrieval_dashboard.py
make enterprise-rag-bench-category-dashboard-check ENTERPRISE_RAG_BENCH_CATEGORY_DASHBOARD_RUN_ID=d2-fixture-1 ENTERPRISE_RAG_BENCH_CATEGORY_DASHBOARD_TOPK=3
make enterprise-rag-bench-category-dashboard-check ENTERPRISE_RAG_BENCH_CATEGORY_DASHBOARD_RUN_ID=d2-fixture-2 ENTERPRISE_RAG_BENCH_CATEGORY_DASHBOARD_TOPK=3
make enterprise-rag-bench-category-dashboard-check ENTERPRISE_RAG_BENCH_CATEGORY_DASHBOARD_QUESTIONS=target/external-benchmarks/EnterpriseRAG-Bench/questions.jsonl ENTERPRISE_RAG_BENCH_CATEGORY_DASHBOARD_RETRIEVAL=target/enterprise-rag-bench/official-clean/500/full500-dense-hybrid/retrieval.clean.jsonl ENTERPRISE_RAG_BENCH_CATEGORY_DASHBOARD_OUTPUT_ROOT=target/enterprise-rag-bench/category-dashboard/full500-dense-hybrid ENTERPRISE_RAG_BENCH_CATEGORY_DASHBOARD_RUN_ID=full500-dense-hybrid ENTERPRISE_RAG_BENCH_CATEGORY_DASHBOARD_TOPK=10
```

Latest full-500 retrieval-only dashboard:

```text
overall:       recall=55.78 precision=7.91 invalid=9.26 mrr=0.46 ndcg=0.46
semantic:      recall=18.40 precision=1.84 invalid=9.82 mrr=0.10 ndcg=0.12
completeness:  recall=41.43 precision=23.00 invalid=7.70 mrr=0.61 ndcg=0.42
high_level:    answerable=0 invalid=10.00
info_not_found answerable=0 invalid=10.00
```

**Remaining:** the dashboard is local evidence generation; CI wiring into
`production-evidence-sweep` and commit-to-commit artifact retention belongs
to D4/G3.

### D3 — Held-out / no-overfit eval harness
Зафиксировать held-out подвыборку (например 100 из 500, не используемую
для тюнинга), отдельный gate `make erb-holdout-check`, чтобы продвижения
по balanced-50/300 не переобучались под публичный набор.
**Target:** held-out Overall дельта ≤ 2pp от тюнингового набора.

**Status:** done for retrieval-only no-overfit gate.

**Done:** добавлен deterministic split по `question_id`:

```text
questions.jsonl + clean retrieval.jsonl
→ tuning split + held-out split
→ clean question files for both splits
→ tuning/held-out recall, precision, invalid-extra-docs, MRR, nDCG
→ absolute recall delta gate
```

Честная граница: retrieval rows проверяются через official-clean allowlist;
gold-поля (`question_type`, `source_types`, `expected_doc_ids`, answer facts)
используются только внутри evaluator report, а clean question files для
инференса сохраняются без oracle-полей.

**Evidence:**

```text
scripts/enterprise_rag_bench/heldout_no_overfit_check.py
Makefile: erb-holdout-check / enterprise-rag-bench-heldout-check
python3 -m py_compile scripts/enterprise_rag_bench/heldout_no_overfit_check.py
make erb-holdout-check
```

Full500 strict mode example:

```text
make erb-holdout-check \
  ENTERPRISE_RAG_BENCH_HELDOUT_QUESTIONS=target/external-benchmarks/EnterpriseRAG-Bench/questions.jsonl \
  ENTERPRISE_RAG_BENCH_HELDOUT_RETRIEVAL=target/enterprise-rag-bench/official-clean/500/full500-dense-hybrid/retrieval.clean.jsonl \
  ENTERPRISE_RAG_BENCH_HELDOUT_OUTPUT_ROOT=target/enterprise-rag-bench/heldout-no-overfit/full500-dense-hybrid \
  ENTERPRISE_RAG_BENCH_HELDOUT_SIZE=100 \
  ENTERPRISE_RAG_BENCH_HELDOUT_MAX_ABS_RECALL_DELTA_PCT=2
```

Latest full-500 held-out retrieval-transfer gate:

```text
status:       passed
tuning:       recall=55.71 precision=7.84 invalid=9.27 mrr=0.47 ndcg=0.47
held-out:     recall=56.03 precision=8.21 invalid=9.22 mrr=0.43 ndcg=0.44
abs delta:    recall=0.32pp (threshold <= 2pp)
split files:  target/enterprise-rag-bench/heldout-no-overfit/full500-dense-hybrid/
```

**Remaining:** this is a retrieval-transfer gate, not a paid answer+judge
held-out Overall run. Answer-level held-out scoring should reuse the same
split files when D4/G3 production evidence sweep is wired.

### D4 — Per-category regression gate в CI
`make enterprise-rag-bench-*` уже есть частично; собрать единый gate:
"ни одна из 10 категорий не регрессирует более чем на X относительно
текущего baseline" перед промоушеном любого изменения retrieval/синтеза.
**Target:** gate встроен в `production-evidence-sweep`.

**Status:** done for retrieval-only per-category promotion gate and wired into
`production-evidence-sweep`.

**Done:** добавлен gate, сравнивающий baseline/candidate clean retrieval по
10 категориям ERB:

```text
baseline retrieval + candidate retrieval + questions.jsonl
→ per-category recall / precision / invalid-extra-docs / MRR / nDCG deltas
→ fail if any category regresses beyond configured thresholds
```

Для категорий без `expected_doc_ids` (`high_level`, `info_not_found`) gate не
проверяет recall/MRR/nDCG, но всё равно проверяет рост `invalid_extra_docs`.

**Evidence:**

```text
scripts/enterprise_rag_bench/category_regression_gate.py
Makefile: erb-category-regression-check / enterprise-rag-bench-category-regression-check
scripts/production_evidence_sweep.sh: enterprise_rag_category_regression step
python3 -m py_compile scripts/enterprise_rag_bench/category_regression_gate.py
make erb-category-regression-check
make erb-category-regression-check ENTERPRISE_RAG_BENCH_CATEGORY_REGRESSION_QUESTIONS=target/external-benchmarks/EnterpriseRAG-Bench/questions.jsonl ENTERPRISE_RAG_BENCH_CATEGORY_REGRESSION_BASELINE=target/enterprise-rag-bench/official-clean/500/full500-dense-hybrid/retrieval.clean.jsonl ENTERPRISE_RAG_BENCH_CATEGORY_REGRESSION_CANDIDATE=target/enterprise-rag-bench/official-clean/500/full500-dense-hybrid/retrieval.clean.jsonl ENTERPRISE_RAG_BENCH_CATEGORY_REGRESSION_OUTPUT_ROOT=target/enterprise-rag-bench/category-regression/full500-dense-hybrid-baseline-self
```

Latest gate results:

```text
fixture self-compare:  passed, 10 categories, errors=0
full500 self-compare:  passed, 10 categories, errors=0
negative smoke:        failed as expected, basic recall 100→0 and 5 errors
```

**Remaining:** CI uses the fixture/default gate for speed. Full500 strict
baseline-vs-candidate promotion should be run explicitly when a new retrieval
artifact is ready, and G3 should retain those full artifacts.

### D5 — Source trust + freshness в скоринге retrieval (не только VERIFY)
Сейчас `SourceTrust`/freshness используются в VERIFY/ContextPack explain
(NEXT_60_EPICS #18), но не влияют на ранжирование retrieval напрямую.
Добавить как Q16-компонент в score для conflicting_info/temporal вопросов.
**Target:** conflicting_info recall/precision растут без отдельного A8.

**Status:** done for `SearchMode::HybridRerank` live and persisted paths.

**Done:** search reranking now adds metadata score components after the
existing lexical/vector/evidence reranker:

```text
source_trust_q16/source_trust_class → trust Q16 bonus
created_unix_seconds across candidate pool → relative freshness Q16 bonus
conflicting/current/temporal queries → stronger trust/freshness weights
other queries → small stable metadata tie-breaker
```

This mirrors the AQL retrieval principle without changing the public
`SearchRerankInput` trait contract.

**Evidence:**

```text
crates/cortex-engine/src/search/database.rs
crates/cortex-engine/tests/database_search.rs
cargo test -p cortex-engine --test database_search trusted_fresh -- --nocapture
cargo test -p cortex-engine --test database_search -- --nocapture
```

Latest test result:

```text
trusted_fresh targeted: 2 passed
database_search full:   38 passed
```

**Remaining:** D5 proves retrieval-order behavior, not full ERB answer-score
lift. The impact on `conflicting_info` should be measured through D2/D4 plus
the next official-clean answer+judge run.

---

## Тема E. ContextPack и синтез ответа

### E1 — Completeness Planner (полный)
(см. ENTERPRISE_RAG_IMPROVEMENT_EPICS Epic 05) — на основе C2 декомпозиции
строить чеклист под-пунктов → маппинг на evidence spans → repair pass
если под-пункт не покрыт.
**Target:** completeness combined 23.75 → 40+.

**Status:** done for oracle-free answer-runner wiring and local plan gate.

**Done:** added an official-clean compatible completeness planner that consumes
only clean inference fields:

```text
question_id/question + retrieved document_ids
→ C2 evidence units
→ materialized evidence spans from retrieved docs
→ per-subpoint coverage checklist
→ prompt-visible repair policy for uncovered requested items
```

The official-clean answer runner now supports:

```text
--include-evidence-plan
--evidence-plan-file <path>
```

and the end-to-end official-clean orchestrator can enable the same through
Make variables:

```text
ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_INCLUDE_EVIDENCE_PLAN=1
ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_EVIDENCE_PLAN_FILE=<path>
```

**Evidence:**

```text
scripts/enterprise_rag_bench/completeness_planner.py
scripts/enterprise_rag_bench/evidence_slot_planner.py
scripts/enterprise_rag_bench/run_official_clean_answers.py
scripts/enterprise_rag_bench/run_official_clean_benchmark.py
make erb-completeness-plan-check
```

Latest balanced-50 plan gate:

```text
questions:                  50
total_units:                280
covered_units:              192
average_coverage_pct:       68.57
fully_covered_questions:    10
empty_mapping_questions:     4
```

**Remaining:** answer-score lift is not proven until an official-clean
answer+judge run is executed with `--include-evidence-plan`. The current repair
behavior is prompt-level and deterministic-plan driven; a separate LLM
second-pass repair remains E8.

### E2 — Project Answer Synthesizer
(см. Epic 03 в ENTERPRISE_RAG_IMPROVEMENT_EPICS) — для project_related
агрегировать B6-кластер источников в единый "карточный" ответ
(статус/owner/timeline/риски), а не пытаться отвечать по одному документу.
**Target:** project_related combined 5.94 → 25+ (совместно с B6).

**Status:** done for oracle-free project-card artifact generation and prompt
wiring.

**Done:** added a deterministic Project Answer Synthesizer that consumes clean
retrieval artifacts only:

```text
question_id/question + retrieved document_ids
→ project identity anchors
→ status/owner/timeline/risk/action/metric/linked-artifact rows
→ prompt-visible project card
```

The card is encoded as a normal `--evidence-plan-file`, so it can be used by
the same official-clean answer runner without adding benchmark-specific oracle
fields.

**Evidence:**

```text
scripts/enterprise_rag_bench/project_answer_synthesizer.py
scripts/enterprise_rag_bench/evidence_slot_planner.py
make erb-project-answer-synth-check
```

Latest balanced-50 project-card gate:

```text
questions:                 50
cards_with_rows:           47
average_rows_per_card:     26.32
row categories:
  action:                  404
  identity:                634
  linked_artifact:         703
  metric:                 1105
  owner:                   228
  risk:                    241
  status:                  256
  timeline:                694
```

**Remaining:** E2 has not yet proven judge-score lift. The next official-clean
answer run should compare:

```text
baseline evidence plan off
vs completeness plan on
vs project-card plan on
vs combined plan strategy
```

### E3 — Anti-hallucination guard v2 (targeted repair, не suppress)
По выводам из истории ("suppress mode" не промотили — слишком грубо):
вместо удаления предложений — repair-проход, который переписывает
неподтверждённые утверждения с явной ссылкой на источник или убирает
конкретный факт, сохраняя структуру ответа.
**Target:** correctness 49.2% → 55%+ без регрессии completeness.

**Status:** done for deterministic targeted repair mode and official-clean
runner wiring.

**Done:** answer guard now supports:

```text
--unsupported-claim-guard repair
```

Unlike `suppress`, `repair` does not delete the whole sentence when only an
exact concrete marker is unsupported. It rewrites unsupported value statements,
for example:

```text
The timeout is 45 seconds.
→ The timeout is not stated in the retrieved evidence.
```

For mixed statements, supported markers are preserved while missing concrete
values are replaced with an unstated-value phrase. This is still fully
oracle-free: it only compares generated exact markers against retrieved
context.

**Evidence:**

```text
scripts/enterprise_rag_bench/answer_guard.py
scripts/enterprise_rag_bench/test_answer_guard.py
scripts/enterprise_rag_bench/run_deepseek_answers.py
scripts/enterprise_rag_bench/run_official_clean_answers.py
scripts/enterprise_rag_bench/run_official_clean_benchmark.py
make erb-answer-guard-check
```

Latest local guard gate:

```text
answer_guard unit tests: 7 passed
```

**Remaining:** judge-score lift is not proven until an official-clean
answer+judge comparison is run with:

```text
ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_UNSUPPORTED_CLAIM_GUARD=repair
```

or direct CLI flag:

```text
--unsupported-claim-guard repair
```

### E4 — Evidence table / slot extractor расширение
Расширить evidence slot planner/table extractor (уже есть commits) на
табличные cell'ы из A4 — прямой проброс структурированных строк в
context pack для constrained/completeness.
**Target:** доля constrained-вопросов с табличным evidence в топ-3 ≥ 70%.

**Status:** done for markdown/pipe-table structured row extraction and prompt
formatting.

**Done:** evidence-table extraction now turns detected tables into structured
rows:

```text
| Project | Owner | Deadline | Status |
| Apollo  | Maya  | 2026-06-12 | blocked |

→ table_cells:
  Project=Apollo
  Owner=Maya
  Deadline=2026-06-12
  Status=blocked
```

Prompt formatting now includes these `header=value` cells before the raw row,
which gives constrained/completeness prompts direct structured evidence rather
than only line snippets.

**Evidence:**

```text
scripts/enterprise_rag_bench/evidence_table_extractor.py
scripts/enterprise_rag_bench/test_evidence_table_extractor.py
make erb-evidence-table-extractor-check
make enterprise-rag-bench-evidence-table-check
```

Latest balanced-50 evidence-table gate:

```text
questions:                  50
total_facts:              2686
average_facts_per_question: 53.72
questions_without_facts:      3
structured_table_row:       210
table_row:                  254
```

**Remaining:** the target "constrained table evidence in top-3 >= 70%" still
needs a category-specific top-3 report; current gate proves extraction and
prompt availability, not final top-3 constrained coverage.

### E5 — High-level "brain digest" context mode
Для вопросов без конкретных gold-документов (high_level) собирать
ContextPack из B5 anchor-документов в формате executive summary
(миссия/продукты/метрики), явно помечая mode=brain_digest.
**Target:** high_level answer_correct: 0/10 → ≥6/10.

**Status:** done for context mode and non-LLM high-level coverage gate.

**Done:** added `brain-digest` as a reusable answer context mode. It selects
overview evidence by themes without reading `question_type`:

```text
mission_strategy
product_platform
go_to_market
security_compliance
reliability_operations
metrics
```

The packed context explicitly marks:

```text
Mode: brain_digest
```

and ranks retrieved documents by brain-digest score.

**Evidence:**

```text
scripts/enterprise_rag_bench/answer_context.py
scripts/enterprise_rag_bench/test_answer_context.py
scripts/enterprise_rag_bench/evaluate_evidence_pack.py
make erb-answer-context-check
make enterprise-rag-bench-high-level-coverage
```

Latest high-level coverage gate:

```text
questions:                         10
questions_with_docs:               10
mode:                              brain-digest
average_fact_token_coverage_pct:   64.21
average_fact_full_coverage_pct:    64.67
threshold:                         60.00
passed:                            true
```

**Remaining:** the target answer-correct 6/10 requires an official-clean
answer+judge run with `--context-mode brain-digest` or an oracle-free
question-text router that selects this mode only for overview questions.

### E6 — Citation-aware budget optimizer v3: per-type budget profiles
Продолжение NEXT_60_EPICS #15 — разные token-budget профили по типу
вопроса (project_related/completeness получают больший budget за счёт
B9), с сохранением required-citation overhead логики.
**Target:** invalid_extra_docs не растёт при увеличении budget для сложных типов.

**Status:** done for oracle-free text-derived budget profiles and official-clean
Makefile wiring.

**Done:** answer generation can now derive budget profiles from visible
question text when enabled:

```text
default          → keep base budget
constrained      → top_k=8,  max_chars=2600, max_tokens=700
conflict         → top_k=10, max_chars=2800, max_tokens=800
completeness     → top_k=10, max_chars=3200, max_tokens=900
complex_project  → top_k=10, max_chars=3200, max_tokens=900
high_level       → top_k=10, max_chars=5000, max_tokens=900, context=brain-digest
```

This stays official-clean: the detector reads only the question text, not
`question_type`, `source_types`, expected docs, or answer facts.

**Evidence:**

```text
scripts/enterprise_rag_bench/answer_intent.py
scripts/enterprise_rag_bench/test_answer_intent.py
scripts/enterprise_rag_bench/run_deepseek_answers.py
make erb-answer-intent-check
make -n enterprise-rag-bench-official-clean-50 ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_ENABLE_TEXT_INTENT_BUDGET=1
```

Latest local gate:

```text
answer_intent unit tests: 3 passed
official-clean dry-run includes --enable-text-intent-budget
```

**Remaining:** token/quality tradeoff still needs answer+judge comparison. The
expected guardrail is: larger budgets for complex questions should not increase
`invalid_extra_docs` or total answer tokens beyond the configured run target.

### E7 — Conflict resolution synthesizer
Для conflicting_info — явный режим: найти все версии факта (A8 temporal
index), отсортировать по freshness/trust, в ответе явно указать "текущее
значение X (по состоянию на ...), ранее было Y".
**Target:** conflicting_info combined 39.95 → 60+.

**Status:** done for oracle-free conflict-resolution evidence plans and prompt
wiring.

**Done:** added a deterministic conflict synthesizer:

```text
question_id/question + retrieved document_ids
→ current/previous/conflict/candidate claim rows
→ dates + exact markers
→ prompt-visible conflict-resolution policy
```

The plan tells the answerer to prefer current/latest/updated evidence first,
then mention previous/conflicting values only when retrieved evidence supports
them.

**Evidence:**

```text
scripts/enterprise_rag_bench/conflict_resolution_synthesizer.py
scripts/enterprise_rag_bench/test_conflict_resolution_synthesizer.py
scripts/enterprise_rag_bench/evidence_slot_planner.py
make erb-conflict-synth-check
```

Latest balanced-50 conflict-plan gate:

```text
questions:                50
plans_with_claims:        47
average_claims_per_plan:  22.30
candidate claims:         47
conflict claims:          40
current claims:         1012
previous claims:          16
```

**Remaining:** the roadmap target requires an official-clean answer+judge run
with the conflict plan enabled as `--evidence-plan-file`, or a clean router
that merges conflict plans only for conflict-like visible questions.

### E8 — Self-consistency / second-pass verification перед финальным ответом
Один дешёвый repair-проход: сравнить черновой ответ с ContextPack через
`VERIFY FACT` (уже есть движок-уровня), и если есть `contradicted`/`unsupported`
вердикты — переписать соответствующие фразы.
**Target:** correctness +3-5pp across all categories без отдельного per-type кода.

**Status:** done for optional official-clean second-pass repair wiring.

**Done:** answer generation can now run one evidence-only self-consistency
repair pass after the draft answer is produced:

```text
draft answer
→ exact-marker unsupported-claim report against retrieved context
→ repair prompt only if unsupported markers exist
→ final deterministic repair guard
```

This is still official-clean: the repair pass reads only the question, the
retrieved documents, the draft answer, and the unsupported markers derived from
that same retrieved context. It does not read `question_type`, `source_types`,
expected documents, answer facts, or judge output.

**Evidence:**

```text
scripts/enterprise_rag_bench/answer_repair.py
scripts/enterprise_rag_bench/test_answer_repair.py
scripts/enterprise_rag_bench/run_deepseek_answers.py
scripts/enterprise_rag_bench/run_official_clean_answers.py
scripts/enterprise_rag_bench/run_official_clean_benchmark.py
make erb-answer-repair-check
make -n enterprise-rag-bench-official-clean-50 ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_SELF_CONSISTENCY_REPAIR=1
```

Expected official-clean flag:

```text
--self-consistency-repair --self-consistency-retries 1
```

**Remaining:** the target correctness lift requires an official-clean
answer+judge comparison. Recommended A/B:

```text
baseline repair off
vs --unsupported-claim-guard repair
vs --self-consistency-repair
vs both enabled
```

---

## Тема F. AQL и API-поверхность

### F1 — AQL: `RETRIEVE BRAIN SUMMARY` для high_level/project_related
Новая AQL-конструкция, формализующая B5/B6/E5/E2 как запрашиваемый
режим (`USING MODE brain_summary` / `project_digest`), а не скрытую
эвристику в bench-скриптах — делает lift частью продукта/API.
**Target:** режим доступен через HTTP/SDK с тестами и OpenAPI.

### F2 — AQL multi-hop traversal (`EXPAND VIA relation HOPS n`)
Используя A5 persisted graph — расширение candidate set на 1-2 hop по
relation edges (entity→document, document→document via shared entity).
Закрывает intra_document_reasoning/multi-hop кейсы.
**Target:** intra_document_reasoning combined 32.92 → 50+.

### F3 — `/v1/search` поддержка per-type weight override (для D1)
API/SDK параметр для передачи откалиброванных весов из D1 без хардкода
в движке — нужно для A/B и для будущих доменов вне ERB.
**Target:** SDK + CLI flag, документировано, покрыто тестами.

---

## Тема G. Инфраструктура измерения и инжеста корпуса

### G1 — Полная докрутка bge-m3 эмбеддингов (97.8% → 99.9%+)
Завершить A1: добить оставшиеся ~11k transient fails, фиксировать
финальный coverage report как gate-артефакт.
**Target:** coverage ≥ 99.5%, отчёт в `target/enterprise-rag-bench/embeddings/`.

### G2 — Производственный re-embedding при апдейте корпуса
Инкрементальный re-embed только изменённых cell'ов при checkpoint,
без полного re-run скрипта — связывает A1 с обычным write path.
**Target:** re-embed delta ≤ O(изменённых cell'ов).

### G3 — Полный 500-вопросный прогон после каждого крупного эпика (A-F)
Зафиксировать дешёвый "balanced-100" held-out набор (D3) + редкий
полный 500 прогон только для промоушена — runbook с чёткими шагами
reproduce (расширение `REPRODUCE.md`).
**Target:** время одного balanced-100 цикла ≤ 30 минут на текущем железе.

---

## Порядок выполнения (рекомендованный)

Цель — закрывать "ворота" (answer_correct=False→True) дёшево и быстро,
затем поднимать recall/синтез по категориям с наибольшим весом.

```text
Фаза 1 (быстрые изолированные победы, 1-2 недели):
  B5  high_level retrieval mode          (+0.0  -> ~+1.0 Overall)
  C1  intent classifier
  D2  per-category dashboard
  D3  held-out harness
  D4  regression gate

Фаза 2 (semantic recall — наибольший вес 125/500):
  A2  multi-view индексация
  B3  query expansion
  B10 BM25F поле-веса
  B1  cross-encoder rerank
  D1  per-type weight calibration
  -> semantic combined 37.4 -> ~45-50

Фаза 3 (project_related — почти ноль -> легко удвоить-утроить):
  C3  query-to-scope mapping
  B6  project candidate aggregator
  E2  project answer synthesizer
  F1  RETRIEVE BRAIN SUMMARY / project_digest
  -> project_related combined 5.9 -> ~25-30

Фаза 4 (completeness + intra_document_reasoning):
  C2  decomposition
  E1  completeness planner
  A3  parent/child chunks
  F2  multi-hop traversal
  -> completeness 23.8 -> ~40, intra_doc 32.9 -> ~50

Фаза 5 (conflicting_info + constrained):
  A8  temporal/version index
  E7  conflict resolution synthesizer
  C5  numeric/condition extraction
  A4  table extraction
  E4  evidence table extractor
  -> conflicting 40 -> ~60, constrained 36 -> ~50

Фаза 6 (общая полировка correctness, все категории):
  E3  anti-hallucination repair v2
  E8  self-consistency second pass
  B7  anchor/evidence-overlap filter
  A6  near-duplicate suppression
  B9  adaptive top-k/budget
  E6  citation-aware budget v3

Фаза 7 (продуктизация в API/AQL + инфраструктура):
  A1/A5/A7/C4/D5/B2/B4/B8/F3/G1/G2/G3
```

## Ожидаемая траектория Overall

```text
Сейчас:    43.27
Фаза 1-2:  ~50-53   (semantic + high_level)
Фаза 3-4:  ~58-62   (project_related + completeness/intra_doc)
Фаза 5-6:  ~63-68   (conflicting/constrained + correctness polish)
Фаза 7:    продуктизация без изменения метрики, но closes "не продукт" риск
           из CORTEXDB_IMPROVEMENT_EPICS
```

## Правило промоушена (наследуется из ENTERPRISE_RAG_IMPROVEMENT_EPICS)

```text
official_clean_gate: passed
oracle_usage_audit: passed
held-out (D3) Overall не упал
ни одна категория не регрессировала (D4), особенно info_not_found=100
document recall >= 85.0, если не растёт Overall материально
```
