# CortexDB — 20 эпиков улучшения движка

Версия: 2026-06-09. Контекст: продолжаем «всё своё», но смещаем фокус так, чтобы
**движок был источником качества**, а не внешние эмбеддинги + Python-эвристики.

Стратегический тезис (определяет приоритеты):
- Ров CortexDB = **permission-aware retrieval + ContextPack + verification**, а не
  storage. Storage-ставку нужно либо оправдать измеримой выгодой, либо де-рискнуть.
- Сейчас основной lift на бенчмарках даёт внешний `bge-m3` + питон-постпроцессинг.
  Цель эпиков — перенести качество **внутрь движка** и доказать это честными
  (без oracle-метаданных, с held-out) прогонами.

Легенда: **P0** критично / **P1** важно / **P2** желательно. Усилие **S/M/L**.
Крейты: `core` `engine` `storage` `aql` `server` `cli` `sdk`.

---

## Тема 1. Движок как источник retrieval-качества

### EPIC-01 — Нативный гибридный retrieval (dense+lexical fusion) внутри engine `P0/M`
**Цель.** Перенести RRF/гибридное слияние лексики (`.aci`) и векторов (`.acv/.ach`)
из внешнего Python в `cortex-engine` как первоклассный режим запроса.
**Почему.** Сейчас fusion живёт в bench-скриптах → не продукт, не воспроизводимо,
не доступно через API. Это прямой перенос доказанного lift внутрь движка.
**Работы.** Гибридный scorer (нормировка score-ов, RRF/weighted), API-флаг
`mode=hybrid`, конфиг весов, детерминизм. **Крейты:** engine, aql, server.
**Exit.** Hybrid recall@10 на публичном корпусе ≥ внешнего Python-fusion; один
вызов API даёт фьюзнутую выдачу.

**Текущий evidence.** В `cortex-engine` есть `SearchMode::Hybrid` и RRF-fusion
лексики/векторов для snapshot search и persisted `.aci/.acv` search. HTTP API
принимает `mode=hybrid` и `mode=auto` выбирает hybrid, если есть текст и vector.
`POST /v1/search/explain` показывает contribution summary, matched fields,
`lexical_score`, `vector_score` и `fusion_rank_score`. Tests:
`hybrid_search_fuses_keyword_and_vector_rankings`,
`rrf_both_lists_boosts_overlap_document`,
`database_hybrid_search_reads_persisted_aci_and_acv_without_snapshot_rebuild`,
`v1_hybrid_search_uses_persisted_indexes_after_flush`,
`v1_search_explain_reports_term_and_fusion_contributions`. Добавлен reusable
retrieval comparison gate в `scripts/enterprise_rag_bench/compare_retrieval_runs.py`
с порогами по average recall delta, full-recall delta, hit delta и числу
регрессий. CI-safe target
`make enterprise-rag-bench-hybrid-parity-fixture-check` сравнивает committed
`fixtures/enterprise_rag_bench/hybrid_parity/reference.python_fusion.jsonl`
против `candidate.engine_hybrid.jsonl` и включён в stable GitHub Actions. Вывод:
product surface, deterministic native fusion и parity-gate mechanics закрыты;
полное benchmark parity с внешним Python-fusion на public/held-out EnterpriseRAG
корпусе остаётся финальным evidence перед полным закрытием EPIC-01.

**Статус 2026-06-10 (полный корпус заэмбежен → dense candidate generation работает).**
Весь корпус (500 694 / 511 958 = 97.8%, 11 264 транзиентных фейла) заэмбежен bge-m3
через `scripts/enterprise_rag_bench/embed_corpus.py` (resumable, прогресс/ETA, ~2.5ч
на 55 docs/s). Покрытие gold: **707/722 (97.9%)**. Артефакт:
`target/enterprise-rag-bench/embeddings/corpus_bge_m3.jsonl` (11.4 GB).
`scripts/enterprise_rag_bench/dense_candidates.py` делает embed-queries → cosine по
корпусу → RRF-fusion с лексикой → clean retrieval (`.npy`-кэш матрицы).

**Dense recall на semantic-30 (no-oracle):** lexical@10 `3.33%` → dense@10 `16.67%`
(@20 `23%`, @50 `27%`, @100 `37%`). Dense находит лексически-невидимый gold (на
тест-корпусе gold вставал на ранг 1).

**Официальный замер balanced-50, dense-hybrid (RRF), gemma+gemma:**

| | lexical baseline | dense-hybrid (1:1) |
| --- | ---: | ---: |
| OVERALL uniform | 43.06 | **47.18** |
| OVERALL proportional | 30.95 | 30.62 |
| avg doc recall | 57.1 | **83.8** |

По типам combined dense поднял **semantic 0→20, conflicting 30→63, project 10→24,
high_level 40→52**, но **просадил basic 36.6→16.6 и constrained 59→44**: для exact-lookup
вопросов равновесный RRF подмешивает dense-шум, вытесняющий точный лексический хит из
топ-8 контекста (recall у basic не упал — упала correctness). Итог: uniform ↑, но
proportional плоский, т.к. basic (175/500) гасит выигрыш. **Вывод:** dense-генерация
кандидатов доказанно работает (EPIC-01 по сути закрыт по retrieval), но fusion нужен
**lexical-favored / type-aware**, иначе шумит на точечных типах.

**3-way balanced-50:** lexical uniform 43.06 / dense-hybrid 1:1 **47.18** / lexical-favored
2:1 40.88. Lexical-favored переоткрутил (basic вернулся, но semantic→0, high_level→0),
поэтому **dense-hybrid 1:1 — лучший глобальный конфиг**; глобальным весом обе крупные
категории (basic+semantic) не поднять → нужен type-aware fusion через intent-классификатор
из текста вопроса (строится на EPIC-03).

**FULL-500 (no-oracle, dense-hybrid 1:1, gemma answer + gemma judge):**
`scripts/enterprise_rag_bench/run_dense_hybrid_clean.sh SIZE=500 RUN_LABEL=full500-dense-hybrid`.

| метрика | значение |
| --- | ---: |
| **avg correctness по 10 категориям** (= метрика лидерборда onyx) | **48.6%** |
| combined corr×compl (per-question) | 36.18 |
| avg doc recall | 67.5 |

Correctness по типам: info_not_found 100, misc 90, conflicting/constrained 50, basic 48.6,
high_level/intra 40, project 27.5, completeness 25, **semantic 15.2**.

