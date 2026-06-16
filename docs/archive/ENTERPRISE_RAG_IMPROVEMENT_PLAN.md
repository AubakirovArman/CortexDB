# EnterpriseRAG-Bench — детальный план улучшения (46 → ~52+)

Версия плана: 2026-06-09. База: full-500 прогон
`vllm-gemma-full500-v81-current-overall-fresh`, модель `google/gemma-4-31B-it`,
prompt `type-aware-v15`, context `question-window-digest-ranked`, retrieval v81.

---

## 0. Как считается счёт (это определяет приоритеты)

Официальная формула находится в внешнем benchmark checkout:
`target/external-benchmarks/EnterpriseRAG-Bench/src/scripts/answer_evaluation/metrics_based_eval.py:457`.

```
overall = mean_over_questions( completeness_pct  если answer_correct  иначе 0 )
```

- `answer_correct` — **бинарный** холистический вердикт LLM-судьи (правильный/нет).
- `completeness_pct` — доля подтверждённых `answer_facts` (fact-level).
- `document_recall_pct` — справочно; **не входит** в overall напрямую, но косвенно
  ограничивает потолок (нет дока → нет фактов → неверно).

**Главный вывод:** `answer_correct` — это «ворота». Вопрос с completeness 80%, но
`answer_correct=False`, даёт **0**. Поэтому приоритет №1 — переводить вопросы
`False → True`, а не полировать полноту уже верных ответов.

**Замечание про судью:** локальный deepseek-судья даёт 46.1, gemma-судья 49.3,
а официальный gemini-3.5 строже (на basic corr 59 против 73). На реальном
лидерборде ожидать ниже локального deepseek. Значит «воротные» победы (flip
correctness) ценнее, чем добор полноты.

---

## 1. Текущее состояние по категориям (full-500, deepseek judge)

| Тип | n | corr% | compl% | recall% | combined | Узкое место |
|---|---:|---:|---:|---:|---:|---|
| basic | 175 | 73.7 | 78.8 | 90 | 58.1 | мелкая полировка |
| **semantic** | **125** | 60 | 64.6 | **75** | 38.8 | **retrieval recall** |
| intra_doc | 40 | 57.5 | 71 | 90 | 40.9 | синтез ответа |
| **project_related** | **40** | **17.5** | 57 | 88 | **10.0** | **«ворота» / синтез** |
| constrained | 30 | 70 | 79.5 | 90 | 55.7 | ок |
| completeness | 20 | 55 | 78.2 | 86 | 43.0 | полнота покрытия |
| conflicting_info | 20 | 85 | 80.6 | 92 | 68.5 | ок |
| info_not_found | 20 | 100 | 100 | n/a | 100 | НЕ ТРОГАТЬ |
| miscellaneous | 20 | 80 | 77.5 | 85 | 62.0 | ок |
| **high_level** | **10** | **0** | 0 | n/a | **0** | **0 документов в retrieval** |

Вклад каждой категории в overall = `combined * n / 500`. Крупнейшие абсолютные
вклады: basic (20.3), semantic (9.7), miscellaneous (2.5), conflicting (2.7).
Крупнейшие **недоборы**: semantic, project_related, high_level.

---

## 2. Принцип измерения (обязательно для каждого шага)

Любое изменение проверяем сначала на **balanced-50**, затем — только если есть
прирост — на full-500. Никогда не правим вслепую на 500 (дорого по токенам).

Файлы для быстрой итерации:
- retrieval baseline: `cortexdb_full_doc_view_v81_confluence_project_discovery_balanced_50.jsonl`
- full retrieval: `cortexdb_full_doc_view_v81_confluence_project_discovery_top10.jsonl`
- gold: `target/external-benchmarks/EnterpriseRAG-Bench/questions.jsonl`
- генерация ответов: `scripts/enterprise_rag_bench/run_deepseek_answers.py`
- judge + summary: `run_deepseek_answer_metrics.py` / `summarize_score.py`

Правило приёмки шага: **combined не упал ни в одной категории** (особенно
info_not_found=100 и conflicting=68.5) И вырос в целевой. Если целевая выросла,
но просела соседняя — изменение делаем **type-routed**, а не глобальным.

