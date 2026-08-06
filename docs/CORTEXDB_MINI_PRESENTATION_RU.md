# CortexDB: база контекста для AI-агентов

Мини-презентация · июль 2026

> CortexDB — не ещё одна обёртка над векторным поиском. Это локальная
> single-node база данных, которая хранит знания надёжно и возвращает модели
> ограниченный, проверяемый и объяснимый контекст.

---

## Слайд 1. Проблема: обычный RAG — это набор разрозненных компонентов

Типичный RAG-проект приходится собирать из нескольких слоёв:

```text
документы → chunking → embeddings → vector DB → reranker
          → prompt assembly → LLM → собственные проверки и аудит
```

На каждом стыке появляются практические риски:

- релевантные документы теряются при поиске;
- в контекст попадают дубли, устаревшие или противоречивые фрагменты;
- лимит токенов контролируется уже после retrieval;
- provenance, permissions и audit приходится добавлять отдельно;
- приложение получает «похожие chunks», а не готовый контракт контекста.

**Идея CortexDB:** сделать контекст для агента не побочным результатом RAG-цепочки,
а основным типизированным продуктом базы данных.

---

## Слайд 2. Рынок: это не одна категория конкурентов

| Категория | Примеры | Что они дают |
| --- | --- | --- |
| RAG/agent-фреймворки | LangChain, LlamaIndex | Оркестрация, loaders, splitters, retrievers, prompts и интеграции |
| Векторные базы | Weaviate, Qdrant, Milvus, Pinecone, Chroma | Быстрый semantic/ANN-поиск по embeddings |
| Универсальные БД и поиск | PostgreSQL + pgvector, Elasticsearch/OpenSearch | Транзакции, SQL либо полнотекстовый поиск, фильтры и аналитика |
| Managed enterprise search/RAG | Vertex AI Search, OpenAI File Search, Amazon Q/Kendra | Готовый облачный ingestion, поиск и генерация ответов |
| End-to-end RAG-приложения | RAGFlow, AnythingLLM, Open WebUI + Chroma, OpenClaw | Готовый пользовательский RAG-продукт |
| **Context database** | **CortexDB** | Durable knowledge cells → policy-aware retrieval → ContextPack/VERIFY FACT |

LangChain и LlamaIndex — важные соседние технологии, но это **библиотеки**, а не
прямая замена базе данных. CortexDB может работать под агентным фреймворком как
долговечный storage/retrieval layer.

---

## Слайд 3. Что именно делает CortexDB

```text
CLI / HTTP / SDK / MCP
          ↓
       AQL query
          ↓
policy + scope filters
          ↓
lexical / vector / hybrid retrieval
          ↓
ContextPack: budget + citations + anomalies + explain
          ↓
optional deterministic VERIFY FACT
          ↓
        AI agent

WAL → MVCC MemTable → checkpoint/compact → durable indexes
```

Ключевые отличия:

- **Database-grade persistence:** WAL, MVCC, checkpoint, compaction и recovery.
- **AQL:** декларативные ограничения retrieval вместо набора ad-hoc вызовов.
- **ContextPack:** результатом являются не сырые строки или chunks, а
  ограниченный по токенам пакет контекста со ссылками и аномалиями.
- **Policy-aware retrieval:** scope и разрешения ограничивают кандидатов до
  выдачи данных агенту.
- **Deterministic `VERIFY FACT`:** отдельный проверяемый отчёт о поддержке,
  противоречиях и недостаточности доказательств.
- **Model-agnostic:** LLM не встроена в ядро; модель ответа можно заменить.

---

## Слайд 4. CortexDB против соседних подходов