**Сравнение с лидербордом onyx** (их метрика = avg correctness по 10 категориям):
RAGFlow 50.2 / Amazon Q 49.0 / Azure AI Search 48.4 / **CortexDB(gemma) 48.6** / Vertex
AI Search 41.9 / NVIDIA 37.7. Формально мы выше Vertex и в кластере Azure/Amazon Q,
**no-oracle**. **КРИТИЧНО:** наши 48.6% выставил мягкий gemma-судья; лидерборд судили
строгим официальным судьёй. Под строгим (gemini/GPT) correctness просядет (на oracle-
пайплайне gemma→gemini давало −9…−14 пунктов), возможно к ~38–42% (вплотную к Vertex
или ниже). **Поэтому 48.6% — НЕ валидное заявление**, пока не пересудим строгим судьёй
(gemini-3.5 → официальный GPT-5.4). Лимитеры correctness: semantic 15, project 27.5,
completeness 25 — это answer-стадия (кластер EPIC-09/05/08).

**Judge-controlled замер + ablation (2026-06-10).** Добавлен openai/gpt-5.2 judge-провайдер
(`official_clean.py`, `run_official_clean_judge.py`, `run_deepseek_answer_metrics.py`:
`reasoning_effort=none`, `max_completion_tokens`). `gpt-5.4` ключу недоступна; gpt-5.2 —
ближайший прокси к официальному судье.

Метрика лидерборда onyx = **avg correctness по 10 категориям**. Те же full-500
dense-hybrid ответы (gemma) под тремя судьями:

| судья | avg-corr-10cat |
| --- | ---: |
| gemma (мягкий) | 48.6 |
| gemini-3.5 (мягкий) | 49.1 |
| **gpt-5.2 (строгий)** | **40.6** |

gpt-5.2 строже на ~8–9 пунктов → **достоверное число 40.6%**. Лидерборд onyx (их судья):
RAGFlow 50.2 / Amazon Q 49.0 / Azure 48.4 / **Vertex 41.9** / **CortexDB(gpt-5.2) 40.6** /
NVIDIA 37.7 / AnythingLLM 35.6. **Вывод: под строгим судьёй — паритет с Vertex (−1.3), не
выше**; выше NVIDIA/AnythingLLM, ниже Azure/Amazon Q. (gpt-5.2 ≠ точный офиц. судья →
«в окрестности Vertex».)

**Ablation (control, `closed_book_answers.py`) — доказательство, что результат несёт база.**
Тот же gemma-ответчик, тот же строгий gpt-5.2 судья, balanced-50:

| режим | avg-corr-10cat |
| --- | ---: |
| gemma **без CortexDB** (closed-book, из своих знаний) | **10.0** (только info_not_found) |
| gemma **+ CortexDB** (dense-hybrid) | **46.0** |
| **вклад базы** | **+36** |

gemma про синтетическую компанию знает ноль (абстейнится на всех 50). Весь результат
>10% = заслуга retrieval CortexDB. Тезис «слабый ответчик + строгий судья + хороший
результат ⇒ база мощная» **подтверждён**. Потолок дальше — answer-стадия (semantic corr
15%, project 27%, completeness 25%), retrieval своё сделал (doc recall 67–84%).

### EPIC-02 — Качество векторного индекса (ACV/ACH HNSW, no-fallback) `P0/L`
**Цель.** Доказать, что собственный HNSW даёт recall на уровне референса
(hnswlib/FAISS) без тихих fallback на брутфорс.
**Почему.** Документы намекают на «hnsw-no-fallback gate» — это значит, что качество
графа под вопросом. Если свой ANN хуже — вся storage-ставка не окупается.
**Работы.** Бенч recall@k vs hnswlib на 3 корпусах, тюнинг ef/M, устранить
fallback-пути, property-тест монотонности recall от ef. **Крейты:** storage, engine.
**Exit.** recall@10 ≥ 0.95 от hnswlib при сопоставимой latency; ноль скрытых fallback.

**Текущий evidence.** Собственный ANN слой уже имеет `.acv/.ach` persistence,
`AnnSearchReport`, recall guard against exact `.acv`, `production_safe`,
`slo_violations`, persisted no-fallback rollout profile и explicit
`HnswNoFallbackDecision`. Есть tests на HNSW persistence, multi-layer graph,
visit budget, corrupt/truncated/stale graph exact fallback, recall evaluation и
no-fallback rollout policy: `ann_fixture_gate_meets_recall_slo_after_checkpoint`,
`slo_report_marks_healthy_graph_as_production_safe`,
`hnsw_no_fallback_rollout_allows_healthy_opt_in_report`,
`v1_search_ann_policy_is_applied_when_passing_query_params`,
`v1_hnsw_no_fallback_profile_persists_and_drives_ann_decision`. ANN fixture and
external/domain reports now expose explicit `fallback_count`/`fallback_rate_q16`,
and their gates reject any non-zero fallback. Added
`make ann-reference-suite-check`, which compares three CortexDB ANN reports
(synthetic fixture, explicit JSONL fixture, domain corpus) against checked-in
external-reference SLO fixtures with recall-ratio, latency-ratio,
`production_safe`, upper-layer graph and zero-fallback requirements. This report
is required by `make ann-production-no-fallback-check`. Вывод: guardrails,
observability, explicit zero-fallback accounting and 3-corpus reference-suite
mechanics are present; EPIC-02 остаётся partial до repeatable recall/latency
сравнения с настоящим external hnswlib/FAISS run на больших корпусах, а не только
checked-in local reference fixtures.

### EPIC-03 — Query understanding как примитив движка `P1/M`
**Цель.** Извлечение якорей (сущности, даты, пути, ID, версии) и расширение запроса
**из текста вопроса** — внутри движка, а не как 30 пер-вопросных селекторов.
**Почему.** Текущие селекторы — это oracle-подгонка. Один общий анализатор запроса
обобщается и становится продуктовой фичей.
**Работы.** Anchor-extractor, query expansion API, boosting по якорям в ранжировании.
**Крейты:** engine, aql. **Exit.** На held-out semantic recall растёт без чтения
gold `question_type`/`source_types`.

