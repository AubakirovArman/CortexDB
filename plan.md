Принято. Начинаем не “исследование”, а **реализацию CortexDB**.

Я беру за основу текущую спецификацию: **AQL → Raw AST → Binder Plane → BoundRetrievePlan → Bitmap VM / Candidates_0**, которую мы уже довели до Milestone 0.3-alpha. 

# PRD-план разработки CortexDB MVP

## 1. Название проекта

**CortexDB**
Agent-native context database для ИИ-агентов.

Короткое описание:

> CortexDB — это собственная база контекстного мозга для ИИ-агентов, которая хранит Knowledge Cells, компилирует AQL-запросы в безопасные планы, применяет permission-safe retrieval и возвращает агенту не строки и не chunks, а готовый Context Pack.

---

# 2. Главная цель MVP

Создать первый рабочий сквозной движок:

```text
AQL string
→ Parser
→ Raw AST
→ Binder
→ BoundRetrievePlan
→ Bitmap VM
→ Candidates_0
```

То есть в MVP мы пока не строим всю промышленную базу. Мы сначала строим **ядро компилятора и безопасного контекстного фильтра**, потому что без него нельзя двигаться к WAL, `.acs`, `.acb`, `.aci`, retrieval и context packing.

---

# 3. Главный принцип разработки

Не использовать готовые СУБД как storage backend.

Разрешено использовать Rust crates для парсинга, тестов, checksum, mmap, compression, но **CortexDB storage engine пишется самостоятельно**.

```text
Нельзя:
PostgreSQL
Qdrant
ChromaDB
Neo4j
Elasticsearch
SQLite как основное хранилище

Можно:
nom
nom_locate
crc32c
zstd
memmap2
roaring
tokio
crossbeam-channel
serde только для debug/API, не для WAL core
```

---

# 4. MVP Scope

## Входит в MVP

### AQL Compiler

* `types.rs`
* `agent_view.rs`
* `policy.rs`
* `ast.rs`
* `parser.rs`
* `binder.rs`
* `executor_mock.rs`

### Security Model

* `AgentView`
* read/write scopes
* allowed modes
* token budget limits
* candidate limits
* `PolicyValidator`
* `Diagnostic Mode`
* `Enforcement Mode`

### AQL v0.3-alpha

Поддержать минимум:

```sql
RETRIEVE CONTEXT
FOR TASK "..."
IN BRAIN investment_projects
USING MODE balanced
BUDGET 12000 TOKENS
WHERE space = "project:investments" AND status = "ready";
```

### Bitmap VM

* `PushAgentAllowed`
* `PushLive`
* `Push(BitmapHandle)`
* `And`
* `Or`
* `Not`
* mock execution через `BTreeSet<u32>`

### Unit Tests

* Q16 math
* policy clamp/deny
* parser basic retrieve
* escaped strings
* binder scope/status
* bitmap VM evaluation
* citations invariant
* stack depth computation

---

## Не входит в первый MVP

Пока не делаем:

* настоящий WAL `.aclog`;
* настоящий mmap segment `.acs`;
* настоящий Roaring `.acb`;
* настоящий BM25 `.aci`;
* настоящий vector `.acv`;
* Context Pack optimizer;
* HNSW;
* LSM compaction;
* distributed replication.

Это будет следующий слой после стабилизации AQL/Binder/Bitmap VM.

---

# 5. Архитектура репозитория

```text
cortexdb/
  Cargo.toml

  crates/
    cortex-aql/
      Cargo.toml
      src/
        lib.rs
        types.rs
        agent_view.rs
        policy.rs
        ast.rs
        errors.rs
        parser.rs
        binder.rs
        executor_mock.rs

      tests/
        aql_parser.rs
        policy_tests.rs
        binder_tests.rs
        bitmap_vm_tests.rs
        e2e_retrieve.rs

    cortex-core/
      Cargo.toml
      src/
        lib.rs

    cortex-storage/
      Cargo.toml
      src/
        lib.rs

    cortex-server/
      Cargo.toml
      src/
        main.rs
```

В первом спринте реально пишем только:

```text
crates/cortex-aql
```

Остальные crates можно создать пустыми как placeholders.

---

# 6. Этапы разработки

## Milestone 0.1 — Bootstrap Rust Workspace

Цель: создать чистый workspace.

Задачи:

* создать `Cargo.toml`;
* создать `crates/cortex-aql`;
* подключить зависимости:

  * `nom`
  * `nom_locate`
  * опционально `thiserror`;
* настроить `cargo test`;
* настроить `cargo fmt`;
* настроить `cargo clippy`.

Критерий готовности:

```text
cargo test
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

проходят.

---

## Milestone 0.2 — Types + AgentView + Policy

Файлы:

```text
types.rs
agent_view.rs
policy.rs
```

Реализовать:

* `AgentId`
* `BrainId`
* `ScopeId`
* `LensId`
* `Q16`
* `q16_from_f64_clamped`
* `q16_mul`
* `RetrievalMode`
* `MemoryType`
* `AgentView`
* `PolicyValidator`
* `PolicyError`
* `PolicyDiagnostic`
* `PolicyReport`
* `EffectiveRetrievePolicy`

Особое правило:

```text
Execution = Fast-Fail
Diagnostics = Collect-All
```

Критерии готовности:

* budget clamp работает;
* candidate limit clamp работает;
* audit mode запрещается без права;
* brain access проверяется;
* remember проверяет write scope;
* verify fact имеет отдельную ошибку `VerifyFactNotAllowed`.

---

## Milestone 0.3-alpha — AQL Parser

Файлы:

```text
ast.rs
errors.rs
parser.rs
```

Реализовать:

* `SourceSpan`
* `Spanned<T>`
* `AqlString<'a>`
* `Identifier<'a>`
* `DecimalLiteral`
* `Literal`
* `Condition`
* `Requirement`
* `Strategy`
* `TtlValue`
* `RawRetrieveContext`
* `AqlStatement`
* parser для `RETRIEVE CONTEXT`;
* `Cow::Borrowed` для строк без escape;
* `Cow::Owned` для escaped strings;
* `WHERE` с precedence:

  * `NOT`
  * `AND`
  * `OR`;
* depth limit для `WHERE`.

Критерии готовности:

* парсится базовый `RETRIEVE CONTEXT`;
* парсится `WHERE space = "..." AND status = "ready"`;
* missing semicolon даёт ошибку;
* unterminated string даёт ошибку;
* переполнение integer не превращается в `0`.

---

## Milestone 0.4-alpha — Binder Plane

Файл:

```text
binder.rs
```

Реализовать:

* `AqlCatalog` trait;
* `BitmapHandle`;
* `BitmapOp`;
* `BitmapProgram`;
* `QualityThresholds`;
* `RetrievalWeights`;
* `ContextPolicy`;
* `BoundRetrievePlan`;
* `Binder::bind_retrieve`;
* `decimal_to_q16`;
* `compute_bitmap_stack_depth`;
* `default_weights`;
* `context_policy_for_mode`.

Ключевые инварианты:

```text
AQL не расширяет права AgentView.
Fast mode не может выключить обязательные citations.
WHERE scope обязан проверяться против AgentView.
SourceTrust/Freshness нельзя silently ignore.
Weights должны нормализоваться в Q16.
NOT считается только внутри segment-local universe.
```

---

## Milestone 0.5-alpha — Bitmap VM Mock Executor

Файл:

```text
executor_mock.rs
```

Реализовать:

* `BitmapProvider`;
* `MockBitmapProvider`;
* `eval_bitmap_program`.

Mock backend:

```text
BTreeSet<u32>
```

Критерий готовности:

```text
AgentAllowed = {1,2,3,4}
Live = {2,3,4,5}
scope(project) = {3,4,5}
status(ready) = {4,5,6}

Result:
AgentAllowed ∩ Live ∩ scope ∩ status = {4}
```

---

## Milestone 0.6 — End-to-End AQL Pipeline

Тест:

```text
AQL string
→ parse_statement
→ bind_retrieve
→ eval_bitmap_program
→ expected Candidates_0
```

Пример запроса:

```sql
RETRIEVE CONTEXT
FOR TASK "Сравнить бюджеты инвестиционных проектов ТОО ABC за 2025 год"
IN BRAIN investment_projects
USING MODE balanced
BUDGET 12000 TOKENS
WHERE space = "project:investments" AND status = "ready";
```

Ожидаемый результат:

```text
BoundRetrievePlan создан.
Policy применена.
Bitmap bytecode создан.
Bitmap VM вернула корректную маску кандидатов.
```

---

# 7. После MVP: следующие большие блоки

После AQL pipeline:

## Milestone 1.0 — ACLOG WAL

* binary frame header;
* TLV payload;
* 8-byte alignment;
* CRC32C;
* delta-only patches;
* WAL Writer Actor;
* group commit.

## Milestone 1.1 — MemTable MVCC

* `created_seq`;
* `deleted_seq`;
* versions;
* tombstones;
* `ReadTxn`.

## Milestone 1.2 — Manifest + mmap lifecycle

* `Manifest`;
* `SegmentHandle`;
* `Arc<Manifest>`;
* retired segments;
* safe reclamation.

## Milestone 1.3 — `.acs` payload segments

* immutable segment;
* cell directory;
* materialized cells;
* source refs.

## Milestone 1.4 — `.acb` bitmap index

* real Roaring Bitmaps;
* segment-local ordinals;
* `AgentAllowed`, `Live`, `Status`, `Scope`.

## Milestone 1.5 — `.aci` lexical index

* tokenizer;
* term dictionary;
* postings;
* BM25.

## Milestone 1.6 — Context Plane

* cascade retrieval;
* heavy scoring;
* MMR;
* Numeric Guards;
* Context Pack.

---

# 8. Definition of Done для первого этапа

Первый этап считается завершённым, если:

```text
1. Rust workspace собирается.
2. Все tests проходят.
3. AQL parser парсит RETRIEVE CONTEXT.
4. Binder создаёт BoundRetrievePlan.
5. PolicyValidator не допускает расширения прав.
6. Bitmap VM возвращает Candidates_0.
7. Нет float в scoring thresholds.
8. Нет unwrap_or(0) на критичных числах.
9. Нет silent ignore требований качества.
10. cargo clippy проходит без warning.
```


Скопируй это в Codex.

```text

```

---