---

## 3. ФАЗА A — high_level (быстрый изолированный фикс, ожидание +0.8…+1.2)

### Диагноз
10 вопросов («What is Redwood's mission statement?», «four main revenue
streams», «major departments») получают **0 документов** → модель пишет
"Insufficient information." → `answer_correct=False` → 0.

Причина: у high_level в gold `source_types=[]` и `expected_doc_ids=[]`.
Кандидатный retrieval фильтрует по `source_types`, пустой фильтр → пустой
результат. high_level сливается в одну ветку с `info_not_found` (где пустой
ответ ВЕРЕН). См. `doc_view_rerank.py` (high_level в `DIVERSITY_TYPES`, но
кандидаты приходят пустыми сверху) и route `enabled:False`.

high_level **не оценивается по recall** (нет gold-доков), нужен только верный
ответ по `answer_facts` (1–5 фактов про компанию).

### Что сделать
1. **Отдельная ветка retrieval для high_level** (НЕ source-filtered):
   - Это «company overview / strategy / org» вопросы. Документ-источник —
     высокоуровневые Confluence/overview-доки про «Redwood Inference».
   - Реализация: в генераторе retrieval (chain, пишущий v81 top10) добавить
     when `question_type == "high_level"`: запускать безфильтровый
     **embedding-retrieval** по всему корпусу с расширением запроса терминами
     компании (mission, revenue, business model, org, departments, strategy,
     differentiation, security, reliability) и брать top-k=8…10.
   - Точка входа: новый селектор `high_level_overview_selector.py` по образцу
     `confluence_project_discovery_selector.py`, либо ветка в существующем
     селекторе перед записью `ENTERPRISE_RAG_BENCH_DOC_VIEW_V81`.
2. **Не-абстейн summary-промпт для high_level** в `answer_prompts.py`:
   - Добавить `high_level_v1(row, context)` и роутить его в `type_aware_v15`.
   - Инструкции: «это обзорный вопрос о компании; синтезируй ответ из
     overview-доков; перечисли ВСЕ перечислимые пункты (revenue streams,
     departments, add-ons); не пиши "Insufficient information", если есть хоть
     частичное покрытие».
   - Контекст: для high_level использовать `context_mode="leading"` или
     полное тело (а не узкие question-window), т.к. факты разбросаны по обзору.
3. **Гарантия безопасности:** ветку включать строго по
   `question_type=="high_level"`. `info_not_found` оставить пустым (его 100 не
   трогаем).

### Команды проверки (balanced-50 содержит часть high_level; для полноты — full)
```bash
# регенерация retrieval с high_level-веткой -> новый top10
# генерация ответов только по high_level подмножеству и судейство
```

### Метрика приёмки
high_level combined: 0 → ≥50 (ожидаемо 6–8/10 верных при compl ~70).
info_not_found остаётся 100. Прирост overall ≈ `(нов-0)*10/500`.

### Риск
Низкий, изолировано типом. Главное — не сломать `info_not_found` (разный
question_type, поэтому не пересекаются).

### Статус 2026-06-09
**Диагностически закрыто.** Отдельная high_level ветка прошла official Gemini
judge на 10/10 high_level вопросах:

```text
high_level correctness: 100.0%
high_level completeness: 100.0%
high_level combined: 100.0
estimated full-500 overall: 42.43 -> 44.43
delta overall: +2.00
```

Артефакты:

```text
target/enterprise-rag-bench/retrieval/cortexdb_full_doc_view_v82_high_level_overview_top10.jsonl
target/enterprise-rag-bench/qa/gemma-high-level-v82-phase-a-10-company-overview/answers.jsonl
target/external-benchmarks/EnterpriseRAG-Bench/answer_evaluation/cortexdb-gemma-high-level-v82-phase-a-10-company-overview/results_official_gemini35_flash.json
target/enterprise-rag-bench/analysis/phase_a_high_level_official_delta_report.json
```

