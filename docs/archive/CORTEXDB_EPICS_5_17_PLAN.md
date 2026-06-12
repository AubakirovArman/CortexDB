# CortexDB — детальный план реализации эпиков EPIC-05…EPIC-17

Версия: 2026-06-10. Дополнение к [CORTEXDB_IMPROVEMENT_EPICS.md](CORTEXDB_IMPROVEMENT_EPICS.md).
Формат каждого эпика: **Цель → Предусловия → Шаги → Как проверить → Exit → Риск**.

Общие правила:
- Каждый новый публичный примитив сопровождается unit-тестом в том же модуле и
  (где есть качество) regression/CI-gate по образцу EPIC-01..04.
- Качество доказываем на **held-out** (EPIC-17), а не на видимых 500.
- Любая фича, влияющая на выдачу, — через `question`-текст и метаданные ячеек,
  никогда через gold-метки бенча.
- Зависимости: 05→06→07 (ContextPack), 06→08→09 (verification), 06→12 (RBAC),
  10 — фундамент для 11/12, 13/14/15→16, всё стекается в 17.

---

## EPIC-05 — ContextPack v2: span-level token-budget packing `P0/L`

**Цель.** Паковать релевантные **спаны** документов под жёсткий токен-бюджет, а не
ведущие N символов/целую ячейку. Это продуктовая реализация того, что в bench-Python
делает `context_windows.py`/`evidence_digest.py`.

**Предусловия.** EPIC-03 (`analyze_search_query` — якоря/expansions) — для anchor-aware
окон. Найти текущий построитель ContextPack: `cortex-engine` (context-модуль), потребитель
— `crates/cortex-server/src/context.rs`; токен-оценщик — `scripts/context_pack_token_estimator_check.py`.

**Шаги.**
1. Локализовать структуру `ContextPack` и её item (ячейка → включённый текст + бюджет).
   Зафиксировать текущее поведение тестом-характеристикой до изменений.
2. Добавить `span_extractor`: на вход (query, anchors, cell content) → список
   спанов (предложение/абзац-окна вокруг совпадений term/anchor) со score
   релевантности. Anchors берём из `analyze_search_query`.
3. Детерминированный бюджетировщик: жадный отбор спанов по score до `budget_tokens`;
   дедуп перекрывающихся спанов; стабильный порядок (score desc, then offset asc).
4. Параметры в конфиге pack: `max_spans_per_cell`, `span_window_tokens`,
   `budget_tokens`. Дефолты — консервативные.
5. Прокинуть режим `packing=span` через engine API и server (рядом с whole-cell),
   whole-cell оставить как fallback/legacy.

**Как проверить.**
- Unit: `span_selection_is_deterministic`, `packing_respects_token_budget`,
  `span_pack_covers_more_gold_facts_than_whole_doc` на committed-фикстуре.
- Gate: расширить `scripts/context_pack_quality_check.py` метрикой
  «gold-fact coverage при фикс-бюджете»; добавить `make context-pack-span-quality-check`.
- Ручная проверка на bench: в answer-пути сравнить span-pack vs leading при равном
  `max-chars/budget`.

**Exit.** При фиксированном токен-бюджете покрытие gold-фактов span-режимом
строго выше whole-doc на фикстуре; бюджет никогда не превышен (property-тест).

**Риск.** Низкий-средний: чистая добавка режима; whole-cell не трогаем.

---

## EPIC-06 — Провенанс и цитаты в ContextPack `P0/M`

**Цель.** Каждый включённый спан несёт `cell_id` + byte-offset (start,end) +
source-метаданные. Фундамент для верификации (08/09) и RBAC-аудита (12).

**Предусловия.** EPIC-05 (есть спаны, которым приписываем провенанс).

**Шаги.**
1. Расширить item ContextPack полями `provenance { cell_id, offset_start,
   offset_end, scope, source_type, commit_seq }`.
2. Сохранять offset при извлечении спана (span_extractor возвращает диапазоны, не
   только текст).