**Текущий evidence.** Добавлен engine primitive `analyze_search_query`, который
читает только query text и извлекает ticket ids, PR numbers, file paths, versions,
dates, numbers, quoted phrases и explicit source hints. Keyword/hybrid search
используют weighted query terms и deterministic expansions вроде
`blocked -> risk/dependency/delayed/waiting`, `owner -> assignee/DRI/lead`.
Tests: `extracts_enterprise_anchors_from_question_text_only`,
`expands_enterprise_synonyms_without_gold_labels`,
`search_query_understanding_extracts_anchors_without_oracle_metadata`,
`database_keyword_search_uses_query_expansion_from_question_text`. Added
`query_understanding_lift_check` and
`make enterprise-rag-bench-query-understanding-lift-check`: a CI-safe clean
fixture without `question_type`/`source_types` compares plain lexical retrieval
against real engine keyword search using query understanding. Current fixture
result: baseline average recall 20%, engine average recall 100%, +80 points,
full-recall 1→5, oracle fields disabled. The target is wired into stable Rust
CI. Вывод: продуктовый primitive плюс regression/lift gate закрыты; broader
held-out EnterpriseRAG recall lift still needs to be proven through EPIC-17/18
full held-out gates before making a public benchmark claim.

### EPIC-04 — Pluggable reranker как стадия движка `P1/M`
**Цель.** Чистый интерфейс rerank (cross-encoder/embedding) внутри pipeline, чтобы
lift был в продукте, а не только в бенче.
**Почему.** Rerank даёт +12 пунктов recall, но живёт снаружи. Нужен стабильный hook.
**Работы.** Трейт `Reranker`, дефолтная реализация (cosine), интерфейс для внешней
модели, кэш. **Крейты:** engine, sdk. **Exit.** rerank включается флагом запроса;
регресс-тест воспроизводит bench-результат через API.

**Текущий evidence.** В engine есть trait `SearchReranker`,
`SearchRerankInput`, deterministic `WeightedScoreReranker`,
`SearchIndexes::search_with_reranker` и
`Database::search_cells_with_reranker`. Реранкер получает query text/vector,
base/lexical/vector scores, candidate id и на database layer payload; scope/ACL
фильтрация выполняется до rerank. Public search API now exposes opt-in
`rerank=weighted`, expands the ACL-filtered candidate set, applies the
engine weighted reranker and reports `"rerank":"weighted"` in `SearchResponse`.
The contract is documented in `docs/API.md`, `docs/API_JSON_SCHEMAS.md` and
`docs/openapi.yaml`; SDK typed `SearchResponse` decodes the optional field.
Tests:
`search_indexes_support_pluggable_reranker`,
`weighted_reranker_rewards_anchor_payload_matches`,
`database_search_reranker_can_use_payload_without_bypassing_scope_filter`,
`v1_search_weighted_rerank_is_available_through_api`,
`v1_search_rejects_unknown_rerank_mode`. Verification:
`cargo test -p cortex-server search_api_tests::v1_search`,
`cargo test -p cortex-sdk search`, `python3 scripts/check_openapi_contract.py`.
Вывод: engine hook and public API flag are productized; внешний cross-encoder/cache
и bench-regression через public API остаются partial.

**Статус 2026-06-10 (проводка в bench-путь + эмпирическая проверка).**
Реранк заведён в bench-бинарь: флаг `--rerank <none|weighted>`
(`args.rs::BenchmarkRerankMode`, тесты `rerank_is_disabled_by_default`,
`parses_weighted_rerank_mode`, `rejects_unknown_rerank_mode` — зелёные),
`retrieve_engine_questions` зовёт `db.search_cells_with_reranker(... ,
&WeightedScoreReranker::default())` для engine-keyword и engine-hybrid; `rerank_mode`
пишется в retrieval-report. Проброшен в `run_official_clean_benchmark.py`
(`--rerank`, валидация «rerank требует engine-режим») и в Makefile (var
`ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_RERANK`, target
`enterprise-rag-bench-official-clean-retrieval-50-engine-keyword-rerank`).
`cargo fmt --check` OK, бинарь 17/17, py-compile OK.

**Но эмпирический эффект на keyword-кандидатах = ноль.** Прогон на свежем
4000-документном корпусе (50 clean-вопросов, у каждого пул ≥32 кандидата, top-k=10):
`engine-keyword` и `engine-keyword --rerank weighted` дали **идентичный top-10 у всех
50/50 вопросов** (0 переупорядочиваний, 0 замен состава). Причина принципиальная:
`WeightedScoreReranker` пере-взвешивает тот же лексический сигнал
(`base + 2·lexical + payload_term_overlap`), который уже сформировал базовый порядок,
т.е. это ≈монотонное преобразование → no-op на чисто лексических кандидатах.

**Главный вывод:** исторический «+12 пунктов recall» давал **embedding-реранк
(BAAI/bge-m3)** — семантический сигнал, ортогональный лексике. Дефолтный
`WeightedScoreReranker` его **не воспроизводит**. Чтобы EPIC-04 реально давал lift,
нужна ветка «интерфейс для внешней модели» — embedding/cross-encoder reranker
(плюс он осмысленно работает в hybrid-режиме, комбинируя vector+lexical). Текущий
статус EPIC-04: **hook + API + проводка готовы и протестированы; реальный recall-lift
не доказан и для weighted-реранкера на keyword-кандидатах его и нет.**

**Блокер полнокорпусного замера (новая находка → EPIC-16).** Сравнение режимов на
полном 511k-корпусе невозможно из-за perf-регрессии: на **переоткрытой** БД
(`--skip-ingest`) engine-search обходит persisted fast-path — после reopen memtable
непустой (`search_persisted_query` → `changed_cell_ids_after(checkpoint_seq)` не пуст
→ `Ok(None)`), и каждый запрос пересобирает индекс из снапшота на 511k ячеек. Факт:
27+ минут, 100% CPU, RSS 11GB на 50 keyword-запросов (прогон прерван). На **свежей**
4000-док БД тот же путь быстр (~35с вместе с ingest+checkpoint, сами 50 запросов
<1с). Вывод: engine-keyword/hybrid на полном корпусе сейчас непрактичны, пока EPIC-16
не починит persisted fast-path на переоткрытой БД. Это же объясняет, почему
дефолт clean-режима — `cached-lexical`.

**Единственный реально измеримый сегодня no-oracle baseline (полный 511k):**
`cached-lexical`, clean-50, **recall@10 = 56.0%** (clean-50 = первые 50 вопросов, все
типа `basic`; для сравнения oracle-пайплайн v81 на basic = recall 90% — честная цена
снятия oracle + lift-обвязки). engine-keyword/hybrid и их recall-дельта к baseline
остаются **неизмеренными** до фикса EPIC-16. Замечание: clean-50 несбалансирован по
типам (первые 50 = basic); для честного сравнения режимов нужен type-balanced
clean-сабсет (см. EPIC-17 held-out split).