| Возможность | LangChain / LlamaIndex | Vector DB | SQL / search engine | CortexDB beta |
| --- | :---: | :---: | :---: | :---: |
| Durable database core | зависит от backend | да | да | **да** |
| RAG/agent orchestration | **да** | обычно нет | нет | нет, интегрируется с runtime |
| Lexical + vector/hybrid retrieval | через интеграции | часто да | зависит от продукта | **да** |
| Декларативный agent-oriented query contract | custom chain | filters/query API | SQL/DSL | **AQL** |
| Token-budgeted context как типизированный ответ | собирается приложением | обычно нет | нет | **ContextPack** |
| Citations, anomalies и explain в одном контракте | собирается приложением | частично | частично | **да** |
| Детерминированная проверка факта и числовых конфликтов | custom logic/LLM | нет | custom logic | **VERIFY FACT** |
| Managed cloud / production HA | зависит от стека | часто да | часто да | **нет (not available in beta)** |

Главная формулировка: **векторные базы оптимизируют поиск ближайших объектов;
CortexDB оптимизирует подготовку управляемого контекста для решения агента.**

---

## Слайд 5. Проверка на 500 000+ корпоративных документов

EnterpriseRAG-Bench моделирует реальную внутреннюю базу компании: Slack, Gmail,
Linear, Google Drive, HubSpot, Fireflies, GitHub, Jira и Confluence, включая
шум, near-duplicates, противоречия и вопросы по нескольким документам.

Результат CortexDB, полученный организатором benchmark:

| Метрика | CortexDB |
| --- | ---: |
| **Overall Score** | **42.04** |
| Correctness | 47.40% |
| Completeness | 49.21% |
| Document Recall | 56.24% |
| Invalid Extra Docs | 9.2 |

Конфигурация CortexDB:

- база данных: **CortexDB**, локальный single-node запуск;
- answer model: **`google/gemma-4-31B-it`**;
- официальный judge: **GPT-5.4**;
- 500 основных вопросов + отдельный закрытый verification set из 80 вопросов;
- verification-ответы, по словам организатора, согласуются с основным результатом.

Важно: `Overall Score` — среднее от `correct × completeness` по каждому вопросу,
а не простое произведение двух агрегированных процентов.

---

## Слайд 6. Где этот результат находится среди конкурентов

Если добавить CortexDB в опубликованную таблицу EnterpriseRAG-Bench на
17 июля 2026 года:

| Место* | Решение | Overall Score |
| ---: | --- | ---: |
| 6 | Amazon Q (Kendra) | 48.96 |
| 7 | Azure AI Search | 48.42 |
| **8** | **CortexDB + Gemma 4 31B IT** | **42.04** |
| 9 | Vertex AI Search | 41.87 |
| 10 | NVIDIA AI Blueprints | 37.73 |
| 11 | Vector baseline + GPT-5.4 | 37.72 |
| 12 | AnythingLLM | 35.58 |
| 13 | Weaviate Verba | 34.48 |
| 14 | LlamaIndex, default config | 27.20 |
| 15 | LangChain, default config | 24.98 |
| 16 | Open WebUI + Chroma | 24.89 |

\* Проекционная позиция до фактической публикации CortexDB организатором.
В текущем leaderboard 15 решений; после добавления CortexDB было бы 16.

**Вывод:** первая официально проверенная попытка CortexDB уже попадает в верхнюю
половину таблицы и обходит 8 из 15 опубликованных решений. Это не SOTA, но это
сильное подтверждение жизнеспособности архитектуры beta-продукта.

---

## Слайд 7. Почему результат особенно важен — и где резерв роста

Почему это хороший первый результат:

- CortexDB — самостоятельная база данных, а не конфигурация готового managed RAG;
- система прошла крупный noisy enterprise corpus и закрытую cross-check проверку;
- ответы генерировала Gemma 4 31B IT, поэтому результат отражает работу всей
  связки «база + retrieval + контекст + модель», а не только дорогой answerer;
- запуск воспроизводим и не использует oracle metadata при retrieval/generation.

Главный резерв роста виден прямо в метриках:

- `56.24%` document recall — почти половина нужных документов ещё теряется;
- `9.2` invalid extra docs — в контекст попадает слишком много шума;
- до Azure AI Search — `6.38` пункта Overall, до Amazon Q — `6.92`.