3. Прокинуть в server-ответ поле `citations: [{cell_id, offset, source}]`
   (`crates/cortex-server/src/responses.rs`) и в SDK-типы
   (`crates/cortex-sdk/src/types.rs`, py/ts SDK).
4. Гарантировать стабильность mapping спан↔ячейка при реранке/сортировке.

**Как проверить.**
- Unit: `every_packed_span_resolves_to_cell_and_offset`,
  `provenance_survives_rerank_reorder`.
- Server snapshot-тест: ответ содержит `citations`, offset попадает в тело ячейки.
- Property: для случайного pack каждый спан → `content[offset_start..offset_end]`
  совпадает с включённым текстом.

**Exit.** Любой факт в паке трассируется до `cell_id` + offset; server/SDK отдают
citations.

**Риск.** Низкий.

---

## EPIC-07 — Conflict/freshness-aware packing `P1/M`

**Цель.** При конфликте ячеек (одна сущность, разные значения) включать обе с
сигналом свежести/авторитетности; «current/FAQ/spec» выше «старых заметок».

**Предусловия.** EPIC-03 (anchors), EPIC-06 (provenance/commit_seq).

**Шаги.**
1. Детектор конфликтов: группировать кандидатные спаны по якорю (сущность+атрибут);
   если значения расходятся — пометить группу `conflict`.
2. Сигнал авторитетности: из метаданных ячейки — `commit_seq`/timestamp (свежесть) +
   source authority (таблица приоритетов: requirements/FAQ/current > meeting/notes/old).
3. Политика упаковки: при конфликте включить топ-2 (current + предыдущий), current
   первым, пометить `superseded=true` у старого.
4. Вынести политику в конфиг (веса свежести/авторитета).

**Как проверить.**
- Фикстура с конфликтующими ячейками (новая current + старая) → pack ставит current
  первым и метит superseded.
- Unit: `conflict_detected_for_same_anchor_different_value`,
  `current_source_ranked_above_superseded`.
- Bench: проверить, что категория `conflicting_info` в clean-режиме не падает.

**Exit.** На conflict-фикстуре выбирается current/авторитетное значение без oracle,
старое помечено superseded.

**Риск.** Средний: неверная эвристика свежести может ухудшить conflicting_info —
держать за gate.

---

## EPIC-08 — VERIFY FACT v2: семантическая верификация `P0/L`

**Цель.** Проверять утверждение на entailment против цитируемых ячеек, не только
подстрокой; калиброванная уверенность.

**Предусловия.** EPIC-06 (цитаты — что верифицировать против чего).

**Шаги.**
1. Найти текущую `VERIFY FACT` в engine; зафиксировать substring-поведение тестом.
2. Ввести трейт `FactVerifier { fn verify(claim, &[CitedSpan]) -> Verdict }` с
   `Verdict { supported: bool, confidence_q16 }`.
3. Реализации: (a) `LexicalOverlapVerifier` — детерминированный, порог по
   overlap/anchor-match (быстрый дефолт); (b) опциональный `NliVerifier` — внешний
   NLI/LLM-judge за флагом (как embedding-эндпоинт, ключ из env).
4. Калибровка: маппинг score→`confidence_q16`; режимы strict/soft по порогу.

**Как проверить.**
- Размеченная фикстура `verify_fact_labeled.jsonl` (claim, cited, label).
- Метрики precision/recall/calibration vs substring-baseline; gate
  `make verify-fact-v2-quality-check`.
- Unit: `lexical_verifier_rejects_unsupported_numeric_change`,
  `confidence_is_monotonic_in_overlap`.

**Exit.** Verify precision/recall измеримы и побивают substring-baseline на фикстуре;
confidence калиброван (reliability bins).

**Риск.** Средний: NLI-бэкенд недетерминирован — дефолт держать на лексическом,
NLI за явным флагом и вне CI-gate.

---

## EPIC-09 — Grounding-guard на этапе ответа `P1/M`