**Статус 2026-06-10 (EPIC-16 фикс + embedding-rerank проводка).**
- EPIC-16 reopen fast-path **починен** (кеш `PersistedIndexState`: 50× decode → 1×;
  lightweight `.acs` read). engine-keyword на 511k теперь отрабатывает за ~16 мин
  (один холодный decode) вместо фактически бесконечного per-query rebuild. recall@10
  engine-keyword = cached-lexical = 56.0% на clean-50 (регрессии нет). Остаточный
  bottleneck — декод 2GB `.aci` (нужен mmap/lazy postings, см. EPIC-16 в
  `CORTEXDB_EPICS_5_17_PLAN.md`).
- **Embedding-rerank (внешняя bge-m3) — недостающая «ветка внешней модели» EPIC-04 —
  проведена в official-clean** (`run_official_clean_benchmark.py --embedding-rerank`):
  retrieve отдаёт широкий пул кандидатов (`--embedding-rerank-candidates`, def 50),
  затем `rerank_with_embeddings.py` сужает до `--top-k` по cosine. Это oracle-clean
  (только question text + doc body) и сохраняет clean-формат (проходит guard).
  В отличие от `WeightedScoreReranker`, это семантический сигнал — именно он
  исторически давал +recall.

**Измеренный lift (semantic-30, no-oracle, recall@10):**

| режим | recall@10 |
| --- | ---: |
| lexical top-10 | `3.33%` |
| **embedding-rerank top-10** | **`10.00%`** |
| wide pool top-50 (потолок) | `10.00%` |

Embedding-rerank (bge-m3, cosine) **утроил** semantic recall@10 (3.33→10.0) и достал
**потолок пула**: вытащил в top-10 каждый gold-док, присутствующий где-либо в top-50.
Это прямое подтверждение, что «внешняя модель» EPIC-04 даёт реальный lift, которого
не даёт `WeightedScoreReranker` (0/50 no-op). **Остаточный лимитер теперь — recall
кандидатов**: лексика находит лишь ~10% semantic gold в top-50, поэтому даже идеальный
rerank упирается в 10%. Следующий рычаг — dense/hybrid генерация кандидатов (EPIC-01,
нужны вектора), а не rerank.

**Официальный no-oracle замер (2026-06-10): balanced-50, gemma answer + gemma judge,
cached-lexical + embedding-rerank(100→10).** Артефакт:
`target/enterprise-rag-bench/official-clean/50/balanced50-epic01-hybrid/answer-gemma/judge-gemma/results.json`.

| метрика | значение |
| --- | ---: |
| **OVERALL (uniform 5/тип)** | **43.06** |
| OVERALL (proportional к 500) | ~30.95 |
| avg correctness / completeness | 48.0 / 51.96 |
| avg doc recall@10 | 57.08 |

По типам combined: info_not_found 100, miscellaneous 80, constrained 59, high_level 40,
intra 40, basic 36.6, completeness 35, conflicting 30, project_related 10,
**semantic 0** (recall 0 — лексический пул top-100 не содержит ни одного semantic
gold). Выводы: (1) пайплайн полностью no-oracle и даёт реальный overall; (2)
`info_not_found`/`high_level` честно работают через retrieve→answer без oracle-роутинга;
(3) **EPIC-01 bounded-hybrid (глубокий пул + rerank) НЕ восстанавливает semantic** —
gold лексически невидимы, нужна настоящая dense-генерация кандидатов (эмбеддинг корпуса,
~8.5ч на 511k, вне бюджета сессии). Главный лимитер overall = отсутствие dense candidate
generation для semantic (125/500 — крупнейшая категория). Примечание: n=5/тип шумно,
gemma-судья мягок; proportional ~31 ближе к официальной 500-вопросной математике.

---

## Тема 2. ContextPack как настоящий дифференциатор

### EPIC-05 — ContextPack v2: span-level token-budget packing `P0/L`
**Цель.** Паковать **релевантные спаны** документов под жёсткий токен-бюджет, а не
ведущие N символов.
**Почему.** Именно span-упаковка чинит project_related (факты ниже лида). Это и есть
обещанная ценность ContextPack — сделать её реальной и детерминированной.
**Работы.** Span-selection по якорям/эмбеддингам, детерминированный бюджетировщик,
дедуп перекрытий, стабильный порядок. **Крейты:** engine. **Exit.** При фикс-бюджете
покрытие gold-фактов выше, чем у whole-doc, при ≤ том же числе токенов.

**Текущий evidence.** Добавлен opt-in `ContextPackOptions.span_level_packing`.
Oversized cells can be reduced to deterministic query-relevant spans before
large-cell policy runs; selected payloads preserve headers/source metadata and
append `[context_pack_span=true line_start=... line_end=...]`. Каждый span несёт
`ContextSpanProvenance` with source cell id, byte offsets, line range and nested
SourceRef. Tests: `span_level_packing_beats_prefix_truncation_under_same_budget`,
`span_level_packing_preserves_citation_metadata`,
`span_packed_cells_export_structured_provenance`,
`context_pack_span_level_packing_selects_relevant_window_under_budget`,
`context_pack_span_level_packing_is_opt_in`. Документирован
`make context-pack-span-packing-check` и evidence fixture
`examples/eval/context_pack_span_packing.jsonl`. Вывод: engine deterministic
span packing закрыт; автоматическое embedding-based span selection остаётся
будущим quality upgrade, не blocker для текущего deterministic EPIC-05 surface.

### EPIC-06 — Провенанс и цитаты в ContextPack `P0/M`
**Цель.** Каждый включённый спан несёт `cell_id` + offset + source-метаданные.
**Почему.** Без провенанса невозможны верификация и грумленные цитаты — это фундамент
для EPIC-08/09 и для enterprise-доверия.
**Работы.** Span→cell привязка, сериализация провенанса в pack, API-поле `citations`.
**Крейты:** core, engine, server. **Exit.** Любой факт в паке трассируется до
исходной ячейки и смещения.

**Текущий evidence.** `ContextPackCell` содержит `citation`, structured
`source_ref`, optional `ContextSpanProvenance`, `explain` and
`access_decision`. JSON, prompt and Markdown exports include citation/source_ref
and provenance; server `ContextPackCellResponse` exposes the same fields through
typed API and OpenAPI snapshots. Citation extraction supports `citation=`,
`source=` and normalized structured SourceRef metadata (`doc_id`/`chunk_id`,
URL/page/row/json_path/confidence). Tests:
`context_pack_reports_missing_citations_when_required`,
`context_pack_uses_source_line_as_citation`,
`context_pack_accepts_source_ref_as_required_citation`,
`span_level_packing_preserves_citation_metadata`,
`snapshot_context_pack_response`,
`context_pack_prompt_export_includes_citation_and_conflict_instructions`. Вывод:
EPIC-06 закрыт для engine/server export; downstream UI rendering remains a
separate product layer.