Приоритеты следующей итерации:

1. source-aware query expansion и улучшение AQL retrieval;
2. более сильный reranking для конфликтов, дат, людей и project-related вопросов;
3. снижение дублей и invalid extra docs до сборки ContextPack;
4. адаптивный бюджет контекста по типу вопроса;
5. контролируемый повторный запуск с более сильной моделью ответа, чтобы отделить
   качество базы/retrieval от качества генерации.

---

## Слайд 8. Позиционирование

### Одним предложением

**CortexDB — локальная agent-native база данных, которая превращает долговечные
знания в ограниченный, цитируемый и проверяемый контекст для AI-агента.**

### Не обещание «заменить всё»

- PostgreSQL остаётся правильным выбором для relational workloads.
- Weaviate/Qdrant/Milvus/Pinecone сильнее как специализированный vector layer.
- LangChain/LlamaIndex сильнее как orchestration framework.
- Vertex AI Search и другие managed-сервисы сильнее по облачной зрелости.

### Настоящее преимущество

CortexDB соединяет в одном локальном контракте то, что в обычном RAG приходится
склеивать вручную: **durability + retrieval policy + token budget + provenance +
verification**.

> Мы ещё не первые в рейтинге. Но первая beta-версия уже доказала, что новая
> архитектура конкурентоспособна на реальном масштабе — и её главный bottleneck
> измерен достаточно точно, чтобы понимать, что улучшать дальше.

---

## Короткий устный питч (45 секунд)

«Большинство RAG-систем собираются из библиотеки, векторной базы и набора
пользовательских проверок. В итоге модель получает сырые фрагменты, а контроль
токенов, прав доступа, источников и противоречий остаётся на приложении.
CortexDB переносит эту ответственность в саму базу данных: надёжно хранит
knowledge cells, выполняет AQL retrieval и возвращает готовый ContextPack со
ссылками, бюджетом и аномалиями. На EnterpriseRAG-Bench с более чем 500 тысячами
документов первая beta-версия набрала 42.04 с Gemma 4 31B IT. После публикации
это соответствовало бы 8-му месту из 16 — выше Vertex AI Search, NVIDIA AI
Blueprints, Weaviate, LlamaIndex и LangChain. Мы не заявляем SOTA или production
readiness, но уже доказали, что архитектура работает, и видим конкретный путь
роста через recall и снижение шумных документов.»

---

## Источники и границы утверждений

- [EnterpriseRAG-Bench dataset and metrics](https://huggingface.co/datasets/onyx-dot-app/EnterpriseRAG-Bench)
- [EnterpriseRAG-Bench leaderboard CSV](https://huggingface.co/spaces/onyx-dot-app/EnterpriseRAG-Bench-Leaderboard/blob/main/data/final_display_data/leaderboard.csv)
- [EnterpriseRAG-Bench repository and reproducibility policy](https://github.com/onyx-dot-app/EnterpriseRAG-Bench)
- [LangChain retrieval documentation](https://docs.langchain.com/oss/python/langchain/retrieval)
- [LlamaIndex documentation](https://docs.llamaindex.ai/)
- [Vertex AI Search overview](https://docs.cloud.google.com/vertex-ai/generative-ai/docs/learn/vertex-ai-search)
- [Weaviate hybrid search documentation](https://docs.weaviate.io/weaviate/search/hybrid)
- Локальные источники CortexDB: [`README.md`](../README.md),
  [`ARCHITECTURE.md`](ARCHITECTURE.md), [`COMPARISONS.md`](COMPARISONS.md),
  [`PUBLIC_CLAIMS_POLICY.md`](PUBLIC_CLAIMS_POLICY.md).

Результат CortexDB `42.04` передан организатором после двух основных прогонов и
verification cross-check. До появления строки CortexDB в публичном leaderboard
место `8/16` следует называть **расчётной позицией**, а не опубликованным рангом.
CortexDB `v0.2.0-beta.2` — single-node beta и пока не рекомендуется для
production workloads.