**Цель.** Перед возвратом ответа проверять, что его утверждения подтверждены паком;
помечать/отбраковывать неподтверждённое. Бьёт по добавлению лишних фактов (главная
причина провала project_related «ворот»).

**Предусловия.** EPIC-06 (цитаты), EPIC-08 (verifier).

**Шаги.**
1. Сегментация ответа на атомарные claim'ы (по предложениям/числовым фактам).
2. Для каждого claim — `FactVerifier.verify(claim, pack_citations)`; собрать
   `unsupported_claims`.
3. Политика: `report` (пометить), `drop` (выкинуть неподтверждённое), `abstain`
   (если доля неподтверждённого выше порога).
4. Реализовать как engine/SDK-примитив И как стадию в bench answer-пути
   (`scripts/enterprise_rag_bench/` — после генерации, до судьи).

**Как проверить.**
- Фикстура ответов с заведомо неподтверждёнными вставками → guard их ловит.
- Bench: на clean-50 доля unsupported claims падает; correctness-gate не падает.
- Unit: `guard_flags_unsupported_numeric_threshold`,
  `guard_passes_fully_grounded_answer`.

**Exit.** Измеримое снижение unsupported-claims на фикстуре/clean-50 без падения
correctness.

**Риск.** Средний: агрессивный drop может срезать верные факты — начинать с `report`.

---

## EPIC-10 — Cell-level ACL как предикат индекса (zero-leak) `P0/L`

**Цель.** ACL применяется **в пути ретривала** (предикат битмапа AQL), а не
пост-фильтром; доказуемое отсутствие утечки неавторизованных ячеек. Самый защитимый
ров против vector DB.

**Предусловия.** Текущее: ACL — пост-фильтр (`view.can_read_scope(scope_id(..))` при
итерации snapshot в `crates/cortex-engine/src/search/database.rs`). Уже есть тесты
`database_hybrid_search_applies_acl_before_topk_snapshot/persisted` — частичная база.

**Шаги.**
1. Перенести scope/ACL-предикат в `cortex-aql` binder → bitmap-байткод: пересечение
   кандидатного битмапа с разрешёнными scope ДО top-k.
2. В persisted-пути (`search_persisted_query`) применять ACL-битмап перед чтением
   top-k из `.aci/.acv` — чтобы неавторизованные не доходили до ранжирования.
3. Убедиться, что rerank (EPIC-04) и ANN (EPIC-02) тоже фильтруются до rerank
   (в server уже «scope/ACL до rerank» — закрепить в engine).
4. Унифицировать snapshot и persisted ACL-путь.

**Как проверить.**
- Property/fuzz: `acl_never_leaks_unauthorized_cell` — рандомные ACL-конфиги +
  рандомные запросы, ни одна неавторизованная ячейка в результатах/паке.
- Latency-бенч: оверхед ACL-предиката ограничен (<X% к baseline).
- Расширить существующие `applies_acl_before_topk_*` на persisted + rerank + ANN.

**Exit.** Fuzz-набор (≥1000 конфигов) находит ноль утечек; ACL-оверхед в бюджете.

**Риск.** Высокий: трогает горячий путь и AQL-байткод. Делать за фича-флагом с
fallback на пост-фильтр, сверять результаты обоих путей в тесте.

---

## EPIC-11 — Мультитенантная изоляция и per-tenant realms `P1/M`

**Цель.** Доказуемая изоляция данных/индексов между тенантами + предсказуемая
производительность под нагрузкой.

**Предусловия.** EPIC-10 (ACL-фундамент). Server уже имеет realms/`DatabaseActor`
(`crates/cortex-server/`).

**Шаги.**
1. Изоляционные тесты: запрос тенанта A не видит ячейки/индексы тенанта B (данные,
   lexical, vector, context).
2. Per-realm ресурсные лимиты (память/кандидаты/бюджет) и их применение в actor.
3. Смоук под параллельной нагрузкой: N тенантов одновременно, проверка p99 соседа.