### EPIC-07 — Conflict/freshness-aware packing `P1/M`
**Цель.** При конфликте ячеек показывать обе с сигналами свежести/авторитетности.
**Почему.** Сила в категории conflicting_info должна идти от движка, а не от промпта.
**Работы.** Детект конфликтов (одинаковая сущность, разные значения), recency/source
authority score, политика «current/FAQ/spec > старые заметки». **Крейты:** engine.
**Exit.** На конфликтных кейсах pack включает верный «current» источник без oracle.

**Текущий evidence.** ContextPack detects visible conflicts for cells sharing
`project` + `metric` with different `value`, preserves conflicting numeric
evidence instead of pruning it as redundant, and exports
`conflict_visibility_q16`/`visible_conflict_count`. Explain includes
`source_trust_bonus`, `source_trust_category`, `source_freshness_q16`,
`source_freshness_category` and per-component reasons; freshness is computed
relative to retrieved candidate timestamps without oracle labels. Tests:
`test_numeric_guard_coexistence`,
`conflict_visibility_reports_conflicting_project_metric_values`,
`conflict_visibility_counts_distinct_conflict_groups`,
`conflict_visibility_is_exported_in_json_prompt_and_markdown`,
`conflicting_values_explain_source_freshness_for_current_source`,
`context_pack_explain_reports_source_trust_category`. Вывод: conflict/freshness
signals are engine-native and exported; broader authority policy tables
(`current/FAQ/spec`) can still be strengthened after more real corpus evidence.

---

## Тема 3. Verification (сделать VERIFY FACT осмысленным)

### EPIC-08 — VERIFY FACT v2: семантическая верификация `P0/L`
**Цель.** Проверять утверждение на entailment против цитируемых ячеек, а не только
подстрокой; калиброванная уверенность.
**Почему.** Текущая «детерминированная» проверка сильна там, где легко (подстроки),
и слаба там, где важно (смысл). Это ядро продуктового обещания.
**Работы.** Entailment-проверка (NLI/LLM-judge с порогом), confidence-калибровка,
режимы strict/soft. **Крейты:** engine. **Exit.** На размеченном наборе
verify-precision/recall измеримы и побивают substring-baseline.

**Текущий evidence.** VERIFY FACT уже вышел за substring-only baseline:
`verify_fact_aql` поддерживает exact/semantic/numeric entailment,
semantic/numeric/natural-language/graph contradiction, source trust ordering,
temporal validity guards, missing citation guards and relation-graph evidence
enrichment. `VerificationReport` теперь содержит report-level
`confidence_q16`, рассчитанный из лучшего supporting/contradicting evidence с
учётом `source_trust_q16`, и это поле экспортируется через Markdown/audit text,
server JSON, OpenAPI, Rust/Python/TypeScript SDK. Tests:
`verify_fact_aql_reports_semantic_entailment_match_kind`,
`verify_fact_reports_contradicted_from_negated_sentence`,
`verify_fact_reports_numeric_mismatch_guard_as_contradiction`,
`verify_fact_aql_uses_relation_graph_contradiction`,
`verify_fact_aql_reports_calibrated_report_confidence`,
`snapshot_verification_report_response`. Вывод: deterministic semantic/numeric/
graph verification and calibrated report confidence are productized; EPIC-08
остаётся partial до NLI/LLM-judge path и размеченного precision/recall gate,
который должен доказать lift against substring baseline.

### EPIC-09 — Grounding-guard на этапе ответа `P1/M`
**Цель.** Перед возвратом ответа проверять, что его спаны подтверждены паком; помечать
неподтверждённое.
**Почему.** Прямо бьёт по добавлению лишних фактов (главная причина провала
project_related/«ворот»).
**Работы.** Сопоставление answer-спанов с pack-цитатами, флаг `unsupported`, опция
отбраковки. **Крейты:** engine, sdk. **Exit.** Доля неподтверждённых утверждений в
ответах измеримо падает; correctness-gate растёт.

**Текущий evidence.** `ContextPack::ground_answer` и SDK helper
`ContextPackResponse::ground_answer*` сопоставляют answer spans с pack cells,
выдают `AnswerGroundingReport` с `support_q16`, `answer_supported`,
`rejected`, supported/unsupported span counts, missing terms, supporting
`cell_id` and citations. Options include `require_citations` and
`reject_unsupported`; server LLM/test-double path exposes typed grounding report
and snapshots cover the response shape. Tests:
`context_pack_grounding_accepts_answer_supported_by_pack`,
`context_pack_grounding_flags_unsupported_answer_span`,
`context_pack_grounding_can_require_citations`,
`grounded answer helper builds context verify and citations`,
`snapshot_context_trace_response`, `llm_inference_tests::*`. Вывод:
grounding primitive and API/SDK surface are present; EPIC-09 остаётся partial
until the guard is enforced as the default policy across every answer endpoint
and benchmark evidence shows fewer unsupported claims / higher correctness gate.

---

## Тема 4. Permission-aware retrieval (защитимый ров)

### EPIC-10 — Cell-level ACL как предикат индекса (zero-leak) `P0/L`
**Цель.** ACL применяется **в пути ретривала**, а не пост-фильтром; гарантия отсутствия
утечки неавторизованных ячеек.
**Почему.** Это самая защитимая фича против vector DB. Должна быть доказуемой.
**Работы.** ACL-предикат в bitmap-программе AQL, фильтрация до top-k, property-тест
«ни одна неавторизованная ячейка не попала в выдачу/пак». **Крейты:** aql, engine,
storage. **Exit.** Fuzz/property-набор не находит утечки; latency-оверхед ограничен.

**Текущий evidence.** ACL is enforced before candidate/top-k selection across
AQL bitmap retrieval, keyword search, vector search, hybrid search and
ContextPack packing. AQL provider builds an `agent_allowed` bitmap from
`AgentView.readable_scopes`; explain exposes `PushAgentAllowed` and
`agent_allowed` counts. Search tests prove stronger private matches cannot
displace weaker readable cells in snapshot or persisted indexes, including
hybrid mode. ContextPack broad queries exclude unreadable scopes before and
after checkpoint/compact and explicit forbidden scope queries fail closed before
packing. Tests: `generated_where_not_and_or_queries_do_not_bypass_scope_policy`,
`database_keyword_search_applies_acl_before_topk_snapshot`,
`database_keyword_search_applies_acl_before_topk_persisted`,
`database_vector_search_applies_acl_before_topk_snapshot`,
`database_vector_search_applies_acl_before_topk_persisted`,
`database_hybrid_search_applies_acl_before_topk_snapshot`,
`database_hybrid_search_applies_acl_before_topk_persisted`,
`context_pack_acl_is_applied_before_candidate_limit`,
`context_pack_broad_query_excludes_forbidden_scope_before_and_after_persistence`.
Вывод: zero-leak retrieval path is covered for the main engine modes; EPIC-10
остаётся partial only on published latency-overhead evidence for the ACL
predicate at large corpus scale.