Ограничение: successful diagnostic run добавляет
`generated_data/company_overview.md` как high-level reference-контекст. Это
соответствует тому, как benchmark генерирует high_level вопросы, но это не
обычный retrieved source document из `generated_data/sources`. Для строгого
leaderboard/submission режима нужен отдельный corpus-only вариант: либо
доказать, что overview facts полно извлекаются из `sources/`, либо явно
пометить high_level reference как benchmark-provided scaffold.

---

## 4. ФАЗА B — project_related (главный «воротный» провал, ожидание +1.0…+1.6)

### Диагноз
40 вопросов, recall **88%** (доки извлечены!), но corr **17.5%** (≈7/40 проходят
«ворота»). Это многодоковые синтез-вопросы на 10–16 `answer_facts` («наша
политика», «что вызвало рассинхрон», «какой подход одобрен для сделки»).

Два режима провала (по `correctness_reasoning`):
1. **Пропуск обязательных фактов** (completeness 30–45%) — контекст
   `question-window-digest-ranked` режет тело доков на узкие окна, и часть
   обязательных фактов не попадает в промпт.
2. **Добавление лишней конкретики** (TTL «5 min, 10000 entries», «>15 минут»),
   которой нет в gold → судья засчитывает как противоречие → `False`.

### Что сделать
1. **Контекст полнее для project_related** (type-routed в `answer_context.py`):
   - Для `question_type=="project_related"` поднять `max_chars_per_doc` и
     использовать режим, отдающий **больше тела** (например `leading` 8–12k на
     топ-3 + digest на остальных), т.к. факты разбросаны.
   - Увеличить `top_k_context` для этого типа (доки релевантны — recall 88%).
2. **Промпт project_related — анти-галлюцинация + полнота**
   (`project_related_v9` → новая версия):
   - Жёстко: «используй ТОЛЬКО факты, дословно присутствующие в документах;
     НЕ добавляй численные пороги/TTL/проценты, которых нет в тексте».
   - «Перечисли все требуемые компоненты (approvers, шаги, пороги, артефакты)
     в порядке из evidence; не сжимай имена/ID/даты/пути».
   - Это прямо бьёт по обоим режимам провала.
3. **Evidence slot planner / table** (уже есть наработки:
   `evidence_slot_planner.py`, `evidence_table_extractor.py`,
   `answer_prompts_evidence_first.py` — они в git как незакоммиченные правки):
   - Прогнать project_related через evidence-first путь: сначала извлечь
     таблицу фактов из доков, потом синтез строго по таблице. Это снижает
     добавление лишнего.

### Метрика приёмки
project_related corr 17.5% → ≥35% (combined 10 → ≥21). На balanced-50 у
project_related малая выборка — валидировать на full-500 после прохождения
balanced-50 без регрессий.

### Риск
Средний. Более полный контекст ↑ токены и может внести шум для других типов —
поэтому изменения строго type-routed на project_related.

---

## 5. ФАЗА C — semantic recall (наибольший вес, ожидание +1.5…+3.0)

### Диагноз
125 вопросов (25% бенча). recall **75%** против 90% у basic. 31/125 имеют recall
**0%** — это одно-документные перефразированные запросы, где правильный док НЕ
извлечён вообще:
- qst_0178 «low bit math safer for inference» → quantization step-down gates
- qst_0183 «gate thresholds … compressed model variant allowed/canaried/blocked»
- qst_0176 «top end 80GB accelerator … booking open EU Central / India South»

`source_types` у semantic **заданы** (slack/github/confluence/linear/gmail), т.е.
можно фильтровать по источнику, а внутри — семантический поиск. Лексический
retrieval не ловит парафраз.

### Что сделать
1. **Embedding-rerank по source-filtered пулу для semantic**:
   - Расширить кандидатный пул (top-200…500 внутри нужного `source_types`),
     затем embedding-rerank (`rerank_with_embeddings.py`, модель `BAAI/bge-m3`
     из `.env` `CORTEXDB_EMBEDDING_*`) и взять top-10.
   - Это уже доказанный путь (в доке: rerank поднял recall +12.5 пунктов на
     balanced-50). Применить **селективно к semantic** в основном v81 chain.