**Как проверить.**
- `cross_tenant_search_returns_nothing_from_other_realm` (data, lexical, vector).
- Нагрузочный смоук: p99 тенанта не деградирует от активности соседа (порог).
- Reuse `scripts/tenant_recovery_check.py`/load-смоуки.

**Exit.** Cross-tenant доступ невозможен в тестах; p99 устойчив к шуму соседа.

**Риск.** Средний.

---

## EPIC-12 — RBAC/policy-модель, привязанная к выдаче `P1/M`

**Цель.** Роли/скоупы/решения доступа логируются и связаны с тем, что попало в pack
(«почему этот пользователь увидел этот факт»).

**Предусловия.** EPIC-06 (provenance: cell_id в паке), EPIC-10 (ACL-решения).
Есть `scripts/enterprise_rbac_gate_check.py` + rbac-policy-store отчёты.

**Шаги.**
1. Policy store: роли → scope-гранты; загрузка/валидация политики.
2. Decision audit-log: на каждый доступ — `{actor, cell_id, scope, decision, reason}`.
3. Привязка решения к `cell_id` в паке (через provenance EPIC-06) — для любого пака
   собрать access-trail по каждой ячейке.

**Как проверить.**
- `pack_has_access_decision_trail_per_cell`.
- Изменение роли меняет состав пака предсказуемо (тест политики).
- Reuse rbac-policy-store gate.

**Exit.** Для любого пака воспроизводится access-decision trail по каждой ячейке.

**Риск.** Низкий-средний.

---

## EPIC-13 — Заморозка формата + миграции `P1/M`

**Цель.** Стабильные `.acs/.acb/.aci/.acv/.ach` + инструмент миграции версий, чтобы
свой storage не был обузой при релизах.

**Предусловия.** Есть `scripts/storage_format_freeze_check.py`,
`check_migration_compatibility.py`, фикстуры `fixtures/migration/`.

**Шаги.**
1. Версионные заголовки во всех форматах сегментов (magic+version).
2. Читатель старых версий (back-compat) в `cortex-storage`.
3. CLI `cortexdb migrate <path>` (`crates/cortex-cli`) — апгрейд сегментов.
4. Golden-фикстуры старых версий под `fixtures/migration/`.

**Как проверить.**
- `old_segment_read_by_new_engine`, `migrate_round_trips_without_data_loss`.
- Gate `make storage-compat-check` / `check-migration-compatibility` на golden-наборе.

**Exit.** Старые сегменты читаются новым движком; миграция round-trip без потерь.

**Риск.** Средний: формат-байткод чувствителен; golden-фикстуры обязательны.

---

## EPIC-14 — Crash-consistency: WAL replay/checkpoint под fault-injection `P0/L`

**Цель.** Доказуемая краш-консистентность (ACLOG WAL + checkpoint), а не смоук.

**Предусловия.** Есть `scripts/crash_fault_check.sh`, `chaos_restart_check.py`,
`replication_failure_injection.rs`.

**Шаги.**
1. Инъекция отказа на каждой границе fsync (WAL append, checkpoint, manifest swap).
2. Property: после рестарта видимость = последний зафиксированный `commit_seq`;
   ни одной частично применённой транзакции.
3. Torn-write детект (checksum) на сегментах/WAL.
4. Рандомизированный сценарный прогон (seed-based, N=1000).

**Как проверить.**
- `recovers_to_last_committed_seq_after_crash_at_each_boundary`.
- `torn_write_is_detected_and_rejected`.
- Кампания `make crash-fault-check` с N рандом-сценариев, ноль потерь/порчи.

**Exit.** ≥1000 рандом-краш сценариев восстанавливаются чисто.

**Риск.** Высокий по охвату, но изолирован в storage; детерминируется seed'ом.

---

## EPIC-15 — Компакция/GC и контроль write-amplification `P1/M`

**Цель.** Ограниченные space/latency под длительной записью.