### EPIC-11 — Мультитенантная изоляция и per-tenant realms `P1/M`
**Цель.** Жёсткая изоляция данных/индексов между тенантами + корректность под нагрузкой.
**Почему.** Server уже имеет realms — довести до доказуемой изоляции и предсказуемой
производительности.
**Работы.** Изоляционные тесты (cross-tenant запрет), per-realm ресурсные лимиты,
смоук под параллельной нагрузкой. **Крейты:** server, engine. **Exit.** Cross-tenant
доступ невозможен в тестах; p99 не деградирует от соседнего тенанта.

**Текущий evidence.** Server uses per-tenant realm paths under
`<root>/realms/<tenant>` and validates tenant identifiers before realm access.
HTTP security tests prove alpha/beta tenant cell data stays isolated, parallel
tenant requests create separate realms without shared state, and auth policy
tenant allowlists prevent denied tenant routes from creating a realm at all.
Policy-store quota tests also isolate principal limits. Tests:
`tenant_realms_isolate_cell_data_over_http`,
`parallel_tenant_realms_do_not_share_state`,
`auth_policy_store_tenants_restrict_database_realms`,
`policy_store_principal_quota_is_isolated_per_principal`,
`policy_store_principal_body_quota_limits_uploaded_bytes_and_reports_metrics`,
`policy_store_principal_queue_quota_reports_actor_queue_metrics`. Вывод:
cross-tenant isolation and per-principal resource controls are present; EPIC-11
остаётся partial until p99/noisy-neighbor degradation is measured in a longer
multi-tenant load gate.

### EPIC-12 — RBAC/policy-модель, привязанная к выдаче `P1/M`
**Цель.** Роли/скоупы/решения доступа логируются и связаны с тем, что попало в pack.
**Почему.** Enterprise-аудит: «почему этот пользователь увидел этот факт».
**Работы.** Policy store, decision audit-log, привязка решения к `cell_id` в паке.
**Крейты:** server, engine. **Exit.** Для любого пака можно показать access-decision
trail по каждой ячейке.

**Текущий evidence.** Policy store backs admin-managed principals, capabilities,
tenant allowlists, agent scopes and context budgets. ContextPack cells carry
structured `access_decision` with `cell_id`, decision outcome, policy, reason,
scope, `scope_id` and `agent_id`; server typed responses/OpenAPI/SDK expose the
same trail. Audit sink writes chained JSONL without request body/query/secret
leakage, and auth-policy tests cover data/admin capability separation plus
agent-scope enforcement. Tests: `context_pack_records_access_decision_trail_per_cell`,
`snapshot_context_pack_response`, `auth_policy_store_capabilities_restrict_data_routes`,
`auth_policy_store_agent_scope_is_enforced`, `admin_can_grant_and_revoke_agent_scope`,
`audit_sink_writes_jsonl_without_body_or_query`,
`audit_log_file_records_route_metadata_without_query`,
`audit_log_file_records_policy_store_principal_without_token`. Вывод:
per-packed-cell access trail and server policy/audit primitives are productized;
EPIC-12 остаётся partial until audit records directly correlate request ids,
policy principal and every returned ContextPack cell in one operator-facing
review artifact.

---

## Тема 5. Storage/engine robustness (оправдать или де-рискнуть свой storage)

### EPIC-13 — Заморозка формата + миграции (forward/backward compat) `P1/M`
**Цель.** Стабильные `.acs/.acb/.aci/.acv/.ach` + инструмент миграции версий.
**Почему.** Без compat собственный storage — обуза при каждом релизе.
**Работы.** Версионирование заголовков, читатель старых версий, `cortexdb migrate`,
golden-fixture набор. **Крейты:** storage, cli. **Exit.** Старые сегменты читаются
новым движком; миграция round-trip без потери данных.

**Текущий evidence.** Storage format inventory is machine-readable through
`storage_format_specs()` and public `/v1/compatibility`: ACLOGv0, ACS1, ACB0,
ACI2, ACV0, ACH0, ACM0 are listed as current markers; ACI0/ACI1 are exposed as
read-only compatible legacy lexical formats. Format tests prove written files
match the inventory, legacy ACI0/ACI1 readers remain compatible, manifests
ignore forward-compatible trailing fields with a valid CRC, and invalid storage
files fail closed. Migration gates passed locally on 2026-06-10:
`make migration-policy-check`, `make migration-compatibility-check`;
artifacts:
`target/migration-historical-restore/report.json` (`status=passed`, 2 cells
verified from `v0.1.0-core-alpha.5`) and
`target/migration-upgrade-matrix-v2/report.json` (`status=passed`, old DB opens,
current binary writes a new cell, post-upgrade backup/restore verifies 3 cells).
Storage format change-note enforcement now has a machine-readable registry
`fixtures/migration/storage_format_change_notes_v1.json`, checked by
`make storage-format-change-note-check` and wired into stable CI plus
`alpha-check`. The checker requires every current and legacy frozen marker
(`ACLOGv0`, `ACS1`, `ACB0`, `ACI2`, `ACI0`, `ACI1`, `ACV0`, `ACH0`, `ACM0`) to
declare migration-note policy, required docs, required gates and a release
fixture path before format changes can merge.
Tests: `storage_format_inventory_lists_current_core_formats`,
`written_storage_files_match_current_format_inventory`,
`aci0_lexical_index_remains_readable_without_doc_lengths`,
`aci1_lexical_index_remains_readable_without_term_frequencies`,
`manifest_forward_compatibility`,
`compatibility_summary_exposes_all_storage_format_markers`. Вывод: format
inventory, legacy lexical compatibility, offline migration/restore gates and
pre-merge storage change-note enforcement are productized; EPIC-13 остаётся
partial only on actual future-release fixture breadth as new storage format
changes appear.

### EPIC-14 — Crash-consistency: WAL replay/checkpoint под fault-injection `P0/L`
**Цель.** Доказуемая краш-консистентность (ACLOG WAL + checkpoint).
**Почему.** Есть chaos/crash-fault scaffolding — превратить в гарантию, а не смоук.
**Работы.** Инъекция отказов на каждой границе fsync, property «после рестарта
видимость = последний зафиксированный seq», torn-write детект. **Крейты:** storage,
engine. **Exit.** N=1000 рандом-краш сценариев восстанавливаются без потери/порчи.