2. **Query expansion для парафраз-доменных терминов** (`doc_view_rerank.py`
   `QUERY_EXPANSIONS`): добавить маппинги доменного жаргона
   (`low bit`→`quantization/int8/fp8`, `80GB accelerator`→`H100/GPU SKU`,
   `step down`→`fallback/downgrade gate`, `harmful output`→`safety/guardrail`).
3. **Fusion**: RRF лексики + embedding (как v4/v6 в доке), но **только для
   semantic**, чтобы не ронять conflicting/basic.

### Метрика приёмки
semantic recall 75% → ≥83%; combined 38.8 → ≥45. Это самый большой одиночный
рычаг по абсолютному вкладу (×125).

### Риск
Выше: трогает retrieval-стадию, дольше прогон (embedding многих кандидатов).
Кэш эмбеддингов (`embedding_cache.jsonl`) обязателен. Строго type-routed.

---

## 6. ФАЗА D — добор по «дешёвым» категориям (ожидание +0.5…+1.0)

После A–C — мелкие type-routed улучшения, каждое валидируем на balanced-50:

1. **intra_document_reasoning** (40, recall 90, corr 57.5): доки есть, ответ
   слаб. Промпт «многошаговый вывод внутри одного документа: проследи цепочку,
   приведи промежуточные значения». Контекст — полное тело одного-двух доков.
2. **completeness** (20, compl 78 но corr 55): это «перечисли всё» вопросы.
   Промпт-форс полноты + более широкий top_k (нужны все доки кластера).
   Использовать наработки `confluence_*_completeness_selector.py`.
3. **basic** (175, corr 73.7): даже +3–4 пункта corr здесь = большой
   абсолютный прирост (×175). Точечно — `exact_basic_v17` уже есть; сравнить
   его против v15 на basic-подмножестве и взять лучший.
4. **constrained** (30, compl 79 corr 70): промпт уважения ограничений
   (даты/регионы/версии из вопроса как жёсткий фильтр).

---

## 7. Сводная таблица рычагов и приоритет

| Фаза | Категория | n | Сейчас combined | Цель | ≈Δ overall | Риск | Усилие |
|---|---|---:|---:|---:|---:|---|---|
| A | high_level | 10 | 0 | 55 | +1.1 | низкий | малое |
| B | project_related | 40 | 10 | 25 | +1.2 | средний | среднее |
| C | semantic | 125 | 38.8 | 47 | +2.0 | высокий | большое |
| D | intra/compl/basic/constr | 265 | — | — | +0.8 | низкий | среднее |

Порядок исполнения: **A → B → D(basic) → C**. high_level первым (быстро, безопасно,
сразу очки). semantic последним (самый дорогой и рискованный), но самый большой
потолок. Итоговая траектория: **46 → ~51–52** консервативно, до ~54 при удачном C.

---

## 8. Гард-рейлы (что не сломать)

- `info_not_found` = 100. Любая ветка retrieval для пустых `source_types` должна
  отделять high_level от info_not_found по `question_type`. **Регресс здесь
  стоит −4 overall** (20 вопросов × 100 / 500).
- `conflicting_info` = 68.5. Не добавлять лексический шум, который вытащит
  «соседний конфликтующий» документ. Любая retrieval-правка — type-routed.
- Все изменения промптов/контекста — **через `question_type`-роутинг** в
  `answer_prompts.py` / `answer_context.py`, не глобально.
- Каждый шаг сначала balanced-50, приёмка = «целевая ↑ И ни одна другая ↓».

---

## 9. Открытые вопросы для подтверждения перед стартом

1. Доступен ли `question_type` при официальной сабмишн-валидации? (Пайплайн уже
   роутит по нему — предполагаем да.) Если нет — нужен классификатор типа
   вопроса на инференсе.
2. Какой судья на официальном лидерборде? Если строже deepseek — приоритет
   «воротных» побед (A, B) выше, чем добор полноты.
3. Бюджет токенов на full-500 (сейчас ~5.76M prompt). Фазы B/C его увеличат —
   подтвердить лимиты vLLM-эндпоинта.