**Предусловия.** Есть `scripts/storage_soak_*`.

**Шаги.**
1. Политика компакции (триггеры по числу/размеру сегментов), GC старых версий MVCC.
2. Метрика write-amplification; экспонировать в отчёте.
3. Бэкпрешер при отставании компакции.
4. 72h soak-кампания с фикс-бюджетом места и p99.

**Как проверить.**
- `compaction_bounds_segment_count`, `gc_reclaims_obsolete_versions`.
- `make storage-soak-72h-*`: место и p99 в заданных границах; write-amp под порогом.

**Exit.** 72h soak: space и p99 в бюджете; write-amp ограничен.

**Риск.** Средний.

---

## EPIC-16 — Производительность: SLO ретривала и ингеста на 500k+ `P1/L`

**Цель.** Latency-SLO для retrieve/context, throughput ингеста, RSS на полном корпусе.

**Предусловия.** Есть `docs/SINGLE_NODE_SLO.md`, `scripts/perf_latency.py`,
`load_suite_check.py`.

**Подтверждённая регрессия (2026-06-10, P0-блокер для EPIC-01/17 на полном корпусе).**
На **переоткрытой** 511k-БД с `--skip-ingest` engine-search обходит persisted
fast-path: после reopen memtable непустой, поэтому
`search_persisted_query` (`crates/cortex-engine/src/search/database.rs:159`)
видит `changed_cell_ids_after(checkpoint_seq)` непустым и возвращает `Ok(None)` →
`search_cells_with_report_with_policy` пересобирает `SearchIndexes` из
`snapshot_versions()` (все 511k ячеек) **на каждый запрос**. Замер: 27+ минут,
100% CPU, RSS 11GB на 50 keyword-запросов (прерван). На свежей 4000-док БД тот же
путь быстр (~35с с ingest+checkpoint; 50 запросов <1с). **Фикс:** после reopen
ячейки, уже покрытые checkpoint_seq, не должны числиться «изменёнными» в memtable
(WAL-replay не должен помечать checkpointed-ячейки как post-checkpoint), плюс
кеш `persisted_index_state` между запросами и mmap/ленивая декодировка `.aci`.
Без этого фикса engine-keyword/hybrid непрактичны на полном корпусе.

**Уточнённый корень + фикс (2026-06-10, реализовано).** Реальная причина —
не snapshot-fallback (memtable после reopen пуст, WAL усечён до 16 байт), а то,
что `persisted_index_state()` пересобирался **на каждый запрос** без кеша: для
каждого из 50 запросов он перечитывал и декодил все live-сегменты
(`.acs` 2.6GB + `.aci` 2GB). Сделано:
1. **Кеш `PersistedIndexState` на `Database`** (`Mutex<Option<PersistedIndexCache>>`,
   ключ = fingerprint live-сегментов `(id, generation, checkpoint_seq)`,
   авто-инвалидация при checkpoint/compaction). 50× декодов → 1× декод.
   Тесты: `reopened_checkpointed_db_reuses_persisted_index_state_across_calls`,
   `checkpoint_that_changes_segments_invalidates_cached_persisted_index`.
2. **Lightweight чтение `.acs`** (`SegmentReader::read_candidate_entries`) — пропуск
   payload-байтов (нужны лишь `candidate→cell` + tombstone). Тест:
   `read_candidate_entries_matches_full_read_without_payload`.

Результат замера 511k engine-keyword (reused БД, `--skip-ingest`): **15:53 wall**
(один холодный декод) против фактически бесконечного времени до фикса (каждый из
50 запросов = ~16 мин декод). recall@10 engine-keyword = cached-lexical = **56.0%**
на clean-50 (регрессии нет; clean-50 = всё basic, поэтому query-understanding не
меняет исход).