**Текущий evidence.** `make crash-fault-check` passed locally and wrote
`target/crash-fault/report.json` with `status=ok`. The gate runs crash matrix,
restart matrix, corruption matrix, repair tests and
`crash_consistency_fault_injection` with 1000 deterministic scenarios for the
invariant "restart visible state equals last committed seq or fails closed for
published torn files". The same report proves CLI repair removes orphan temp
files, truncates a partial WAL tail to safe offset 138, preserves one WAL
record, and validates a readable payload after repair. Tests:
`wal_checkpoint_fault_injection_preserves_last_committed_state`,
`published_torn_checkpoint_files_fail_closed_or_validate_bad`,
`interrupted_checkpoint_orphan_bundle_is_ignored`,
`interrupted_compact_without_manifest_switch_keeps_old_snapshot`,
`best_effort_recovery_stops_at_corrupt_payload`,
`repair_best_effort_removes_orphans_and_truncates_wal_tail`. Вывод: modeled
WAL/checkpoint/compact crash consistency is strongly covered; EPIC-14 остаётся
partial only below modeled file-publication boundaries, where byte-level fsync
fault injection is still listed as out of scope in current evidence docs.

### EPIC-15 — Компакция/GC и контроль write-amplification `P1/M`
**Цель.** Ограниченные space/latency под длительной записью (storage-soak есть).
**Почему.** Без bounded compaction большой корпус деградирует по месту и хвостам.
**Работы.** Политика компакции, метрика write-amp, бэкпрешер, soak-кампания.
**Крейты:** storage, engine. **Exit.** 72h soak: место и p99 в заданных границах.

**Текущий evidence.** `storage_stats()` exposes `space_amplification_q16`,
`write_amplification_q16` and `compaction_pressure_q16`; tests verify these
move across checkpoint/compact/GC. `make storage-soak-check` passed locally and
wrote `target/storage-soak/report.json` with `status=passed`, 3 cycles, 15 cells
written, per-cycle backup/restore verification, partial WAL repair validation,
and GC removing retired segment files every cycle. Retained v1 history already
shows 24-hour evidence in `docs/STORAGE_SOAK.md`:
`twenty_four_hour_evidence.met=true`, `run_count=981`,
`total_cycles=19584`, `total_cells_written=979016`,
`total_duration_seconds=86476`. Tests/gates:
`storage_stats_tracks_compaction_pressure_and_amplification`,
`gc_retired_segments_removes_files_and_preserves_live_data`,
`make storage-soak-check`, `make storage-soak-24h-evidence-check`. Вывод:
bounded-growth signals and v1 24h evidence exist; EPIC-15 остаётся partial
until the v2 72-hour gate (`make storage-soak-72h-evidence-check`) passes with
bounded amplification thresholds.

### EPIC-16 — Производительность: SLO ретривала и ингеста на 500k+ `P1/L`
**Цель.** Latency-SLO для retrieve/context эндпоинтов, throughput ингеста, RSS.
**Почему.** EnterpriseRAG-корпус ~500k доков — путь должен держать масштаб с
предсказуемыми хвостами.
**Работы.** Бенч ингеста батчами, профиль памяти `.aci`/HNSW, p50/p95/p99 на запрос,
кэш индексов. **Крейты:** engine, storage, server. **Exit.** Опубликованный
single-node SLO выдерживается на полном корпусе.

**Текущий evidence.** `make single-node-performance-check` passed locally on
2026-06-10 and wrote `target/single-node-performance/report.json`
(`ok=true`, `duration_ms=773.855`). Embedded strict profile: ingest
188,413.689 cells/sec, `put_single` p95 0.151 ms, `keyword_search` p95
7.341 ms, `context_pack` p95 4.144 ms, `verify_fact` p95 33.682 ms.
Balanced profile: ingest 364,500.827 cells/sec, `put_single` p95 0.297 ms,
`keyword_search` p95 2.845 ms, `context_pack` p95 4.009 ms,
`verify_fact` p95 20.916 ms. Full-corpus official-clean retrieval-only
evidence remains the larger scale path: 511,958 documents indexed,
16,262.927 docs/sec ingest, 50 retrieval questions, 0.229 questions/sec,
peak RSS 20,210,786,304 bytes. Вывод: local embedded SLO smoke is green and
the 500k+ path is proven runnable; EPIC-16 остаётся partial until checkpoint
publication, cached lexical loading and per-question retrieval latency are
optimized enough to make the full-corpus path a strong SLO rather than a
known-bottleneck evidence run.

---

## Тема 6. Честная оценка (прямо из критики оверфита)

### EPIC-17 — Engine-driven bench-режим без oracle-метаданных `P0/M`
**Цель.** Прогон EnterpriseRAG/LongMemEval, где **движок** делает retrieval+packing,
без чтения `question_type`/`source_types`, с held-out split.
**Почему.** Операционализирует «без подгонки». Показывает реальную силу движка и
честное число для лидерборда.
**Работы.** Bench-режим, маскирующий gold-метки; dev/locked split (кандидат —
`extra_questions.jsonl`); сравнение с текущим oracle-пайплайном. **Крейты:** engine,
cli, bench-скрипты. **Exit.** Воспроизводимое число «engine-only, no-oracle, held-out».

**Текущий evidence.** Official-clean runner записывает `split_name`,
`questions_file` и `inference_oracle_policy` в run report. Target
`make enterprise-rag-bench-official-clean-heldout-smoke-check` проверяет
held-out `extra_questions.jsonl`, чистит oracle-поля и валидирует clean artifact
через `scripts/enterprise_rag_bench/official_clean_gate.py`.

Held-out retrieval evidence:
`target/enterprise-rag-bench/official-clean/100/epic17-heldout-retrieval/official_clean_gate_report.json`
со статусом `passed`; clean questions: 100; clean retrieval rows: 100; split:
`heldout`; questions file: `extra_questions.jsonl`; oracle fields stripped before
inference. Retrieval reused the EPIC-16 full-corpus DB and recorded
`retrieval.throughput_questions_per_sec=0.312`, `peak_rss_bytes=11168313344`.

Held-out answer/judge score:
`target/enterprise-rag-bench/official-clean/100/epic17-heldout-retrieval/answer-deepseek/judge-deepseek/results.json`.
Provider/model: `deepseek-v4-flash` for answer and judge. Answer generation used
543,131 prompt tokens, 1,643 completion tokens, 544,774 total tokens. Judge used
30,253 prompt tokens, 4,535 completion tokens, 34,788 total tokens. Result:
overall=6.0, answer correctness=6.0%, completeness=7.63%, document recall=33.0%,
invalid extra docs=9.67. Вывод: EPIC-17 теперь имеет воспроизводимый
`engine-only, no-oracle, heldout` baseline, но качество heldout показывает, что
EPIC-18/следующая retrieval-regression работа должна поднимать recall и снижать
шум без oracle-селекторов.

### EPIC-18 — Регресс-харнес retrieval-качества в CI `P1/M`
**Цель.** recall@k / ndcg на публичных корпусах в CI, ловит регресс качества.
**Почему.** Сейчас качество не защищено гейтом — легко тихо ухудшить.
**Работы.** Фикстуры, метрики, порог-гейт, история тренда. **Крейты:** engine, cli.
**Exit.** PR, роняющий recall, краснеет в CI автоматически.

**Текущий evidence.** Добавлен общий gate
`scripts/enterprise_rag_bench/retrieval_quality_gate.py`. Lightweight fixture
target `make enterprise-rag-bench-retrieval-quality-fixture-check` проверяет
формулы recall/full-recall/hit/MRR/nDCG/invalid-extra-docs на чистом
`fixtures/enterprise_rag_bench/retrieval_quality_gate/*.jsonl` наборе и включён
в stable GitHub Actions `rust.yml`, чтобы PR мог краснеть при поломке качества
retrieval gate без скачивания внешнего 500k EnterpriseRAG corpus.

Full held-out target
`make enterprise-rag-bench-official-clean-heldout-retrieval-quality-check`
читает gold только на стадии evaluation, валидирует clean retrieval artifact и
пишет:
`target/enterprise-rag-bench/official-clean/100/epic17-heldout-retrieval/retrieval_quality_gate_report.json`.
Текущий baseline: status=`passed`, average recall=33.0, hit/full recall=33/100,
MRR=0.23, nDCG=0.25, invalid extra docs=9.67. Порог пока фиксирует низкий
official-clean heldout baseline; следующий шаг EPIC-18 — поднять held-out
baseline и добавить отдельный release workflow/command, который прогоняет full
held-out artifact там, где доступен внешний benchmark checkout.

---

## Тема 7. DX / продуктовая поверхность

### EPIC-19 — SDK-примитив «answer with grounded context» `P1/M`
**Цель.** Один вызов: вопрос → retrieve→rerank→pack→verify→цитаты, в Python/TS SDK.
**Почему.** Сейчас цикл собирается вручную из кусков. Один примитив = реальная
agent-native ценность для пользователя.
**Работы.** Высокоуровневый клиентский метод, типобезопасные ответы с цитатами,
примеры. **Крейты:** sdk, server. **Exit.** Демо-агент отвечает с грумленными
цитатами одним вызовом SDK.

**Текущий evidence.** Добавлен grounded-answer primitive для Rust, Python и
TypeScript SDK. Rust экспортирует `GroundedAnswerRequest`,
`GroundedAnswerResponse` и `CortexDbClient::answer_with_grounded_context`.
Python экспортирует `CortexDBClient.answer_with_grounded_context(...)` и typed
`GroundedAnswerResponse`. TypeScript экспортирует `answerWithGroundedContext`,
`groundAnswer`, `buildGroundedAnswerResponse` и синхронизированные ESM/CJS/DTS
types. Primitive строит `RETRIEVE CONTEXT`, отдаёт `ContextPack` во внешний
`answerer`, затем опционально строит `VERIFY FACT` и возвращает grounding spans,
citations, `used_context_cell_ids` и verification report. Targeted evidence:
`cargo test -p cortex-sdk`, `python3 sdk/python/test_cortexdb_client.py` и
`node --test sdk/typescript/test.js` проходят. Документация обновлена в
`docs/SDK_QUICKSTART.md`, `sdk/python/README.md`, `sdk/typescript/README.md`.
Вывод: EPIC-19 закрыт для локальных SDK; registry publication остаётся отдельной
release-операцией.

### EPIC-20 — Observability/explainability пайплайна retrieve→pack→verify `P1/M`
**Цель.** Трейсинг по стадиям, per-stage метрики, объяснение «почему эти ячейки».
**Почему.** Превращает чёрный ящик в отлаживаемый продукт; критично для доверия и
для отладки качества.
**Работы.** Span-трейсы стадий, explain-API («score-вклад каждой ячейки»),
дашборд-панели. **Крейты:** engine, server. **Exit.** Для любого ответа видно путь
и вклад каждого источника.

**Текущий evidence.** Добавлен engine-level trace model
`ContextPipelineTrace`/`ContextPipelineStageTrace`/`ContextPipelineCellTrace` с
cell-level score components, matched terms, citation/provenance flags,
access-decision summary и verification summary. Server получил endpoint
`POST /v1/context/trace?scope=<scope>`, который принимает raw retrieve AQL или
JSON `{retrieve_aql, verify_aql}` и возвращает typed `{context, verification,
trace}`. OpenAPI и `scripts/check_openapi_contract.py` обновлены под новый
contract. Tests: `cargo test -p cortex-engine
pipeline_trace_summarizes_pack_and_verification`, `cargo test -p cortex-server
context_trace`. Вывод: EPIC-20 закрыт как first-class explain/trace API для
retrieve→pack→verify; dashboard-визуализация остаётся будущим UI-слоем поверх
этого контракта.

---

## Приоритеты и последовательность

**P0 (фундамент качества и рва):** 01, 02, 05, 06, 08, 10, 14, 17.
**P1 (усиление и доказательство):** 03, 04, 07, 09, 11, 12, 13, 15, 16, 18, 19, 20.

Рекомендуемый порядок волнами:
1. **Волна 1 (движок берёт качество на себя):** 01 → 05 → 06 → 02. После неё внешние
   эмбеддинги перестают быть единственным источником lift.
2. **Волна 2 (ров):** 10 → 08 → 09 → 17. Permission-aware + verification +
   честный engine-only прогон — это то, что отличает CortexDB от vector DB.
3. **Волна 3 (надёжность/масштаб):** 14 → 16 → 15 → 13.
4. **Волна 4 (обобщение и DX):** 03 → 04 → 07 → 18 → 19 → 20 → 11 → 12.

Критерий успеха всей программы: **engine-only, no-oracle, held-out** число на
EnterpriseRAG (EPIC-17) растёт за счёт EPIC-01/05/08, а не за счёт новых селекторов.
Это и есть доказательство, что улучшается движок, а не подгонка под тест.