**Остаточный bottleneck (следующий шаг EPIC-16):** один холодный декод всё ещё
~16 мин, и доминирует он **декодом 2GB `.aci`** (`LexicalIndex::read`), а не
`.acs` (lightweight-чтение wall-time почти не изменило: 860s→948s, в пределах
шума). Настоящий фикс масштаба — **mmap + ленивые posting-list по терму** для
`.aci`, чтобы не декодить весь индекс в память перед первым запросом. Это
storage-редизайн, вынесен в отдельный шаг.

**Шаги.**
1. Бенч ингеста батчами (throughput docs/s) на 500k; профиль RSS `.aci`/HNSW.
2. p50/p95/p99 на запрос для cached-lexical / engine-keyword / engine-hybrid / +rerank.
3. Кэш/индекс-load оптимизация: mmap постингов, ленивая декодировка `.aci`,
   переиспользование между запросами (горячий кеш).
4. Опубликовать single-node SLO на полном корпусе.

**Как проверить.**
- `make single-node-performance-check` на полном корпусе; пороги p99/throughput.
- Регрессия latency ловится историей (`performance_trend_check.py`).
- Профиль памяти в отчёте retrieval (поле уже есть: `resource_usage.rss_bytes`).

**Exit.** Опубликованный SLO выдерживается на 511k-корпусе; index-load latency
снижена против baseline.

**Риск.** Средний.

---

## EPIC-17 — Engine-driven bench без oracle-метаданных + held-out `P0/M`

**Цель.** Прогон EnterpriseRAG, где document_ids рождает **движок** (hybrid + rerank +
query-understanding) без `question_type`/`source_types`, с held-out split. Это единый
критерий, доказывающий EPIC-01/03/04/05/08.

**Предусловия.** Уже есть `official-clean` конвейер (guard, prepare, retrieve, answer,
judge), теперь и `--retrieval-mode engine-keyword|engine-hybrid` + `--rerank weighted`
(EPIC-04). Есть `extra_questions.jsonl` (кандидат на locked split).

**Шаги.**
1. **Held-out split:** dev (тюнинг) и locked (один финальный замер). Использовать
   `--split-name` и зафиксировать locked-набор (напр. `extra_questions.jsonl` или
   детерминированную выборку, не пересекающуюся с dev).
2. **Модовое сравнение retrieval (recall@10 vs gold, gold только для замера):**
   cached-lexical → engine-keyword → engine-keyword+rerank → engine-hybrid(+rerank).
   Через `enterprise-rag-bench-official-clean-compare-retrieval`.
3. **Answer+judge** на лучшем retrieval-режиме, три судьи (gemma/gemini/deepseek),
   формула `overall = mean(completeness if correct else 0)`.
4. **Gate:** улучшение должно держаться на **locked** split, не только dev.
   Добавить `make enterprise-rag-bench-official-clean-heldout-quality-check` с порогом
   «engine-режим ≥ cached-lexical» по recall и overall.

**Как проверить.**
- Кривая recall@10 растёт: cached-lexical → engine-keyword → +rerank → hybrid.
- Clean overall (held-out) у engine-режима ≥ cached-lexical у всех трёх судей.
- Регресс-gate краснеет, если engine-режим просел ниже lexical.

**Exit.** Воспроизводимое **engine-only, no-oracle, held-out** число, которое растёт
за счёт EPIC-01/03/04 (и далее 05/08), а не за счёт селекторов. Это и есть финальное
доказательство всей программы.

**Риск.** Средний: дорогой полный прогон (токены/время); держать dev-итерацию на 50,
full/locked — только для финального замера.

---

## Сводная последовательность

1. **Волна ContextPack:** 05 → 06 → 07.
2. **Волна verification:** 08 → 09 (после 06).
3. **Волна рва:** 10 → 11 → 12 (10 — фундамент).
4. **Волна storage:** 13 → 14 → 15 → 16.
5. **Сквозной критерий:** 17 — запускать после каждой волны как замер прогресса.

Главный КПЭ программы: held-out engine-only число (EPIC-17) монотонно растёт по мере
закрытия 01/05/08 — если оно стоит на месте, значит улучшается обвязка, а не движок.
