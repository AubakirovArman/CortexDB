# План развития CortexDB — «Подотчётность ответа»
## Дорожная карта к Governed Context Engine (GCE)

> **Документ:** план развития / стратегическая дорожная карта
> **Версия проекта на момент составления:** `v0.2.0-beta.2`
> **Дата:** 2026-06-28
> **Северная звезда:** сделать *подотчётность ответа* (answer accountability) непоглощаемым контрактом —
> каждый `ContextPack`/`VERIFY`-ответ несёт детерминированный, криптографически проверяемый **accountability receipt**,
> который независимая третья сторона проверяет **без доверия к базе**.
>
> **Как составлен.** Документ — результат многоагентного исследования с заземлением на исходный код
> (ссылки `file:line` подтверждены чтением файлов) и **состязательным ревью** плана. Все ключевые
> утверждения о текущем состоянии проверены по коду; дефекты (cosine `dot.abs()`, XOR-«шифрование»,
> `(project,metric)`-эвристика конфликтов, расхождение fail-closed на ANN-пути, отсутствие крипто-примитивов)
> подтверждены в исходниках.

---

## Оглавление

- [0. Резюме (executive summary)](#0-резюме)
- [1. Северная звезда: контракт и проверяемый receipt](#1-северная-звезда)
- [2. Текущее состояние подотчётности (по коду)](#2-текущее-состояние)
- [3. Столпы развития](#3-столпы-развития)
- [4. Дорожная карта по фазам](#4-дорожная-карта)
- [5. Доказательство категории (тест Oso)](#5-доказательство-категории)
- [6. Обязательные исправления, риски и метрики](#6-исправления-риски-метрики)
- [Приложение A. Детализация столпов](#приложение-a-детализация-столпов-задачи-exit-гейты-усилия-зависимости)
- [Приложение B. Реконсилированная фазовая последовательность](#приложение-b-реконсилированная-фазовая-последовательность-и-критический-путь)
- [Приложение C. Реестр гейтов](#приложение-c-реестр-гейтов-make-name)

---

## 0. Резюме

**Что это.** План превращения CortexDB из «правдоподобного serving-слоя» в защитимое новое поколение БД —
**Governed Context Engine (GCE)** — через единственный непоглощаемый контракт: *подотчётность ответа*.

**Ключевая поправка состязательного ревью (определяет весь план).** Сама по себе подпись + Merkle-бандл —
**не ров**: тонкая обёртка `pgvector + policy-engine + ed25519` воспроизводит ~80% receipt. Непоглощаемыми
являются ровно **две вещи**, и они — P0:

1. **`access_root`, привязанный к фактически исполненной алгебре плана.** Верификатор обязан
   *переисполнить подписанную `bitmap_program` против процитированных ячеек*. Обёртка, лишь заявляющая лист
   `allowed`, проваливается, пока не реализует ту же fail-closed-алгебру `[PushAgentAllowed, PushLive, And]`.
2. **Transparency log (anti-equivocation anchor).** Единственный класс подделки, который подпись тоже не
   закрывает: БД подписывает два противоречивых, но по отдельности валидных receipt'а. В исходном плане это
   занижено до «stub» — здесь поднято в P0.

**Авторитетная последовательность (7 фаз).** Критик корректно вынес фундамент в Phase 0 и поставил
багофиксы перед подписью («receipt над испорченными данными = нотариально заверенная ложь»):

| # | Фаза | Тег | Суть |
|---|---|---|---|
| P0 | Фундамент детерминизма + канонических байт | `beta.3` | один `canonical_bytes()`, исключение `elapsed_nanos` — единая точка согласия БД↔верификатор |
| P1 | Корректность (предусловия) | `beta.4` | cosine `abs()`, cell-id slot-width, numeric-aware конфликты, `budget_exceeded`-аномалия |
| P2 | Настоящая крипта + подписанный receipt + standalone-верификатор | `v0.3.0` | первая крипта (BLAKE3 + Ed25519), Merkle-receipt, захваченный access, верификатор без связи с движком |
| P3 | Fail-closed parity на физическом слое | `v0.3.0` | persisted ANN `allowed` == `eval_bitmap_program(plan)`; `model_hash` в receipt — делает `access_root` честным |
| P4 | Crypto hardening (audit + at-rest) | `v0.3.0` | AEAD-бэкап (XChaCha20-Poly1305 + Argon2id), keyed audit-chain, честные доки |
| P5 | Verify-сила + замороженный learned-ранкер | `v0.4.0` | измеренный contradiction recall ≥0.90; веса ранжирования в аудируемом Q16-артефакте |
| P6 | Cluster fail-closed + transparency anchor | `v1.0-rc` | Raft из research-track в release; equivocation-лог как first-class |

**Критический спинной хребет (строго серийный):**
`canonical_bytes()+детерминизм (P0)` → `багофиксы (P1)` → `крипто-модуль + Merkle-receipt + Ed25519 (P2)` →
`standalone-верификатор (P2)` → `ANN-parity + захваченный access + model_hash (P3)` → `transparency log`.

**Три первых хода (highest-leverage):** (1) Phase 0 — `canonical_bytes()` + исключение `elapsed_nanos`;
(2) багофиксы корректности (cosine, cell-id, конфликты, budget-аномалия); (3) крипто-модуль +
plan-bound `access_root` + transparency log. **Срезать:** живой 4-конкурентный бенчмарк-матрикс (только
nightly, snapshot); per-scope ANN-субграфы (отложить — exact-fallback достаточно).

---

## 1. Северная звезда

### 1. Тезис и почему это не фича

CortexDB сегодня — это не новый storage-движок: субстрат учебниковый (ACLOG WAL + MVCC MemTable + LSM-сегменты + roaring bitmap + BM25 + i16/HNSW, Raft есть, но за feature-флагом). Реально новой является **категория результата** — Governed Context Engine (GCE), где результат запроса — это `ContextPack`: отфильтрованный по правам, ограниченный по бюджету токенов, процитированный, span-провенансированный, конфликт-аннотированный бандл для LLM, плюс детерминированная LLM-free операция `VERIFY FACT` (`Supported/Contradicted/Mixed/Insufficient` + `numeric_conflicts`).

Проблема: каждый из этих артефактов по отдельности воспроизводим тонкой библиотекой над `pgvector + policy-engine (Cerbos/OpenFGA) + RAG-framework`. По тесту Oso «feature-or-product» и по прецеденту Stonebraker/Pavlo (vector-DB поглощены RDBMS за ~год) категория «новая БД» **заявлена, но не доказана**. Cerbos для RAG отдаёт query-plan, который *приложение* само транслирует в фильтры — ровно та обёртка, которую предсказывает тезис поглощения.

**Северная звезда:** сделать «подотчётность ответа» (answer accountability) **непоглощаемым контрактом**. Конкретно — каждый `ContextPack`/`VERIFY`-ответ несёт детерминированный, **криптографически проверяемый accountability receipt**, который независимая третья сторона проверяет **БЕЗ доверия к базе**. Это превращает «ответ подотчётен» из утверждения БД в независимо проверяемое доказательство. Именно это закрытие сопротивляется поглощению: тонкая обёртка может сэмитить *похожие* поля, но не может выпустить receipt, чьи access-решения **навязаны алгеброй плана** и чей verdict **байт-в-байт воспроизводим**.

### 2. Что receipt связывает (и что для этого уже есть)

Receipt — это Merkle-коммитмент: каноникализирует и хэширует (BLAKE3) каждое governed-утверждение по ячейке, сворачивает листья в корни, привязывает к `determinism_hash` над каноническими входами и подписывает Ed25519 только маленький фиксированный заголовок.

| Связываемое утверждение | Источник в коде (есть) | Зрелость | Дефект для receipt |
|---|---|---|---|
| Per-cell access decision | `context/mod.rs:108-132` (`ContextAccessDecision{cell_id,decision,policy,scope,scope_id,agent_id}`, `Allowed\|NotRecorded`) | partial | Это пакет-тайм **ре-деривация** через `PolicyRewrite::allows_scope` (`pack/access.rs:14-15`), а не захваченная запись принуждения; нет `policy_version`, нет AgentView-дайджеста |
| Span provenance + citation | `context/mod.rs:134-142`, `span.rs:49-72` (byte/line диапазоны, `source_ref`) | partial | Заполняется только при span-trim переполненной ячейки (`span.rs:32-38`); full-cell паки несут `provenance:None`; citation — свободный текст, не привязан к байтам источника |
| VERIFY verdict + conflicts | `verification/types.rs:60-103` | partial | `VerificationExecutionReport` встраивает wall-clock `total_elapsed_nanos`/`elapsed_nanos` (`operator.rs:28,183,188,203,207`) — **недетерминизм**, обязан быть исключён из хэшируемой поверхности |
| Conflict preservation | `context/conflicts.rs:12-41`, `conflict_visibility_q16` | partial | Группировка по сырой `(project,metric)` строке через `dedup.rs:53-70`; `$1.2M` vs `1,200,000 USD` vs `1.2 million` = ложный 3-сторонний конфликт; покрытие неизмерено |
| Token-budget accounting | `context/mod.rs:147-149` | strong | Привязка есть; нужен `budget_commitment` |
| Fail-closed enforcement | `binder.rs:137-145` (seed `[PushAgentAllowed, PushLive, And]`, WHERE только AND) | strong | Алгебраически расширение прав невозможно; но **нет** подписанной аттестации `BitmapProgram`/AgentView-версии — гарантия в коде, но не проверяема per-answer |

**Чего нет полностью (вся верифицируемость):**
- НЕТ канонической сериализации, НЕТ content-hash, НЕТ подписи над паком/отчётом — `json_export.rs:6-81` использует `json!` с фиксированным порядком ключей, но **не** сортирует рекурсивно и не нормализует числа/строки.
- НОЛЬ настоящих крипто-примитивов во всём workspace (только `roaring`). Каждая «integrity»-поверхность — FNV-1a64 или XOR-FNV: audit-цепочка `event_hash` — **беkey-овый** FNV-1a64 (`audit_chain.rs:8-9,42-65`), коммитит только HTTP-метаданные и СЧЁТЧИКИ; «encrypted backup» — XOR+FNV keystream с подделываемым FNV `auth_tag` (`backup/encrypted/crypto.rs:3-66`), не AEAD, не MAC.
- Идентичность ячейки не аутентифицирована БД: `content_hash` — опциональная **самозаявленная** строка из payload (`cell/descriptor.rs:20`), не вычисленный дайджест.
- НЕТ standalone-верификатора и схемы `accountability_receipt`.

### 3. Структура receipt (`accountability_receipt.v1`)

Receipt добавляется как ОДНО опциональное top-level поле к `context_pack.v1` (схема явно разрешает аддитивные поля до v2, `docs/schemas/context_pack.v1.json:5`), поэтому v1-потребитель его игнорирует.

**Подписываемый заголовок (единственные подписываемые байты, фиксированный размер):**
```
{
  schema_version : "accountability_receipt.v1",
  hash_alg       : "blake3-256",
  sig_alg        : "ed25519",
  db_instance_id, key_id, created_unix_seconds,
  access_root, provenance_root, cell_set_root,
  verification_root, budget_commitment, conflict_commitment,
  pack_root,            // = blake3(access_root||provenance_root||cell_set_root||
                        //          verification_root||budget_commitment||conflict_commitment)
  determinism_hash,     // = blake3(domain||canon(query)||canon(AgentView-проекция)||
                        //          canon(ContextPackOptions)||bitmap_program)
  signature             // Ed25519 над каноническими байтами заголовка
}
```

**Листья (упорядоченные Merkle-деревья; сортировка по `cell_id`, затем фикс-подключ; лист = `blake3(domain_tag_byte || JCS_canonical_bytes(claim))`):**

| Корень | Лист | Что проверяет верификатор |
|---|---|---|
| `access_root` | `{cell_id,decision,policy,scope_id,agent_id,policy_version}` | ОТКЛОНЯЕТ пак, если у любой admitted-ячейки лист `!= allowed` |
| `provenance_root` | `{cell_id,source_cell_id,byte_start,byte_end,line_start,line_end,source_ref,citation,cell_content_hash}` | Цитата лежит внутри байтов источника и присутствует по смещению |
| `cell_set_root` | `{cell_id, cell_content_hash = blake3(canonical cell bytes)}` | Якорь идентичности — **вычислен БД**, НЕ payload-строка |
| `verification_root` | `{status,confidence_q16}` + per-evidence `{cell_id,match_kind,match_score_q16,source_trust_q16}` + per-conflict `{cell_id,metric,left,right}` | Все `elapsed_nanos` ИСКЛЮЧЕНЫ; verdict ссылается только на admitted-ячейки |
| `budget_commitment` | `{token_budget_tokens, estimated_tokens=Σ(cell.estimated_tokens), truncated, per-cell (cell_id,estimated_tokens)}` | Σ ≤ бюджет; флаг `truncated` согласован |
| `conflict_commitment` | `{conflict_visibility_q16, visible_conflict_count, anomalies[]}` | `VisibleConflict` нельзя молча уронить (инвариант GCE 5) |

`pack_root` связывает ВЫХОД, `determinism_hash` связывает ВХОД, подпись над заголовком связывает оба — поэтому «same inputs ⇒ byte-identical pack+receipt» становится внешне проверяемым.

### 4. Модель угроз — что БД (даже злонамеренная/багованная) НЕ должна суметь подделать

| Атака | Ловится шагом верификатора |
|---|---|
| Admit ячейки, которую principal читать не может | `access_root`: нет `allowed`-листа — отказ |
| Цитата span'а, которого нет в байтах источника | пересчёт диапазона + substring + `cell_content_hash` |
| Скрыть `VisibleConflict` / завысить verdict | подписаны `conflict_commitment` и `verification_root` |
| Перерасход / искажение бюджета | `budget_commitment` |
| Заявить детерминизм, вернув другой пак | `determinism_hash` + опциональный re-run |
| Переиспользовать receipt другого запроса | `determinism_hash` связывает `(query, AgentView, options)` под одной подписью |
| Подменить любое поле post-hoc | Ed25519 над корнями |

**Вне области (документируется явно):** эквивокация — БД подписывает два **взаимно-противоречивых, но индивидуально-валидных** receipt'а. Это единственный класс, который подписывающая обёртка тоже не может предотвратить, поэтому именно здесь живёт ров. Митигация — append-only **transparency log** значений `pack_root` (CT-стиль). До его поставки claim «non-absorbable» материально слабее — transparency log должен быть first-class deliverable, а не stub. Receipt доказывает **внутреннюю согласованность**, а НЕ фактическую истинность ячеек.

### 5. Жёсткие предусловия: подписанный-но-неверный receipt = заверенная ложь

Receipt над повреждёнными доказательствами хуже отсутствия receipt'а — он превращает баг в криптографически заверенную ложь. Эти дефекты обязаны быть закрыты ДО сборки receipt:

- **cosine `dot.abs()`** (`hnsw/metric.rs:44`): анти-коррелированные векторы получают идеальный матч; корректная знаковая реализация уже есть в `context/dedup.rs:20-51`. Receipt не может честно аттестовать «наиболее релевантные ячейки» при сломанной метрике.
- **guarded-ANN post-filter** (`search_impl.rs:33-119`, фильтр на :90, бюджет visit на :74-77): sparse-scope (наиболее security-чувствительный) агент выжигает бюджет на out-of-scope узлах; `budget_exceeded` вычислен, но не попадает в `ContextPack` — receipt заверит неполноту как полноту. Минимум: surface как аномалия `RetrievalIncomplete`.
- **(project,metric)-эвристика конфликтов** без нормализации единиц/валют — покрытие конфликтов неизмерено; примитивы `verification/numeric` уже есть, нужна проводка.
- **slot-width** (`cell_ids.rs`: memory 31-bit `0x7fff_ffff` с guard; session/feedback 28-bit `0x0fff_ffff` с молчаливым `&`-усечением): два агента, отличающиеся битами 28-30, коллизируют в один session/feedback cell-id — receipt может привязать ячейку к не тому агенту.
- **XOR-«шифрование» / FNV audit-цепочка** — поверхность подписи/MAC обязана опираться на реальную крипту (BLAKE3/Ed25519/AEAD-Argon2id), иначе receipt «verifiable in name only».

### 6. Exit-гейты (в стиле `make X-check`, вписаны в alpha/release-lane)

| Гейт | Критерий прохождения |
|---|---|
| `make accountability-receipt-schema-check` | `docs/schemas/accountability_receipt.v1.json` заморожена, валидирует golden-fixture, аддитивна к `context_pack.v1` (v1-only потребитель игнорирует поле) |
| `make accountability-receipt-determinism-check` | Два прогона `RETRIEVE CONTEXT` + `VERIFY FACT` на фикс-сторе ⇒ байт-идентичные receipt'ы, включая байт-идентичные Ed25519-подписи (RFC 8032 deterministic nonces); ни одного `elapsed_nanos`/wall-clock байта в хэшируемой поверхности (field-exclusion тест) |
| `make accountability-receipt-tamper-check` | Табличный mutation-suite (flip `estimated_tokens`; `Allowed→NotRecorded`; сдвиг `source_byte_start`; drop `VisibleConflict`; swap verdict; replay под другим query/AgentView; flip байта подписи) — верификатор ОТКЛОНЯЕТ каждый случай и принимает unmutated golden |
| `make accountability-receipt-verify-check` | Standalone `cortex-receipt-verify` при входах ТОЛЬКО `{pack JSON + receipt + raw bytes/content-hashes admitted-ячеек + public key}` валидирует 100% фикстур; dependency-graph-ассерт подтверждает, что бинарь **не** линкует `cortex-engine`/`cortex-storage`/`cortex-aql` |
| `make accountability-receipt-check` | Зонтичный = четыре под-гейта + grep-гейт «ни один FNV-1a64/XOR-FNV не подпирает лист/корень/подпись/`determinism_hash`»; `cargo tree` подтверждает ровно один hash-crate (blake3/sha2) + один sig-crate (ed25519-dalek), за cargo feature-флагом для первого релиза |

### 7. Зависимости и связь с остальными столпами

`canonical_bytes()` (JCS/RFC 8785 или явная integer-only байт-форма, рекурсивная сортировка ключей, без float/timestamp, domain-tags) — **единственная точка согласия** между БД и верификатором, гейтит корни, `determinism_hash` И no-engine-link верификатор. Её обязан владеть один столп (reproducibility OWNS, receipt CONSUMES), иначе два расходящихся хэш-схемы молча форкнут БД и верификатор. Это под-задача, недооценённая своим тегом «M»: ей нужны cross-language golden vectors и замороженный нормативный документ **до того**, как что-либо хэшируется.

Receipt — **load-bearing артефакт** северной звезды. Fail-closed (binder) становится подотчётным через `access_root`; conflict preservation — через `conflict_commitment`; budget accounting — через `budget_commitment`. Без receipt'а эти свойства остаются самозаявленными аннотациями БД, которые мимикрирует обёртка `pgvector+Cerbos+RAGAS`. С receipt'ом воспроизведение receipt'а требует переписать весь governed детерминированный движок — это и есть absorption-resistant closure.

**Усиление аргумента (критично):** самый сильный non-absorbable claim — НЕ «мы эмитим подписанный receipt» (воспроизводимо обёрткой), а «`access_root` привязан к **реально исполненному** plan-algebra принуждению, проверяемо повторным прогоном связанного `bitmap_program` против цитированных ячеек». Проверка доступа в верификаторе должна **требовать переоценки связанной программы**, а не доверять листу `allowed`. Тогда обёртке недостаточно сэмитить receipt — ей придётся реализовать fail-closed алгебру плана и привязать исполненную программу. Это, плюс transparency log против эквивокации, и есть настоящий ров.

## 2. Текущее состояние

Северная звезда — «подотчётность ответа» как непоглощаемый контракт — требует, чтобы каждый `ContextPack`/`VERIFY` нёс **криптографически проверяемую квитанцию** (accountability receipt), которую независимая третья сторона проверяет **без доверия к БД**. Оценим, насколько кодовая база v0.2.0-beta.2 к этому готова: что из **семантических входов** квитанции уже есть и где зияет **слой верифицируемости**.

Краткий вердикт: семантический субстрат — от частичного до сильного; слой верифицируемости (каноническая сериализация, хэш, подпись, оффлайн-верификатор) — **отсутствует почти полностью**. Плюс четыре подтверждённых дефекта корректности, которые подписанная квитанция превратила бы из багов в **нотариально заверенную ложь**.

### Карта готовности: входы квитанции против кода

| Что квитанция должна связать | Статус | Где в коде | Разрыв (по делу) |
|---|---|---|---|
| Per-cell access_decision (решение области/RBAC, допустившее ячейку) | partial | `context/pack/access.rs:8-46`; типы `context/mod.rs:108-132`; экспорт `context/export/json_export.rs:25-33` | Это **пересчёт на этапе сборки пака** через `PolicyRewrite::allows_scope` (`access.rs:14`), а не захваченная запись того, что реально сделал биндер/скан. Может расходиться с фактическим исполнением. Негативный исход — расплывчатый `NotRecorded` без переоцениваемой причины отказа и без версии политики. Не связано ни с каким хэшем — самозаверенная аннотация. |
| Цитаты + span-провенанс (байтовый/строчный диапазон к источнику) | partial | `context/span.rs:29-40,49-72`; тип `context/mod.rs:134-142`; экспорт `json_export.rs:17-24,83-94` | Провенанс как данные — сильный. НО заполняется **только** когда `span_level_packing` обрезает пак, превысивший бюджет (`span.rs:32-38`); полноячеечные паки получают `provenance: None`. Цитата — свободный текст из метаданных, **никогда не валидируется** против `source_ref` и не привязана к дайджесту байтов источника. |
| VERIFY FACT: вердикт + конфликты (Supported/Contradicted/Mixed/Insufficient + numeric_conflicts) | partial | оператор `verification/operator.rs:31-190`; тип `verification/types.rs:60-103`; ключ конфликтов `verification/numeric/fact_claim.rs` | Логика детерминирована при фиксированном сторе. НО отчёт об исполнении встраивает wall-clock `total_elapsed_nanos`/`elapsed_nanos` (`operator.rs:28,183,207`) — **недетерминированно, обязано быть исключено** из любой хэшируемой поверхности. Детекция конфликтов — единичная эвристика `(scope, metric, project)` без конверсии единиц/валют; conflict recall не измерен. |
| ground_answer (пост-ответный grounding-страж) | partial | `context/grounding.rs:8-25,56-142` | Детерминирован, но поддержка — чистое пересечение мешков слов (`grounding.rs:101-129`), без отрицания/энтейлмента: «бюджет НЕ 1.2B» «поддержан» ячейкой «бюджет 1.2B». Отчёт **не входит** в `ContextPack`, **нет** в `json_export`, **не привязан** к хэшу. Библиотечный страж, который вызывающая сторона может молча проигнорировать. |
| Fail-closed биндер (алгебра плана) | **strong** | `cortex-aql/src/binder.rs:137-145` | Каждый retrieve-план засеян `[PushAgentAllowed, PushLive, And]`, а `WHERE` только **AND-ится** сверху — расширение области невозможно по алгебре плана. Это самый сильный примитив подотчётности. Разрыв — чисто в квитанции: биндер **не выдаёт подписанной/хэшированной аттестации** битмап-программы или версии AgentView/политики, которую он исполнил. |
| Детерминизм (same inputs ⇒ byte-identical pack+receipt) | **missing** | тесты `crates/cortex-engine/tests/determinism.rs`; гейт `engine-determinism-check` (mk/core-contracts.mk:95) | Есть только in-repo снапшот-тесты повторяемости. **Нет** экспортируемого determinism-хэша, **нет** канонического сериализатора, **нет** кросс-процессного артефакта. `json!` (`json_export.rs:68-81`) опускает часы (хорошо), но **не сортирует ключи рекурсивно** и не нормализует числа/строки — это не квитанция, а удобная строка. |
| Целостность аудита (хэш-цепочка) | **weak** | `cortex-server/src/audit_chain.rs:8-9,42-65` | `event_hash` — **неключевой FNV-1a64** (`format!("{hash:016x}")`), не MAC. Любой, кто читает лог, пересчитывает каждый хэш и перезаписывает историю незаметно (ключа нет нигде). Хуже: фиксирует только HTTP-конверт + **счётчики** (method, path, status, principal, scope_decision, cell/citation count) — **никогда** сами `cell_id`, цитаты, вердикт или хэш пака. Доказывает, что эндпоинт вызвали, а не **что** он вернул. |
| Шифрование at-rest (бэкап) | **weak** | `backup/encrypted/crypto.rs:1-5,33-45`; codec `codec.rs` | Cipher suite `cortexdb.xor-fnv64-stream.v1`: keystream = FNV(passphrase,nonce,counter) XOR — повторяющееся 8-байтовое слово на каждые 8 байт, тривиально ломается известным открытым текстом. «KDF» = один проход FNV (без соли/итераций). `auth_tag` (`crypto.rs:33-45`) = FNV(passphrase, nonce, FNV(plaintext), FNV(ciphertext)) — **не AEAD, не MAC, подделываем** любым, у кого есть passphrase. |
| Guarded/filtered ANN для sparse-scope агентов | **weak** | `search/hnsw/search_impl.rs:33-119`; косинус `search/hnsw/metric.rs:44` | Allowed-set применяется как **пост-фильтр** под общим бюджетом обхода: узкоскоупный (самый чувствительный к безопасности) агент исчерпывает `max_visited` на чужих узлах и возвращает мало/ноль своих → recall молча деградирует. `budget_exceeded` не выводится в `ContextPack` как аномалия — ответ не может раскрыть, что он, возможно, неполон. |
| Идентичность ячейки (DB-аутентифицированная) | **missing** | `cortex-core/src/cell/descriptor.rs:20`; `query/metadata/types.rs:31` | `content_hash` — **опциональная самозаверённая строка** из payload, не вычисленный дайджест. Квитанция не может ей доверять — нужен DB-computed `cell_content_hash = H(canonical cell bytes)`. |
| Реальные крипто-примитивы во всём workspace | **missing** | grep по всем `crates/*/Cargo.toml` и корню | **Ноль** крипто-зависимостей. Присутствует только `roaring` (`cortex-aql/Cargo.toml:10`, `cortex-storage/Cargo.toml:10`). Нет `sha2`/`blake3`/`ed25519`/`hmac`/`chacha20poly1305`/`aes-gcm`/`argon2`. Каждая «integrity»/«crypto» поверхность — самопальный FNV или XOR-FNV. |

### Структура того, что есть сегодня против того, что нужно

Сегодня ответ — это набор **самозаверённых аннотаций**, физически несвязанных и неподписанных:

```
ContextPack (json_export.rs, schema "context_pack.v1")
├── cells[]
│   ├── payload, citation (свободный текст, не валидирован)
│   ├── access_decision   ← пересчёт allows_scope, не захват (access.rs)
│   └── provenance: Option ← None для полноячеечных паков (span.rs:32-38)
├── conflict_visibility_q16 / visible_conflict_count
├── anomalies[] (только TokenOverload реально пушится)
└── token budget accounting
        ⇩ нет канонизации, нет хэша, нет подписи
VerificationReport (types.rs) — отдельно, + elapsed_nanos (операторы)
AuditRecord (audit.rs) — отдельно, FNV-цепочка только над счётчиками
```

Целевая квитанция должна выглядеть так (для контраста — ничего из этого в коде нет):

```
accountability_receipt.v1  (подписанный фикс-размерный заголовок)
{ schema_version, hash_alg:"blake3-256", sig_alg:"ed25519",
  db_instance_id, key_id, created_unix_seconds,
  access_root,        ← Merkle над ContextAccessDecision (захваченными)
  provenance_root,    ← Merkle над span+citation+cell_content_hash
  cell_set_root,      ← Merkle над DB-computed cell_content_hash
  verification_root,  ← вердикт+конфликты, БЕЗ elapsed_nanos
  budget_commitment,  ← Σ estimated_tokens ≤ token_budget, truncated
  conflict_commitment,← conflict_visibility, visible_conflict_count
  pack_root,          ← blake3(все шесть выше) — связывает ВЫХОД
  determinism_hash,   ← blake3(query ‖ AgentView ‖ options ‖ bitmap_program) — ВХОД
  signature }         ← Ed25519 только над заголовком
```

Сегодня нет ни одного из этих корней, ни canonical-bytes модуля, ни `cell_content_hash`, ни подписи, ни оффлайн-верификатора (`cortex-cli` содержит `cli_audit_chain.rs`, но **нет** `cortex-receipt-verify` и **нет** схемы `accountability_receipt`).

### Четыре дефекта корректности, отравляющие любую квитанцию

Подпись над испорченным доказательством — хуже, чем отсутствие подписи: баг становится нотариально заверенной ложью. Все четыре подтверждены в файле:

1. **Косинус через `dot.abs()`** (`search/hnsw/metric.rs:44`): `Some(((dot.abs() * 65_535) / norm.abs()) as u64)` — антикоррелированные векторы (`v` против `-v`) получают **идеальный матч**, а промежуточное `dot.abs() * 65_535` (i64) может переполниться на высокоразмерных i16-векторах. Корректная реализация **уже есть** в том же крейте — `context/dedup.rs:20-51` (`cosine_similarity_q16`: возвращает 0 при `dot ≤ 0`, расширение до i128/u128). Две реализации **расходятся** — латентная несогласованность.

2. **Эвристика конфликтов без нормализации** (`context/conflicts.rs:12-41`): группировка по lowercased `(project, metric)`, флаг при `values.len() > 1` над **сырыми строками**. `$1.2M` против `1,200,000 USD` против `1.2 million` дают **ложный 3-way конфликт**; форматно-равные истинные конфликты в любом не-`key=value` payload — **ноль конфликтов** (экстрактор `dedup.rs:53-70` матчит только литеральные префиксы строк). При этом модуль нормализации **уже существует и не подключён** (`verification/numeric/parse.rs`, `value.rs`: `parse_currency_code`, `parse_unit_code`, `parse_magnitude_suffix`, `NumericValue::normalized_eq`). Фикс — это в основном проводка.

3. **Коллизия slot-width в cell-id**: память — 31-битный слот `MEMORY_AGENT_SLOT_MASK = 0x7fff_ffff` с **охранным** конструктором `memory_cell_id` (`cell_ids.rs:6,13-20`, возвращает `None` при переполнении). А сессия (`session.rs:156`) и фидбэк (`feedback.rs:83`) маскируют агента на **28 бит** — `(agent_id.0 & 0x0fff_ffff) << 32` — и **молча усекают**. Два агента, отличающиеся битами 28-30, дают **одинаковый** session/feedback cell-id: квитанция может привязать ячейку к **не тому агенту**.

4. **Guarded-ANN пост-фильтр** (`search/hnsw/search_impl.rs:33-119`, см. таблицу): деградация recall для sparse-scope, скрытая (нет аномалии неполноты в паке). Подписанная квитанция заверила бы **неполный** контекст как полный.

### Вывод для последовательности работ

Семантические входы (`access_decision`, `provenance`, `VERIFY`, `grounding`, conflict-visibility, token accounting) и **fail-closed биндер** уже дают подотчётности «по конструкции» внутри БД. Чего нет — это **замыкание верифицируемости**: ничто не канонизирует, не хэширует, не подписывает и не делает результат проверяемым извне. Поэтому до постройки квитанции необходимы два предусловия: (а) устранить четыре дефекта корректности (или вывести их неполноту как **хэшируемую аномалию**), иначе квитанция нотарит ложь; (б) ввести первые реальные крипто-примитивы (`blake3`/`sha2` + `ed25519-dalek`), вытеснив FNV из `audit_chain.rs` и XOR-FNV из `backup/encrypted/crypto.rs`. Канонический `canonical_bytes()` (исключающий `elapsed_nanos`) — единая точка согласия БД и верификатора — должен быть специфицирован **один раз и рано**, до сборки любого Merkle-корня.

## 3. Столпы развития

Этот раздел фиксирует **критический путь** к северной звезде — «подотчётность ответа» (answer accountability) как непоглощаемый контракт. Тезис строгий: подотчётность реализована тогда и только тогда, когда каждый `ContextPack`/`VERIFY`-ответ несёт **криптографически проверяемую квитанцию (accountability receipt)**, которую независимая третья сторона проверяет **без доверия к самой БД**. Всё остальное — это либо вход в эту квитанцию, либо предпосылка её честности.

Семь столпов, упорядоченных по доминирующему критическому пути. Принцип последовательности один: **доверенные улики → реальная криптография → каноническая сериализация → квитанция → независимый верификатор → доказательство категории → продакшн-масштаб.** Подписанная квитанция над испорченными уликами — это «нотариально заверенная ложь» (signed-but-wrong receipt), поэтому MUST-FIX-дефекты строго предшествуют подписи.

### Карта столпов

| # | Столп | Категориеобразующий | Версия-вешка | Роль в северной звезде |
|---|-------|:---:|---|---|
| P0 | Каноническая сериализация + детерминизм (фундамент) | косвенно | beta.3 | Единая точка согласия DB↔verifier; гейт всего хешируемого |
| P1 | Correctness prerequisites (must-fix баги) | нет | beta.4 | Делает улики истинными до нотаризации |
| P2 | Реальная криптография (замена обфускации) | да | beta.4 ∥ | Первый hash/signature в воркспейсе; основа подписи/MAC/AEAD |
| P3 | Verifiable Accountability Receipt | **да (определяющий артефакт)** | 0.3 | Сам артефакт северной звезды |
| P4 | Provable fail-closed governance end-to-end | да | 0.3 | Делает `access_root` честным на физическом слое |
| P5 | Deterministic verification at strength | да | 0.4 | Покрытие конфликтов, которое квитанция подписывает |
| P6 | Reproducibility + frozen learned ranker | да | 0.4 | Делает `determinism_hash` нагруженным |
| P7 | Absorption proof + open GCE spec + production scale | да | 0.5 / 1.0 | Превращает контракт в публично проверяемую категорию |

> **Жёсткое правило координации:** `canonical_bytes()` и `determinism_hash` принадлежат P0/P6 (владелец), а P3 их **потребляет**. Запрещено иметь две схемы хеширования — это молчаливо форкнет байты БД и верификатора (ровно тот провал, от которого квитанция должна защищать).

---

### P0 — Каноническая сериализация + детерминизм (pillar-zero)

**Почему отдельно.** В исходном плане `canonical_bytes` (AR-1), исключение `elapsed_nanos` (DV6) и `REPRO-1` разбросаны по трём столпам. Будучи построены независимо, они **разойдутся**. Это зависимость минимум четырёх столпов, поэтому выносится в фундамент. Подтверждено: `verification/operator.rs` встраивает `Instant::now()`/`total_elapsed_nanos`/`elapsed_nanos` — это недетерминизм, который ОБЯЗАН быть исключён из хешируемой поверхности.

| Задача | Детали | exit-гейт |
|---|---|---|
| **P0-1** Каноническая поверхность | Нормативный `canonical_bytes()` (RFC 8785 JCS или явная байт-форма: только integer/Q16, рекурсивная сортировка ключей, без float/timestamp, domain-tag на лист) для `ContextPack` и `VerificationReport`. Не переиспользовать `json!` из `json_export.rs` — он не сортирует ключи рекурсивно. | `make canonical-serialization-check`: property-тест — `canonical_bytes` инвариантен к перестановке порядка вставки ключей; в выводе нет ни одного байта timestamp/`elapsed_nanos` |
| **P0-2** Исключение часов | Перенести `total_elapsed_nanos`/`elapsed_nanos` (operator.rs:28,188,203) в неподписываемый телеметрический side-channel. | field-exclusion grep-тест: хешируемая поверхность не содержит `SystemTime`/`elapsed_nanos` |
| **P0-3** Field-classification allowlist | Явный allowlist «хешируется / не хешируется»; добавление поля в `ContextPack`/`VerificationReport` без классификации **роняет** тест. | тест падает на неклассифицированном новом поле |

**Зависимости:** нет (стартует в день ноль). **Блокирует:** P3 (AR-4), P5 (DV6), P6 (REPRO), P7 (replica-invariance).
**Вешка:** `canonical_bytes()` байт-стабилен под key-permutation, все wall-clock-поля доказуемо исключены; криптография ещё не нужна (допустим явно именованный non-crypto content hash `cortexdb.determinism.contenthash.noncrypto.v0`).

---

### P1 — Correctness prerequisites (must-fix баги)

**Северная звезда:** не дифференциатор сам по себе, но **строгая предпосылка**. Если cosine ранжирует анти-коррелированные векторы как идеальные совпадения, квитанция заверяет испорченный сигнал релевантности; если `feedback`/`session` cell-id коллидируют, квитанция привязывает ячейку к **чужому агенту**; если детекция конфликтов даёт ложные/пропущенные конфликты, заверенный набор `VisibleConflict` (GCE-инвариант 5) — фикция.

Все дефекты подтверждены в исходниках:

| Задача | Дефект (file:line) | Исправление | exit-гейт |
|---|---|---|---|
| **CP-1** | cosine `dot.abs()` (`hnsw/metric.rs:44`): `Some(((dot.abs() * 65_535) / norm.abs()) as u64)` — анти-коррелированные = идеальное совпадение; `dot.abs()*65_535` переполняется на i64. | Взять проверенный знак-клэмп из `context/dedup.rs:20-51` (return 0 при `dot<=0`, i128/u128-расширение). | `cosine-metric-correctness-check`: `cosine(v,-v)==0`, `cosine(v,v)==max`, порядок `{v, ⟂, -v}` = v→…→-v, нет overflow на high-dim |
| **CP-2** | Две расходящиеся реализации cosine (`metric.rs` vs `dedup.rs`) — латентная несогласованность. | Единый источник истины (общий `cosine_similarity_q16`) или тест на байт-идентичный Q16. | тест: обе реализации дают идентичный Q16 на общем фикстуре |
| **CP-3** | Slot-width коллизия: memory — 31-бит `0x7fff_ffff` с guarded-конструктором; **feedback** (`feedback.rs:83` `agent_id.0 & 0x0fff_ffff`) и session (`session.rs:156,164`) — 28-бит, feedback **молча `&`-усекает** без guard. Два агента, различающиеся битами 28–30, дают один cell-id. | Единая документированная ширина (31 бит) через общий `cell_ids`-хелпер, который **возвращает `None`/ошибку**, а не усекает. За schema-version/migration-гейтом. | `cell-id-collision-check`: различные `(agent_id,sequence)` не коллидируют в memory/session/feedback; over-width → `None` |
| **CP-4** | `(project,metric)`-эвристика: `context/conflicts.rs:12-41` группирует по сырой строке, экстрактор `dedup.rs:53-70` ловит только литералы `project=`/`metric=`/`value=`. `$1.2M` vs `1,200,000 USD` vs `1.2 million` = ложный 3-сторонний конфликт. | **Заводка уже существующего** `verification/numeric` (`parse_currency_code`, `parse_unit_code`, `parse_magnitude_suffix`, `NumericValue::normalized_eq`/`conflicts_with`) в `conflicts.rs`; string-fallback только для нечисловых. Целое/Q16. | `conflict-normalization-check`: нормализованно-равные не флагуются, истинные конфликты флагуются на ≥2 форматах; recall/precision в отчёте |
| **CP-5** | guarded-ANN — post-filter под общим visit-budget (`search_impl.rs:90`, cutoff:74-77); `budget_exceeded` вычисляется, но **не выносится** в `ContextPack`. | Новый `ContextPackAnomalyCode::RetrievalIncomplete`, проброс флага в pack + `json_export`. Это дешёвая «честная» половина guarded-ANN-проблемы. | `ann-budget-disclosure-check`: при исчерпании бюджета pack несёт аномалию неполноты, она проходит через все export-форматы |
| **CP-6** | — | Агрегат `correctness-prerequisites-check`, проводка в beta-lane по конвенции `mk/*.mk` (cargo test + `scripts/*_check.py --report`). | `make correctness-prerequisites-check` зелёный в release-lane |

**Зависимости:** P0 (для DV6-части детерминизма). Параллелен P2.
**Вешка:** все улики, которые квитанция свяжет (ранжирование, набор конфликтов, идентичность ячейки, флаг неполноты), корректны/захвачены/раскрыты.

> ⚠️ **Жёсткий гейт:** `correctness-prerequisites-check` зелёный **до** того, как сборка квитанции (AR-4) допущена к merge. MUST-FIX блокирует эмиссию квитанции, а не «едет параллельно в релиз».

---

### P2 — Реальная криптография (замена обфускации)

**Текущее состояние:** в воркспейсе **ноль** криптопримитивов (`grep` по всем `Cargo.toml` — только `roaring`). Каждая «integrity/encryption»-поверхность — самописный FNV-1a64 или XOR-FNV:
- **Бэкап** (`backup/encrypted/crypto.rs`): cipher suite `cortexdb.xor-fnv64-stream.v1` — keystream `FNV(passphrase,nonce,counter) ⊕ plaintext` (повторяющееся 8-байтное слово), `KDF` = один проход FNV (без соли/итераций), `auth_tag` = FNV — **подделываемый**, не AEAD, не MAC.
- **Audit chain** (`audit_chain.rs:8-9,42-48`): беключевой FNV-1a64; коммитит только HTTP-метаданные и **счётчики** (не cell_ids, не verdict, не pack hash). Любой, кто может писать лог, переписывает историю незаметно.

| Задача | Детали | exit-гейт |
|---|---|---|
| **CRY-1** | Добавить аудированные RustCrypto-крейты (`blake3`/`sha2`, `ed25519-dalek`, `chacha20poly1305`, `argon2`, `getrandom`, `zeroize`, `subtle`) под feature-флаг `accountability-receipt`. NB: нет `[workspace.dependencies]` — объявлять по-крейтно. | `crypto-deps-policy-check`: нужные крейты на месте; grep по `crates/*/src` (без tests/benches) даёт **ноль** FNV/XOR-routine в backup/audit-путях |
| **CRY-2** | Единый `cortex-crypto`-модуль: hash + AEAD (XChaCha20-Poly1305) + KDF (Argon2id) + MAC (HMAC-SHA-256) + sign/verify (Ed25519, детерминированные RFC-8032 nonces) + KeyId + zeroize. Одна точка вызова для квитанции, audit и backup. | `crypto-primitives-check`: KAT-векторы совпадают, constant-time compare, round-trip sign/verify |
| **CRY-3** | Бэкап → XChaCha20-Poly1305 + Argon2id (формат v2, `cortexdb.xchacha20poly1305-argon2id.v2`), AAD = заголовок; v1 XOR-FNV **отказ при чтении** (typed error). Удалить `apply_keystream`/`auth_tag`. | `encrypted-backup-check` (round-trip; tamper любого байта ct/tag/nonce/salt/AAD → ошибка, ноль утечки plaintext) + `encrypted-backup-legacy-refuse-check` |
| **CRY-4** | Audit chain → SHA-256 + HMAC-SHA-256 (или Ed25519-checkpoints); общий модуль writer (server) и verifier (cli), без дублирования FNV. v2-строка + отказ/верификация v1. | `audit-chain-check`: tamper прошлого события → fail; без MAC-ключа подделка невозможна; cli верифицирует server-written chain |
| **CRY-5** | Коммит per-answer receipt hash в `AuditRecord` (привязка к P3). | `audit-receipt-binding-check`: запись с несовпадающим receipt hash отвергается |
| **CRY-6** | Node keystore: keypair + key_id + источник (file/env, не логируется) + ротация (key_id bump + dual-trust window + re-anchor) + экспорт публичного ключа. | `key-management-check`: ротация сохраняет историческую верификацию; `secrets-check` — ключи не попадают в лог |
| **CRY-7** | Честные claims: `SECURITY_MODEL.md:77`, `BACKUP_RESTORE.md` называют реальный cipher/KDF/AEAD; убрать «XOR-FNV как encryption». | `crypto-claims-honesty-check` (doc-lint) |
| **CRY-8** | Агрегат `crypto-foundation-check` → `security-gate-v2-check`. | оба гейта зелёные end-to-end |

**Зависимости:** независим от P1 (параллелен); внешняя — allowlist/vendoring RustCrypto + энтропия `getrandom` в CI-песочнице.
**Вешка:** ни одна integrity/confidentiality-поверхность не на FNV/XOR; at-rest и audit покоятся на AEAD/MAC, связанных в квитанцию.

---

### P3 — Verifiable Accountability Receipt (артефакт северной звезды)

**Это и есть несущий артефакт.** Дополнительное необязательное верхнеуровневое поле `accountability_receipt` (схема `accountability_receipt.v1`) на `context_pack.v1` (раздел honored: «additive optional fields allowed until v2», `docs/schemas/context_pack.v1.json:5`). Это закрытие, которое тонкая обёртка pgvector+policy **структурно не может** воспроизвести.

**Структура квитанции** (подписывается только фиксированный заголовок; всё остальное — детерминированно перевыводимо верификатором):

```
accountability_receipt.v1 {
  header (ЕДИНСТВЕННЫЕ подписываемые байты, фикс. размер):
    schema_version, hash_alg="blake3-256", sig_alg="ed25519",
    db_instance_id, key_id, created_unix_seconds,
    access_root, provenance_root, cell_set_root,
    verification_root, budget_commitment, conflict_commitment,
    pack_root, determinism_hash, signature

  Merkle-листья (отсортированы по cell_id, листок = blake3(domain_tag || JCS_bytes(claim))):
    ACCESS[cell]       {cell_id, decision, policy, scope_id, agent_id, policy_version}
    PROVENANCE[cell]   {cell_id, source_cell_id, byte/line range, source_ref, citation, cell_content_hash}
    CELL_SET[cell]     {cell_id, cell_content_hash = blake3(canonical cell bytes)}   // DB-computed, НЕ payload-string
    VERIFICATION       {status, confidence_q16} + per-evidence + per-conflict  // БЕЗ elapsed_nanos
    BUDGET             {token_budget, Σ estimated_tokens, truncated, per-cell costs}
    CONFLICT           {conflict_visibility_q16, visible_conflict_count, anomalies[]}

  pack_root        = blake3(access||provenance||cell_set||verification||budget||conflict)   // СВЯЗЫВАЕТ ВЫХОД
  determinism_hash = blake3(query || AgentView-проекция || ContextPackOptions || bitmap_program)  // СВЯЗЫВАЕТ ВХОД
}
```

| Задача | Детали | exit-гейт |
|---|---|---|
| **AR-2** | Заморозить `docs/schemas/accountability_receipt.v1.json` + golden-fixture + spec. Заморозка **после** P1 и AR-6 (иначе `policy_version`/`RetrievalIncomplete`/temporal-conflict вынудят v2-bump). | `accountability-receipt-schema-check`: golden валиден, поле аддитивно (v1-only consumer его игнорирует) |
| **AR-3** | DB-computed `cell_content_hash = blake3(canonical cell bytes)`, НЕ self-asserted `content_hash` из payload (`descriptor.rs:20`). | `accountability-cell-hash-check`: детерминирован, меняется на 1 байт, независим от payload-string |
| **AR-4** | Тело: пять Merkle-деревьев + `pack_root` + `determinism_hash` через P0-канонизатор. Промоут grounding-report и `budget_exceeded` в хешируемую поверхность. | `accountability-receipt-determinism-check`: два прогона RETRIEVE+VERIFY на фикс. store → байт-идентичные тела (все roots равны) |
| **AR-5** | Ed25519-подпись заголовка (детерминированные nonces ⇒ байт-идентичная подпись), потребляет keystore CRY-6. | `accountability-receipt-sign-check`: same key+inputs → байт-идентичная подпись; rotated key_id детектируется |
| **AR-6** | **Захваченное** access-решение из binder/scan-пути (с `policy_version` + AgentView-digest), а не пересчёт `allows_scope` в `context/pack/access.rs:8-46`. Исключённые ячейки сворачиваются в denied-set. | `context-access-decision-capture-check`: каждая допущенная ячейка — `Allowed` с непустым `policy_version`; ни одной `NotRecorded` на успешном AQL-пути |
| **AR-7** | Standalone `cortex-receipt-verify` (свой крейт, **не линкует** cortex-engine/server/aql — assert по dependency-graph). 7-шаговый алгоритм только из публичных входов. | `accountability-receipt-verify-check`: 100% golden-фикстур приняты; dependency-closure без engine |
| **AR-8** | Table-driven tamper-suite + агрегат. | `accountability-receipt-tamper-check`: 100% отказ на мутациях, приём genuine |

**Алгоритм верификатора (без доверия к БД; входы = pack JSON + receipt + сырые байты допущенных ячеек + публичный ключ):** (1) Ed25519 над каноническим заголовком; (2) перевывести все листья/roots из JSON, свернуть к подписанным roots; (3) **отвергнуть**, если у любой допущенной ячейки access-листок ≠ `allowed`; (4) для каждого citation — `0 ≤ byte_start ≤ byte_end ≤ len(cell)` И подстрока присутствует по смещению; (5) `Σ estimated_tokens ≤ token_budget`; (6) verdict/конфликты ссылаются только на допущенные cell_ids; (7) перевывести `determinism_hash` (при возможности перезапуска — требовать байт-идентичность).

**Модель угроз (что БД НЕ может подделать, каждое ловится конкретным шагом):** допустить нечитаемую ячейку; процитировать спан вне байтов; уронить/скрыть `VisibleConflict` или завысить verdict; перерасходовать бюджет; заявить детерминизм при ином pack; переиграть чужую квитанцию; подправить любое поле post-hoc.

**Вне области (документировать явно):** **эквивокация** — БД подписывает две противоречивые-но-индивидуально-валидные квитанции. Это единственный класс, который подписывающая обёртка тоже не остановит, поэтому именно здесь живёт ров — митигируется append-only **transparency log** для `pack_root` (P7/SCALE-3). Квитанция доказывает внутреннюю согласованность, НЕ фактическую истинность ячеек.

**Зависимости:** P0 (канонизатор), P1 (честные улики), P2 (hash/sign), P4 (честный `access_root`).

---

### P4 — Provable fail-closed governance end-to-end

**Северная звезда:** fail-closed — **единственный по-настоящему непоглощаемый примитив**. Binder сеет каждый план `[PushAgentAllowed, PushLive, And]` и только AND-ит WHERE (`binder.rs:137-145`), так что расширение области невозможно **по алгебре плана** — сильнее RLS и сильнее Cerbos/OpenFGA, которые лишь *рекомендуют* фильтр, применяемый приложением вне БД. Но `access_root` честен только если гарантия держится **через физические сканы и ANN**, а не только на binder.

**Подтверждённое расхождение:** персистентный ANN/lexical-путь строит `allowed` из `search/access.rs:9-17` `allowed_candidates`, который объединяет **только `readable_scopes()`** — НЕ компонует `PushLive` (status) и WHERE-сужение. Два пути enforcement не доказуемо равны.

| Задача | Детали | exit-гейт |
|---|---|---|
| **FC-2** | Выводить персистентный `allowed` из связанной `bitmap_program` (live + WHERE), а не из `readable_scopes` в одиночку. | `ann-scope-parity-check`: `allowed == eval_bitmap_program(plan) ∩ vector_cells`; кейс, где `live=false`/WHERE-исключённая, но readable ячейка теперь исключена |
| **FC-3** | Убрать recall-коллапс для sparse-scope: exact top-k fallback при малом `|allowed|` (именно sparse-кейс) и/или независимый бюджет для allowed-обхода. | `ann-sparse-scope-recall-check`: recall@k самой разреженной децили ≥ exact − ε |
| **FC-6** | Scope-leak-бенч по **каждой** выходной поверхности (payload, citations, `source_ref`, explain, anomalies, VERIFY-evidence, `EngineError.safe_message`) с уникальным sentinel. | `scope-leak-bench-check`: 0 sentinel на ≥200 комбинациях (agent×query×format×persistence×budget), до/после checkpoint+compact |
| **FC-7** | Машинно-проверяемая модель инварианта (proptest): `admitted ⊆ agent_allowed ∩ live ∩ where` для обоих путей; экспорт стабильного `model_hash`, связанного в `access_root`. | `fail-closed-invariant-model-check`: ноль контрпримеров; pinned `model_hash` |
| **FC-8** | Агрегат в release-lane. | `fail-closed-end-to-end-check` зелёный |

**Зависимости:** P3 (потребляет `access_root`/denied-set/`model_hash`/`RetrievalIncomplete`); делит cosine-fix с P1.
**Вешка:** «ни байта вне области, измеримо, end-to-end»; `access_root` заверяет реально случившийся enforcement, а не pack-time-пересчёт.

> **Острый тезис (усиление непоглощаемости):** сильнейшее утверждение — НЕ «мы эмитим подписанную квитанцию» (воспроизводимо обёрткой), а «`access_root` привязан к **фактически исполненной** алгебре плана, проверяемой перезапуском связанной `bitmap_program` против цитируемых ячеек». Поэтому **верификатор должен пере-вычислять связанную программу**, а не доверять листку `allowed`. Это превращает FC-2+AR-6+FC-7 в подлинное закрытие.

---

### P5 — Deterministic verification at strength

**Северная звезда:** подписанный verdict стоит ровно своего **покрытия конфликтов**. Квитанция, заверяющая «конфликтов нет» над single-`(project,metric)`-эвристикой без конвертации единиц/валют и без temporal/citation-конфликтов — нотаризованное слепое пятно.

| Задача | Детали | exit-гейт |
|---|---|---|
| **DV1** | Унифицировать ContextPack-видимость конфликтов на VERIFY-нормализаторе (один набор конфликтов для квитанции). | `context-pack-conflict-visibility-check` (расш.): cross-path agreement |
| **DV2** | Целочисленная конвертация единиц/валют в `value.rs:64-94`: класс длины (m↔km↔cm), массы (kg↔g), времени (h↔min↔s) + magnitude B/M/K; cross-class = `Incomparable`; cross-currency = `Conflict` (без выдуманного FX), но **помечен** отдельно. u128-промежутки против overflow. | `verify-numeric-normalization-check`: `60min==1h`, `1h vs 2h` конфликт, нет f32/f64 |
| **DV3** | Multi-value extraction (заменить `single_numeric_value`, `fact_claim.rs:390-394`, который дропает ячейки с >1 числом). | `verify-multivalue-extraction-check` |
| **DV4** | Дат. факты больше не short-circuit-ят numeric-детекцию (`fact_claim.rs:112,172-175`); «то же `(project,metric)`, два значения на одну дату» = temporal-конфликт. | `verify-temporal-conflict-check` |
| **DV5** | Citation-конфликт: две ячейки цитируют один `source_ref`, но числа расходятся. | `verify-citation-conflict-check` |
| **DV6** | Исключить `elapsed_nanos` из хешируемой поверхности (часть P0); канонический conflict-сериализатор. | `verify-determinism-check` в `engine-determinism-check` |
| **DV7** | Размеченный recall-бенч (≥150 кейсов: magnitude/unit/currency/temporal/citation/format + must-NOT-conflict). | `verify-conflict-recall-check`: **recall ≥ 0.90, false-conflict ≤ 0.05** |
| **DV8** | Заменить «Limitations (Alpha)» в `VERIFY_FACT.md:297-333` измеренными числами. | `docs-claims-check` сверяет с отчётом DV7 |

**Зависимости:** P3 (consumer `verification_root`/`conflict_commitment`); **не** зависит от крипто-трека (параллелен). Все примитивы (`verification/numeric`) уже в дереве — фикс это в основном **заводка**, не новый парсер.

---

### P6 — Reproducibility + frozen learned ranker

**Северная звезда:** делает ногу `determinism_hash` реальной и нагруженной — превращает «same inputs ⇒ byte-identical pack+receipt» из утверждения в **экспортируемый, перевыводимый третьей стороной свидетель**, и гарантирует, что улучшения качества (learned ranker) едут как **замороженные целочисленные Q16-веса**, которые не сломают бит-воспроизводимость и no-LLM-инвариант.

**Текущее состояние:** скоринг — хардкод магических констант (`builder.rs:215-232`: `base_bm25 + trust + freshness − redundancy + feedback`; MMR `3*relevance`); rerank-веса — ручные (`rerank/types.rs:71-97`, `calibration.rs:17-87`); offline-LTR — только Python (`learned_ranking_calibration_check.py`), **никогда не компилируется** в Rust-константы. Детерминизм — только snapshot-строки (`tests/determinism.rs`), `engine-determinism-check` — статический lint (банит `HashMap`) над **архивированным** docs.

| Задача | exit-гейт |
|---|---|
| **RANK-1** Все коэффициенты → замороженный Q16-артефакт + генерируемый модуль (чистый рефактор) | `ranking-frozen-weights-check`: нет голых magic-констант вне generated-модуля |
| **RANK-2** Offline-trainer компилирует выбранные профили в артефакт + drift-гейт | `ranking-weights-drift-check`: trainer-артефакт == checked-in == engine-loaded |
| **REPRO-2** `determinism_hash` связывает (query, AgentView, options, `frozen_weights_version`) | `weights-version-binding-check`: hash меняется ⟺ меняются веса |
| **REPRO-3** Cross-process байт-идентичность pack+verify; заменить lint-only гейт реальным harness | `pack-determinism-hash-check`: два процесса + checkpoint байт-идентичны |
| **RANK-4** Explain-faithfulness: `score == Σ explain-компонент` | `ranking-explain-faithfulness-check` |

**Зависимости:** P0 (владелец `canonical_bytes`, P6 потребляет), P2 (реальный hash для `determinism_hash`), P1 (cosine/ANN-фиксы до заморозки весов на real-embedding-фикстурах).

---

### P7 — Absorption proof + open GCE spec + production scale

**Северная звезда:** превращает контракт из внутреннего свойства в **публично проверяемую категорию**, разряжая Oso-тест «feature-or-product».

**Спецификация + доказательство:**

| Задача | exit-гейт |
|---|---|
| **SPEC-1/2/3** Открытая `docs/spec/GCE_CONTRACT.md` (ContextPack-тип + 6 GCE-инвариантов + conformance-обязательства), заморозка `accountability_receipt.v1`, алгоритм верификатора + модель угроз (каждый класс подделки ↔ защищающее поле) | `gce-spec-doc-check`, `receipt-threat-model-check` |
| **CONF-1** Публичный conformance + adversarial-suite (scope-widening, fabricated-citation, dropped-conflict, forged-audit, anti-correlation); thin-wrapper reference **доказуемо** падает ≥3 осей | `aab-conformance-check` |

**⚠️ Over-scope (резать):** живой head-to-head matrix против 4 конкурентов (Zep/Mem0/Cognee + pgvector+OPA, помечен XL) — это **benchmark-театр**: он *доказывает* ров, а не *строит* его, тяжёл, флакает (4 сторонних стека, API-ключи, version-drift) и будет назван «подтасованным» независимо. **Достаточно одного** документированного thin-wrapper-reference, проваливающего оси эквивокации/access-binding — это разряжает Oso-тест на ~10% усилий. Полный 6-осевой AAB-leaderboard и live-baseline держать **только в nightly**, с CI-safe mini-подмножеством (по образцу `balanced_50`).

**Production scale (Raft уже построен — `replication/` + ~25 тестов, но gated как research):**

| Задача | exit-гейт |
|---|---|
| **SCALE-1** Доказать fail-closed-инвариант через кластер: 0 out-of-scope на follower-read, mid-failover, partition (может вскрыть пробелы репликации AgentView/policy_version) | `consensus-failover-binder-check` |
| **SCALE-3** Replica-invariant квитанция (байт-идентична независимо от реплики) + коммит audit-head + **transparency log** `pack_root` (промоут из stub в first-class) | `receipt-replica-invariance-check` |
| **SCALE-4** Промоут `consensus-partition-soak/failover-slo/rejoin` из research в release-lane (требовать N подряд soak-green) | `release-check` включает кластер-гейты |

**Зависимости:** P3 + P0 (replica-invariance требует канонизатора); SCALE-1 — XL, бюджетировать фикс репликации, а не только тест.

---

### Сводный критический путь (строго серийный спайн)

```
P0 canonical_bytes + исключение elapsed_nanos     ← гейт всего хешируемого
  └─ P1 cosine/conflict/cell-id/budget-disclosure ← улики истинны до нотаризации
       └─ P2 cortex-crypto (blake3 + ed25519)     ← первый крипто в воркспейсе
            └─ P3 AR-4 Merkle-квитанция + AR-5 подпись
                 └─ P3 AR-7 standalone verifier (no-engine-link, dependency-asserted)
                      └─ P4 FC-2 ann-scope-parity + AR-6 captured access + FC-7 model_hash
                           └─ P7 transparency log + thin-wrapper-reference (Oso-closure)
                                └─ P7 SCALE-1/3 cluster fail-closed + replica-invariant receipt
```

**Параллельные ветки (общего кода нет):** весь P1 (must-fix) ∥ весь P2 (crypto-foundation) — пересекаются только в AR-4. P5 (conflict-recall) и P6-`RANK-1` независимы от крипто-спайна. Schema-freeze (AR-2, SPEC-1/2) едет параллельно с крипто-реализацией. AAB-адаптеры (если оставлены) — nightly throughout.

### Топ-риски и митигации

| Риск | Митигация |
|---|---|
| Подпись над всё ещё испорченными уликами = нотаризованная ложь | Жёсткий гейт: `correctness-prerequisites-check` зелёный **до** merge AR-4 |
| Неоднозначность канонизации форкает байты DB↔verifier | Зафиксировать RFC-8785 JCS в нормативном доке + cross-language golden-векторы; верификатор перевыводит канонизацию из spec, **не** импортирует engine-модуль (assert по dependency-graph) |
| Тезис непоглощаемости слабее заявленного (обёртка эмитит ~80% квитанции) | Сделать access-проверку **engine-bound** (перезапуск подписанной `bitmap_program`) + transparency log против эквивокации — **это** ров, а не подпись |
| Schema-freeze до финализации листьев → вынужденный v2-bump | Не морозить `accountability_receipt.v1` пока не сядут P1 + AR-6 (`policy_version`) + DV4/DV5 (temporal/citation kinds) |
| Скрытый недетерминизм (HashMap-итерация, float, residual wall-clock) | Всё хешируемое — через единый P0-модуль из `BTreeMap`/`BTreeSet`; harness 3× cross-process в CI |
| Первые крипто-deps стопорятся на allowlist/энтропии CI | Front-load CRY-1; подтвердить vendoring + `getrandom`-энтропию до downstream; feature-флаг для первого релиза |
| Перф-бюджет квитанции не задан (Merkle + Argon2id + Ed25519 на запрос) | Задать p99-exit-критерий на AR-4; иначе квитанцию выключат флагом и ров испарится на практике |
| Промоут флакающих consensus-тестов дестабилизирует release CI | N подряд soak-green до промоута + demote-escape-hatch; тяжёлое — nightly |

## 4. Дорожная карта

Северная звезда — «подотчётность ответа» как **независимо проверяемый контракт**: каждый ContextPack/VERIFY несёт детерминированную, криптографически проверяемую квитанцию (accountability receipt), которую третья сторона проверяет, **не доверяя БД**. Эта дорожная карта секвенирует путь так, чтобы квитанция не стала «нотариально заверенной ложью»: сначала чиним доказательную базу и кладём настоящую крипту, потом строим квитанцию, потом независимый верификатор, потом доказываем непоглощаемость, и только затем — кластер.

### Главный принцип секвенирования: спинной хребет строго серийный, измерения — параллельны

Один доминирующий критический путь связывает всё: **достоверные доказательства → реальная крипта + каноническая сериализация → квитанция → офлайн-верификатор → доказательство категории → кластерная подотчётность**. Подписать квитанцию над `dot.abs()`-косинусом (`hnsw/metric.rs:44`), над ложными конфликтами `(project,metric)` без нормализации (`context/conflicts.rs:12-41`), над FNV-«auth tag» (`backup/encrypted/crypto.rs:33-45`) и над access-решением, которое переисчисляется в момент сборки пакета (`context/pack/access.rs:8-46`), а не захватывается в момент enforcement — значит конвертировать баг в криптографически заверенную ложь. Поэтому корректность и крипта **предшествуют** подписи. Измерительная и бенчмарочная работа (recall-корпус, scope-leak бенч, learned-ranker) идёт параллельно хребту.

### Фаза 0 (v0.2.0-beta.3) — Фундамент детерминизма и канонических байтов

Извлечь из «receipt»-столпа то, что является зависимостью **четырёх** столпов, и сделать отдельным pillar-zero: канонический модуль (`AR-1`/`REPRO-1`, JCS/RFC-8785 или явная целочисленная форма, рекурсивная сортировка ключей, без float/таймстампов, domain-теги) для ContextPack и VerificationReport, и исключение wall-clock из хешируемой поверхности. Подтверждено: `Instant::now()`/`elapsed_nanos` живут в `verification/operator.rs` (строки 28,37,46,60,83,93,102,155,166,188,203) — это провально для byte-identical. Существующий `engine-determinism-check` (`mk/core-contracts.mk:95`) сегодня — статический линт, запрещающий токены `HashMap`/`HashSet` и ссылающийся на **архивированный** `docs/archive/ENGINE_DETERMINISM.md`; то есть нынешняя «гарантия детерминизма» слабее, чем кажется.

| Задача | Exit-гейт |
|---|---|
| `AR-1` канонический модуль (JCS) | `make accountability-canonical-check` — `canonical(value)` байт-стабилен между запусками и под перестановкой порядка ключей |
| `REPRO-1` `canonical_bytes()` для ContextPack + VerificationReport | property-тест: инвариантность под перестановкой ключей карты; ни одного байта `elapsed_nanos`/`SystemTime` |
| Исключение wall-clock из хешируемой поверхности (`DV6`-половина) | field-exclusion тест грепает хешируемую поверхность и падает на любом wall-clock поле |
| Field-classification allowlist (hashed vs non-hashed) | добавление поля в ContextPack/VerificationReport без классификации ломает гейт |

**Веха выхода:** `canonical_bytes()` существует и доказанно детерминирован; крипта ещё не нужна (можно явно именованный non-crypto content hash). **Это единственная точка согласия DB↔verifier — её надо специфицировать один раз, рано, и закрепить за одной командой**, иначе она будет переписана дважды и тихо форкнет байты БД и верификатора.

### Фаза 1 (v0.2.0-beta.4) — Корректность: доказательства должны быть истинны до нотаризации

Закрыть подтверждённые дефекты, которые иначе будут заверены подписью. Держать фазу тощей — это блокер, не пункт назначения.

| Задача | Дефект (file:line) | Exit-гейт |
|---|---|---|
| `CP-1`/`FC-1` косинус | `hnsw/metric.rs:44` `dot.abs()` ⇒ анти-коррелированные = идеал; эталон есть в `context/dedup.rs:20-51` | `make cosine-metric-correctness-check`: `cosine(v,-v)==0`, `cosine(v,v)==max`, порядок `{v, ⊥, -v}`, нет overflow на high-dim i16; падает, если `dot.abs()` вернётся |
| `CP-2` унификация двух косинусов | `metric.rs` vs `dedup.rs` расходятся | тест: байт-идентичный Q16 на общем фикстуре |
| `CP-3` слот-ширина cell-id | memory 31-бит `0x7fff_ffff` (guarded), session/feedback 28-бит `0x0fff_ffff` тихо `&`-обрезают (`session.rs:156,164`, `feedback.rs:83,90`) | `make cell-id-collision-check`: различные `(agent,seq)` не коллидируют; over-width ⇒ `None`, не truncation |
| `CP-4` нормализация конфликтов | `context/conflicts.rs:12-41` + `dedup.rs:53-70`; готовый неиспользуемый `verification/numeric/{parse,value}.rs` | `make conflict-normalization-check`: `$1.2M`==`1,200,000 USD`==`1.2 million`; истинные конфликты флагуются; детерминизм conflict-set |
| `CP-5` раскрытие неполноты | `budget_exceeded` вычисляется в `search_impl.rs:75,119`, но не доходит до пакета | `make ann-budget-disclosure-check`: пакет несёт `RetrievalIncomplete` аномалию при исчерпании бюджета |

**Веха выхода:** `correctness-prerequisites-check` зелёный в release-lane. Это **жёсткий гейт перед сборкой квитанции** (`AR-4`).

### Фаза 2 (v0.3.0-beta.1) — Реальная крипта и подписанная квитанция над честными входами

Ввести первую крипту воркспейса (подтверждено: во всех `Cargo.toml` только `roaring`), собрать Merkle-квитанцию и **standalone-верификатор**. Сюда же сворачиваем захват access-решения (`FC-5`/`AR-6`): `access_root` бесполезен как переисчисление.

**Структура квитанции (целевая):**

```
receipt header (подписывается, фикс-размер):
  { schema_version:"accountability_receipt.v1", hash_alg:"blake3-256",
    sig_alg:"ed25519", db_instance_id, key_id, created_unix_seconds,
    access_root, provenance_root, cell_set_root, verification_root,
    budget_commitment, conflict_commitment, pack_root, determinism_hash, signature }

leaf = blake3(domain_tag_byte || JCS_canonical_bytes(claim)), листья сортированы по cell_id:
  ACCESS       {cell_id, decision, policy, scope_id, agent_id, policy_version}
  PROVENANCE   {cell_id, source_cell_id, byte/line range, source_ref, citation, cell_content_hash}
  CELL_SET     {cell_id, cell_content_hash = blake3(canonical cell bytes)}   ← DB-computed, не payload-string
  VERIFICATION {status, confidence_q16} + per-evidence + per-conflict      ← без elapsed_nanos
  BUDGET       {token_budget, sum(estimated_tokens), truncated, per-cell}
  CONFLICT     {conflict_visibility_q16, visible_conflict_count, anomalies[]}

pack_root        = blake3(access||provenance||cell_set||verification||budget||conflict)   — связывает ВЫХОД
determinism_hash = blake3(domain||query||AgentView-проекция||options||bitmap_program)      — связывает ВХОД
signature        = Ed25519(canonical(header))   — связывает оба
```

| Задача | Exit-гейт |
|---|---|
| `CRY-1`/`AR-1deps` аудированные RustCrypto (blake3/sha2, ed25519-dalek, chacha20poly1305, argon2, getrandom, zeroize, subtle) за feature-флагом | `make crypto-deps-policy-check`: крейты на месте; ноль FNV/XOR в backup+audit production-путях |
| `CRY-2` единый `cortex-crypto` модуль + KAT-векторы | `make crypto-primitives-check` |
| `AR-2` заморозка `accountability_receipt.v1` как additive-поля на `context_pack.v1` (`docs/schemas/context_pack.v1.json:5` разрешает до v2) | `make accountability-receipt-schema-check`: golden валиден; v1-only потребитель игнорирует новое поле |
| `AR-3` DB-computed `cell_content_hash` | детерминизм; отличается на 1 байт; не зависит от payload `content_hash` (`descriptor.rs:20`) |
| `AR-4` пять деревьев + `pack_root` + `determinism_hash` | `make accountability-receipt-determinism-check`: два прогона = byte-identical тело |
| `AR-5`/`CRY-6` Ed25519 подпись header (RFC-8032 детерм. nonce) + key custody/rotation | `make accountability-receipt-sign-check`: тот же ключ+вход ⇒ байт-идентичная подпись |
| `AR-6`/`FC-5` захваченное access-решение + `policy_version` | `make context-access-decision-capture-check`: ни одна admitted-ячейка не `NotRecorded`; решение из scan-пути, не re-derivation |
| `AR-7` standalone `cortex-receipt-verify` (НЕ линкует engine) | `make accountability-receipt-verify-check`: dependency-graph гейт подтверждает отсутствие `cortex-engine/-server/-aql` |
| `AR-8` tamper-suite (7 классов) + umbrella `accountability-receipt-check` в `alpha-check` | 100% reject мутаций, accept genuine; mutation-of-mutation guard |

**Веха выхода:** `accountability-receipt-check` зелёный в `alpha-check`; верификатор без engine-линка принимает genuine и отвергает все 7 классов tamper.

> **Перегрейд усилий (риск):** `AR-6` помечен L, но это XL — протаскивание захваченного решения через `binder→scan→pack` трогает горячий путь и несколько крейтов; `AR-7` — XL (независимая реканонизация + dependency-граф гейт). Разбить каждую на (a) compact decision token / verifier-lib и (b) consume-in-pack / no-engine-dep gate.

### Фаза 3 (v0.3.0-beta.2) — Сквозной fail-closed: сделать `access_root` честным на физическом слое

Закрыть подтверждённое расхождение: persisted ANN/lexical `allowed` строится из `readable_scopes` **только** (`search/access.rs:9-18`) и **не** компонует `PushLive`/WHERE, тогда как binder seedит `[PushAgentAllowed, PushLive, And]` (`binder.rs:137-145`). Это два разных enforcement-слоя — квитанция их сегодня примирить не может. Без этой фазы `access_root` заверяет переисчисление, а не enforcement, и единственный настоящий ров (fail-closed by plan algebra) недоказан end-to-end.

| Задача | Exit-гейт |
|---|---|
| `FC-2` derive `allowed` из bound bitmap-program | `make ann-scope-parity-check`: persisted `allowed == eval_bitmap_program(plan) ∩ vector_cells` |
| `FC-3` recall разреженного scope (exact-fallback при малом `|allowed|`) | `make ann-sparse-scope-recall-check`: recall@k сильнейше-разреженного дециля ≥ exact − ε |
| `FC-6` scope-leak бенч по ВСЕМ выходным поверхностям | `make scope-leak-bench-check`: 0 forbidden-байт в ≥200 комбинациях (agent×query×format×persistence×budget), до/после checkpoint+compact |
| `FC-7` машинно-проверяемая модель инварианта + `model_hash` | `make fail-closed-invariant-model-check`: proptest без контрпримеров `admitted ⊆ allowed ∩ live ∩ where` для обоих путей; стабильный `model_hash` в квитанцию |
| `FC-8` агрегатный гейт | `make fail-closed-end-to-end-check` зелёный в beta-lane |

**Веха выхода:** `access_root` связан с фактически исполненным enforcement, доказанным re-оценкой bound-программы против цитируемых ячеек — **именно это, а не подпись, является настоящим закрытием абсорбции**.

### Фаза 4 (v0.3.0-beta.3) — Закалка крипто-фундамента (audit + at-rest становятся настоящими)

Раз модуль крипты и key custody есть — заменить FNV/XOR. Подтверждённые дефекты: audit-цепь — unkeyed FNV-1a64 (`audit_chain.rs:8-9,42-65`, дублируется в `cli_audit_chain.rs`), коммитит только HTTP-envelope + COUNTS; backup — `cortexdb.xor-fnv64-stream.v1` с подделываемым FNV `auth_tag` (`crypto.rs:3-66`).

| Задача | Exit-гейт |
|---|---|
| `CRY-3` AEAD-backup (XChaCha20-Poly1305 + Argon2id, v2) + refuse-legacy | `make encrypted-backup-check` (tamper любого байта ct/tag/nonce/salt/AAD ⇒ ошибка, 0 leak) + `make encrypted-backup-legacy-refuse-check` |
| `CRY-4` keyed audit-цепь (SHA-256 + HMAC/Ed25519, shared writer/verifier) | `make audit-chain-check`: tamper прошлого события ⇒ fail; без ключа подделка невозможна; cli верифицирует server-цепь |
| `CRY-5` коммит receipt-hash в audit-запись | `make audit-receipt-binding-check`: запись коммитит receipt-hash; mismatch ⇒ reject |
| `CRY-7` честные docs + doc-lint | `make crypto-claims-honesty-check`: `docs/SECURITY_MODEL.md:77` называет реальный cipher/KDF; XOR-FNV-as-encryption удалён |
| `CRY-8` агрегат `crypto-foundation-check` | в `security-gate-v2-check` |

**Веха выхода:** at-rest/audit-целостность покоится на реальном AEAD/MAC, привязанном к квитанции. Закрыты два из четырёх дефектов, подрывающих тезис подотчётности.

### Фаза 5 (v0.4.0-beta.1) — Верификация при силе + замороженный детерминированный ранкер

Углубляют **conflict_commitment** и **determinism_hash**, но не блокируют существование квитанции — потому позже. Подтверждено: `fact_claim.rs:112,172-175` early-return для датированных фактов (нет numeric-конфликтов); `single_numeric_value` (`:390-394`) дропает multi-value ячейки; ранкер — это магические константы (`builder.rs:215-232`, `rerank/calibration.rs:17-87`), а `learned_ranking.enabled` (`database/ranking.rs:43-55`) лишь переключает между двумя рукописными наборами.

| Задача | Exit-гейт |
|---|---|
| `DV1`–`DV5` нормализация/конверсия единиц-валют, multi-value, temporal-same-date, citation-конфликты | `make verify-numeric-normalization-check`, `verify-temporal-conflict-check`, `verify-citation-conflict-check` (integer-only, grep против f32/f64) |
| `DV7` размеченный recall-корпус (≥150 кейсов) | `make verify-conflict-recall-check`: recall ≥ 0.90, false-conflict ≤ 0.05 |
| `RANK-1` извлечь все коэффициенты в frozen Q16-артефакт (чистый рефактор) | `make ranking-frozen-weights-check`: нет голых констант вне generated-модуля |
| `RANK-2`/`RANK-3` offline-LTR компилирует в артефакт + drift-гейт + engine-side lift | `make ranking-weights-drift-check` + `learned-ranking-calibration-check` (lift ≥ 2500 bps, win-rate ≥ 75%) |
| `REPRO-2`/`REPRO-3` `determinism_hash` + cross-process гейт | `make weights-version-binding-check`: hash меняется ⟺ меняются веса; `make pack-determinism-hash-check` |
| `RANK-4` explain-faithfulness | `score == Σ explain-компонентов` под frozen-весами |

**Веха выхода:** заверенный verdict несёт измеренное покрытие конфликтов; качество — функция инспектируемого версионированного артефакта весов, а не недокументированных констант.

### Фаза 6 (v0.5.0) — Доказательство категории (Oso feature-or-product) — урезанное

Публичный, внешне-проверяемый контракт категории. **Урезать competitor-театр:** живой 4-стек матрикс (Zep/Mem0/Cognee + pgvector) — это маркетинг, доказывающий ров, а не строящий его; он тяжёл, флаки (API-ключи, дрейф версий) и будет назван «подстроенным» независимо. Достаточно открытой спеки + standalone-верификатора + **ОДНОЙ** документированной попытки thin-wrapper, проваливающей ось equivocation/access-binding, и одного живого pgvector+OPA/Cedar baseline.

| Задача | Exit-гейт |
|---|---|
| `SPEC-1` открытый `docs/spec/GCE_CONTRACT.md` (ContextPack + шесть инвариантов + conformance) | `make gce-spec-doc-check`: покрытие терминов сверено с `context/mod.rs` |
| `SPEC-3` алгоритм верификатора + threat-model (каждый класс подделки → защищающее поле) | `make receipt-threat-model-check` |
| `BASE-1`/`BASE-2` ОДИН живой pgvector+OPA/Cedar baseline + head-to-head | `make aab-baseline-matrix-report`: baseline UNRANKED по receipt-verifiability и determinism, CortexDB RANKED по всем шести осям |
| `CONF-1` публичный conformance + adversarial suite | `make aab-conformance-check`: thin-wrapper проваливает ≥3 оси |
| `WIRE-1` агрегат `accountability-check` в release+nightly | `.PHONY`-аудит проходит |

**Веха выхода:** непоглощаемость **эмпирична и опубликована**, а не утверждена. Явно зафиксировать каверат: квитанция доказывает внутреннюю согласованность и third-party-проверяемость, **не** Byzantine-стойкость.

### Фаза 7 (v1.0.0-rc) — Кластерная подотчётность + анти-equivocation якорь

Поднять уже построенный Raft-стек (подтверждено: полный `replication/` + ~25 тестов; в release-lane сейчас только `replication-lifecycle-check`, а `consensus-partition-soak/failover-slo/rejoin` сидят под research-track `distributed-consensus-research-check`, `mk/core-security-ops.mk:129-148`) и — **критично** — выпустить append-only transparency log, закрывающий equivocation. Это единственное здесь, что влияет на поглощаемость; остальное — production-готовность.

| Задача | Exit-гейт |
|---|---|
| `SCALE-1` кластерный fail-closed | `make consensus-failover-binder-check`: seed `[PushAgentAllowed, PushLive, And]` сохранён и 0 out-of-scope на follower-read, mid-failover, partition |
| `SCALE-2` cross-node read-your-writes + monotonic-read | `make multi-agent-cluster-consistency-check` через failover и partition heal |
| `SCALE-3` replica-invariant квитанция + audit-head + transparency log | `make receipt-replica-invariance-check`: byte-identical pack+receipt+`determinism_hash` между репликами; лог `pack_root` детектит две противоречивые квитанции |
| `SCALE-4` промоция consensus-гейтов в release-lane (после N подряд soak-green) | `make release-check` включает их; `public-claims-check` с честными HA-заявлениями |

**Веха выхода:** независимая третья сторона верифицирует ответ от **любой** реплики, fail-closed доказанно держится через failover/partition, а HA — честный release-гейт, не research-track.

### Критический путь (строго серийный)

1. **Фаза 0:** канонические байты + исключение `elapsed_nanos` — гейтит всё хешируемое.
2. **Фаза 1:** косинус, нормализация конфликтов, cell-id, раскрытие неполноты — квитанция над порчей = заверенная ложь.
3. **Фаза 2:** реальная крипта + `cortex-crypto` + key custody → Merkle-квитанция + Ed25519 → standalone-верификатор без engine-линка.
4. **Фаза 3:** `FC-2` ann-scope-parity + `AR-6` захваченное access-решение + `FC-7` `model_hash` — единственный настоящий ров, доказанный end-to-end.
5. **Фаза 6:** живой baseline head-to-head + thin-wrapper, проваливающий access-binding/equivocation.
6. **Фаза 7:** кластерный fail-closed + replica-invariant квитанция + **transparency log** (должен быть НА критическом пути, не демоутнут в stub).

### Что параллелится

- Весь MUST-FIX-трек (CP/FC/DV-фиксы) **параллелен** крипто-треку (`CRY-1..CRY-4`) — общего кода нет; только `AR-4` нуждается в обоих.
- Канонический модуль (Фаза 0) специфицируется параллельно баг-фиксам — зависит лишь от набора полей, но **обязан** приземлиться до `AR-4`.
- `RANK-1` (чистый рефактор) и весь learned-ranker — независимы от хребта после `RANK-1`.
- `DV7` recall-бенч и `FC-6` scope-leak-бенч — измерения, параллельные сборке квитанции, гейтятся лишь своими баг-фиксами.
- Competitor-адаптеры (если оставлены) — тяжёлая nightly-работа сквозь Фазу 2, блокируют лишь матрикс Фазы 6.
- `SCALE-1`/`SCALE-2` разрабатываются против существующего replication-harness параллельно, но честно не проходят до существования квитанции (`SCALE-3` зависит от канонического сериализатора).
- Заморозка схем (`AR-2`, `SPEC-1/2`) — параллельно крипто-имплементации.

### Главные риски секвенирования

- **Квитанция над всё-ещё-порченными доказательствами** — худший провал. Митигация: `correctness-prerequisites-check` зелёный в release-lane **до** мерджа `AR-4`.
- **Неоднозначность канонизации** тихо форкает байты БД и верификатора. Митигация: закрепить RFC-8785 JCS в замороженном нормативном доке + cross-language golden-векторы; верификатор реканонизирует из спеки, не импортируя engine.
- **Equivocation недофинансирован.** Это единственный класс, который подписывающий wrapper тоже не остановит — значит здесь живёт ров. Поднять transparency log из stub в first-class, gated-deliverable (Фаза 7).
- **Заморозка схемы до финализации листьев** форсирует v2-bump, когда `FC-5` добавит `policy_version`, `CP-5` — `RetrievalIncomplete`, `DV4/DV5` — temporal/citation-виды. Митигация: **не** морозить `accountability_receipt.v1` до приземления Фазы 1 и `FC-5`.
- **Скрытый недетерминизм** (итерация HashMap, форматирование float, locale) утекает в root. Митигация: всё хешируемое — через один канонический модуль из BTreeMap/BTreeSet; гонять harness 3× cross-process в CI.
- **Несогласованные on-disk-брейки** (backup v2, audit v2, cell-id-кодировка) тихо ломают cross-version byte-identity. Митигация: один консолидирующий migration-гейт; legacy XOR-FNV/FNV **отвергаются** на чтении типизированной ошибкой.
- **Бюджет латентности квитанции не задан** — Merkle-fold + Argon2id + Ed25519 на запрос добавляют реальную задержку. Задать perf-exit-критерий для `AR-4`, иначе квитанцию выключат флагом и ров испарится на практике.

## 5. Доказательство категории

### Зачем этот раздел существует

Северная звезда — «подотчётность ответа» как непоглощаемый контракт — будет реализована, когда каждый `ContextPack`/`VERIFY`-ответ несёт криптографически проверяемую квитанцию (receipt). Но реализация квитанции внутри движка не доказывает категорию. Конкурент-скептик (и прецедент Stonebraker/Pavlo: векторные БД были поглощены RDBMS за ~год) скажет: «это библиотека-обёртка над Postgres+pgvector+policy-engine, а не новая БД». Этот раздел — про **внешнее доказательство**: тест Oso «фича или продукт», открытую нормативную спецификацию Governed Context Engine (GCE) и публичный бенчмарк подотчётности (AAB-1), который делает непоглощаемость **измеримой и воспроизводимой**, а не декларируемой.

Раздел зависит от двух пиллар-предшественников (реальная криптография: BLAKE3+Ed25519 вместо FNV/XOR; эмиссия receipt с `pack_root`/`determinism_hash`) и **потребляет** их. Сам он ничего не подписывает — он сертифицирует, что подписанное нельзя дёшево воспроизвести обёрткой.

### Тест Oso «фича или продукт», применённый честно

Тест Oso спрашивает: данная способность — это несколько `if` и таблица ролей, прикручиваемых к приложению, или она пересекает порог сложности/критичности, оправдывающий выделенный движок? Применяем по-инженерному, без снисхождения к себе.

| Способность ответа | Воспроизводима обёрткой pgvector+policy? | Вердикт Oso |
|---|---|---|
| Pre-filter по разрешениям | Да — Cerbos query-plan → приложение транслирует в фильтр Pinecone/Qdrant/pgvector | Фича |
| Цитаты + span-провенанс | Да — RAG-фреймворк рядом с индексом | Фича |
| Поверхность конфликтов | Да — постобработка над выдачей | Фича |
| Журнал аудита | Да — таблица решений (decision log) | Фича |
| Детерминированный скоринг | Да — фиксированные веса в коде | Фича |
| Подпись Ed25519 над бандлом | Да — обёртка тоже умеет подписывать JCS-бандл | Фича |
| **access_root, привязанный к фактически исполненному плану fail-closed** | **Нет** — обёртка применяет фильтр out-of-band, она не может доказать, что выдача была сужена | **Продукт** |
| **Анти-эквивокация (нельзя выдать две противоречивые валидно-подписанные квитанции)** | **Нет** — единственный класс подделки, который подписывающая обёртка тоже не закрывает | **Продукт** |

**Вывод честный и неприятный.** Большинство пунктов «подотчётности» поодиночке проходят тест Oso как ФИЧА. Решающий контр-пример — собственный продукт Cerbos для RAG: policy-engine выдаёт query-plan, а **приложение** транслирует его в фильтры векторного стора и применяет сам. Это ровно та обёртка, которую предсказывает тезис поглощения.

Непоглощаемое ядро — НЕ отдельная фича, а **замыкание (closure)**, которого обёртка структурно не даёт. Оно держится на двух вещах, и обе — load-bearing:

1. **`access_root`, связанный с реально исполненным fail-closed-планом.** Биндер сеет каждый план `[PushAgentAllowed, PushLive, And]` и только AND-ит пользовательский `WHERE` (binder.rs:137-145, подтверждено). Расширение области невозможно по алгебре плана — это сильнее RLS и сильнее Cerbos/OpenFGA, которые лишь РЕКОМЕНДУЮТ фильтр, применяемый приложением. **Чтобы пройти тот же верификатор, обёртке пришлось бы реализовать эту алгебру плана и привязать исполнённую `BitmapProgram` к квитанции** — не «реализовать весь движок» (это риторика), а воспроизвести именно это связывание.
2. **Анти-эквивокация через прозрачный лог.** Подписывающая обёртка может всё, кроме одного: помешать оператору выпустить две противоречивые, но каждая-по-себе-валидно-подписанная квитанции. Это единственный класс подделки, который подпись не ловит, — значит, ровно здесь живёт ров.

> ⚠️ **Критическая поправка к формулировке.** Тезис «воспроизвести квитанцию = переписать весь движок» — это риторика, а не доказательство. Тонкая обёртка pgvector+OPA+ed25519 МОЖЕТ выпустить JCS-канонический, Merkle-корневой, подписанный бандл с leaf-ами access/provenance/budget — это ~80% `accountability_receipt.v1`. Поэтому нормативное требование к верификатору: **проверка доступа должна ПЕРЕ-ВЫЧИСЛЯТЬ подписанную `bitmap_program` против цитируемых ячеек, а не доверять leaf-у `decision: "allowed"`.** Без этого `access_root` подтверждает повторный вывод (re-derivation), а не enforcement — и ров испаряется.

### Открытая спецификация GCE (нормативная, не маркетинговая)

Спецификация публикуется в `docs/spec/GCE_CONTRACT.md` + замороженные JSON Schema в `docs/schemas/`, так, чтобы независимый разработчик мог построить совместимый GCE, а третья сторона — проверить любой ответ CortexDB **без доверия к БД**.

**Состав:**

- **Тип результата `ContextPack`** — полевая привязка к `crates/cortex-engine/src/context/mod.rs:97-192` (access_decision, span-провенанс, `conflict_visibility_q16`, бюджет токенов, аномалии).
- **Шесть инвариантов GCE** — каждый со ссылкой на источник: (1) результат = скомпилированный управляемый контекст; (2) детерминированное LLM-free управление на Q16 (`Q16_ONE=65535`); (3) fail-closed по алгебре плана (binder.rs:137-145); (4) провенанс+верификация как first-class выходы; (5) сохранение конфликтов, не LWW; (6) TTL/decay как сигнал ранжирования.
- **`accountability_receipt.v1`** — ОДНО additive optional поле верхнего уровня в `context_pack.v1` (соблюдая клаузу «additive optional fields allowed until v2», `docs/schemas/context_pack.v1.json:5`), так что v1-консьюмер его игнорирует.
- **Каноникализация** — RFC 8785 JCS или явная байтовая форма (целые/Q16, рекурсивная сортировка ключей, без float/timestamp, domain-tag). Это **единственная точка согласия** DB↔верификатор; существующий `json!` экспорт ключи рекурсивно НЕ сортирует, поэтому каноникализатор — отдельный нормативный модуль.
- **Алгоритм офлайн-верификатора + модель угроз** — точные 7 шагов и перечень того, что злонамеренная/багованная БД НЕ должна суметь подделать.

**Структура receipt (заголовок — единственные подписываемые байты, фиксированный размер):**

```
accountability_receipt.v1
├── header (подписан Ed25519 целиком)
│   ├── schema_version   : "accountability_receipt.v1"
│   ├── hash_alg         : "blake3-256"
│   ├── sig_alg          : "ed25519"
│   ├── db_instance_id, key_id, created_unix_seconds
│   ├── access_root          ← REJECT-условие: любой leaf != "allowed" ⇒ отказ
│   ├── provenance_root      ← span ⊆ байты ячейки + cell_content_hash
│   ├── cell_set_root        ← leaf = {cell_id, cell_content_hash = blake3(canon bytes)}  (НЕ payload content_hash)
│   ├── verification_root    ← {status, confidence_q16} + evidence + конфликты; elapsed_nanos ИСКЛЮЧЕНЫ
│   ├── budget_commitment    ← sum(estimated_tokens) ≤ token_budget_tokens, truncated
│   ├── conflict_commitment  ← conflict_visibility_q16, visible_conflict_count, anomalies[]  (инвариант 5)
│   ├── model_hash           ← из fail-closed-invariant-model (FC-7): аттестация свойства, не утверждение
│   ├── pack_root            = blake3(access ‖ provenance ‖ cell_set ‖ verification ‖ budget ‖ conflict)  ← OUTPUT
│   ├── determinism_hash     = blake3(query ‖ AgentView-проекция ‖ options ‖ frozen_weights_version ‖ bitmap_program)  ← INPUT
│   └── signature            : ed25519(canonical(header))   [RFC-8032 детерминированные nonce]
└── leaves[] (открытые байты, сворачиваются в roots; верификатор пере-выводит из публичных данных)
```

`pack_root` связывает ВЫХОД, `determinism_hash` — ВХОД, подпись над заголовком связывает оба ⇒ «одинаковый вход ⇒ байт-идентичный pack+receipt» становится внешне проверяемым.

**Модель угроз — что БД НЕ должна суметь подделать (каждое ловится конкретным шагом верификатора):**

| Класс подделки | Защищающее поле | Шаг верификатора |
|---|---|---|
| Допустить недоступную ячейку | `access_root` (+ пере-оценка `bitmap_program`) | отказ, если leaf != allowed ИЛИ план не допускает ячейку |
| Сфабриковать цитату/span вне байтов ячейки | `provenance_root` + `cell_content_hash` | `0 ≤ byte_start ≤ byte_end ≤ len`, подстрока на смещении |
| Скрыть `VisibleConflict` / завысить вердикт | `conflict_commitment`, `verification_root` | свёртка не сойдётся |
| Перерасход/искажение бюджета | `budget_commitment` | `sum ≤ budget`, согласованность `truncated` |
| Заявить детерминизм, вернув другой pack | `determinism_hash` (+ опц. пере-исполнение) | пере-вычисление хеша |
| Реиграть чужую квитанцию | `determinism_hash` связывает (query, AgentView, options) | не совпадёт под другим запросом |
| **Эквивокация (две противоречивые валидные квитанции)** | **прозрачный лог `pack_root` (вне самого receipt)** | внешний witness/mirror |

> Out-of-scope, документируется явно: квитанция доказывает **внутреннюю согласованность**, НЕ фактическую истинность ячеек. Эквивокация закрывается ТОЛЬКО append-only прозрачным логом `pack_root`. **Это и есть место, где живёт ров — его нельзя демотировать в «stub/follow-up».**

### Бенчмарк подотчётности AAB-1

`AAB-1` — публичная категория, оценивающая систему по шести осям при ФИКСИРОВАННОМ бюджете токенов B ∈ {2k, 4k, 8k}. Оси 1-4 переиспользуют признанную методологию (ALCE/TREC-RAG nugget+sentence-support для цитат; ConflictBank/MAGIC для конфликтов; LongMemEval/LoCoMo как QA-субстрат; permission-aware-RAG для scope-утечек), чтобы категория была credible. Оси 5-6 — новый, непоглощаемый вклад.

| # | Ось | Метрика / цель | Методология |
|---|---|---|---|
| 1 | Scope-leak@budget | 0 запретно-scope-ячеек в pack ИЛИ цитатах | расширяет `context-pack-private-scope-check` |
| 2 | Citation precision/recall | NLI-entailment (ALCE TRUE) + sentence-support | переиспользуется |
| 3 | Contradiction recall + false-conflict | recall ≥ 0.90, ложн. ≤ 0.05 | ConflictBank/MAGIC + unit/currency-варианты |
| 4 | Tokens-to-answer | один эталонный токенайзер (cl100k/o200k) | детерм. профиль — вторичная метрика |
| 5 | **Receipt-verifiability** | **100% или UNRANKED** + tamper-rejection | standalone-верификатор без линковки движка |
| 6 | **Determinism** | байт-идентичность pack+receipt, кросс-процесс | переиспользует канонический сериализатор |

**Решающий ход — gating, а не leaderboard-дельта.** Headline-метрика — гармоническая комбинация, **заблокированная осями 5+6**: если receipt-verifiability < 100% ИЛИ хоть один tamper не пойман — система **UNRANKED, а не низко-ранжированная**. Именно gating делает категорию непоглощаемой: предсказанный результат head-to-head — Zep/Graphiti (~63.8% LongMemEval), Mem0 (sub-7k токенов), Cognee и стек pgvector+(OPA/Cedar)+RAGAS могут быть конкурентны по точности/токенам, но ВСЕ дают 0/UNRANKED на осях 5-6, потому что ни один не выпускает третье-сторонне-проверяемую квитанцию и ни один не fail-closed по конструкции плана. Вывод бенчмарка — **структурный**, а не дельта в таблице.

> ✂️ **Поправка по объёму (anti-overscope).** Живой head-to-head с четырьмя сторонними стеками (Zep/Mem0/Cognee + pgvector-policy) — XL и flaky (API-ключи, дрейф версий) и будет назван «подтасованным» вне зависимости от результата. Для прохождения теста Oso достаточно **ОДНОГО** credible живого baseline (pgvector+OPA/Cedar) + опубликованной открытой спецификации + standalone-верификатора + одной документированной попытки «тонкой обёртки», которая доказуемо проваливает ≥3 оси. Остальные конкуренты — корроборация в nightly-полосе, НЕ gating и НЕ в быстрой release-полосе. Это даёт ~10% усилий при том же доказательстве категории.

### Задачи и exit-гейты

Все гейты следуют конвенции репозитория `<area>-<facet>-check`: `cargo test` + `python scripts/<name>_check.py --report <JSON>`, регистрация в `mk/phony.mk`, агрегация в `accountability-check` → release/nightly-полосы.

| ID | Задача | Exit-гейт | Усилие | Зависит |
|---|---|---|---|---|
| SPEC-1 | Открытая `GCE_CONTRACT.md` (тип результата + 6 инвариантов + обязательства совместимости) | `make gce-spec-doc-check`: покрытие терминов сверяется с `context/mod.rs`; JSON-отчёт | M | — |
| SPEC-2 | Заморозка `accountability_receipt.v1.json` как additive-поля | `make accountability-receipt-spec-check`: валидация golden + schema-freeze diff (зеркало `context-pack-schema-contract-check`) | M | SPEC-1, real-crypto |
| SPEC-3 | Алгоритм верификатора + модель угроз (каждый класс ↦ защищающее поле) | `make receipt-threat-model-check`: перечислены все 7 шагов и все классы подделки | M | SPEC-2 |
| VERIF-1 | Standalone `cortex-receipt-verify` (НЕ линкует cortex-engine/server/aql) | `make accountability-receipt-verify-check`: валидирует 100% golden из публичных входов; cargo-graph-ассерт исключает движок | L (фактически XL — разделить на lib + dep-гейт) | SPEC-3, receipt-эмиссия |
| VERIF-2 | Adversarial tamper-suite (7 классов мутаций) | `make accountability-receipt-tamper-check`: отказ на каждой мутации, приём genuine, mutation-of-mutation guard | M | VERIF-1 |
| VERIF-3 | Кросс-процессный байт-идентичный детерминизм | `make accountability-receipt-determinism-check`: 2 процесса ⇒ идентичный `determinism_hash`; ассерт отсутствия `elapsed_nanos` (verification/operator.rs) в хешируемой поверхности; расширяет `engine-determinism-check` | M | VERIF-1 |
| FC-7 | Машинно-проверяемая модель fail-closed (proptest над bitmap-program И persisted-ANN путём) + экспорт `model_hash` | `make fail-closed-invariant-model-check`: нет контрпримера `admitted ⊆ agent_allowed ∩ live ∩ where`; стабильный `model_hash` | L | FC-2, FC-3 |
| BASE-1 | Живой baseline pgvector+OPA/Cedar+RAG (ОДИН, не четыре) на общем корпусе | `make aab-baseline-stack-check` (nightly, контейнеризован): отвечает на фикс-набор, эмитит сравнимый JSON | L | — |
| BASE-2 | Six-axis scorer + absorption-proof отчёт | `make aab-baseline-matrix-report`: baseline 0/UNRANKED на осях 5-6, CortexDB ранжирован на всех шести | L | BASE-1, VERIF-1, VERIF-3 |
| CONF-1 | Публичный conformance + adversarial suite (scope-widening, fabricated-citation, dropped-conflict, forged-audit, анти-корреляция) | `make aab-conformance-check`: CortexDB проходит всё; reference-обёртка проваливает ≥3 оси | M | VERIF-2, BASE-2 |
| WIRE-1 | Агрегат `accountability-check` в release + heavy nightly | `make accountability-check` зелёный в release-полосе; phony-аудит подтверждает `.PHONY` | S | SPEC-2, VERIF-2, VERIF-3, CONF-1 |

**Измеримые критерии выхода пиллара:**

- `make accountability-check` (spec-freeze + standalone-verify + tamper + кросс-процессный детерминизм + conformance) зелёный в release-полосе.
- `make aab-baseline-matrix-report` воспроизводимо показывает: CortexDB ранжирован на всех шести осях, живой pgvector+policy baseline — UNRANKED на receipt-verifiability и determinism.
- `cortex-receipt-verify` валидирует пару (pack.json + receipt.json + сырые байты допущенных ячеек + публичный ключ) офлайн, exit 0/non-0, **без линковки** `cortex-engine` (ассерт графа зависимостей).
- Верификатор шага доступа **пере-оценивает** подписанную `bitmap_program`, а не доверяет leaf-у `allowed` (закрывает дыру обёртки).
- Прозрачный лог `pack_root` опубликован как first-class (не stub), мера против эквивокации.
- Опубликована `GCE_CONTRACT.md`, достаточная для независимой реализации совместимого GCE.

### Зависимости и порядок

- **Receipt + real-crypto (предшественник):** BLAKE3/SHA-256 Merkle + Ed25519, замена FNV-1a64 (`audit_chain.rs:42-65`) и XOR-FNV (`backup/encrypted/crypto.rs:3-66`). Гейты VERIF-* и tamper НЕ проходят на FNV.
- **MUST-FIX корректность (предшественник):** фикс cosine `dot.abs()` (metric.rs:44 — подтверждён; вдобавок `dot * 65_535` — `i64` до приведения, риск переполнения на высокоразмерных i16) и поверхность `budget_exceeded`. Анти-корреляционная ловушка CONF-1 зависит от фикса cosine: **подписанная квитанция над испорченным evidence — нотариально заверенная ложь.**
- **Детерминизм (предшественник):** wall-clock `total_elapsed_nanos`/`elapsed_nanos` (`verification/operator.rs:28,188,203`) вынести из хешируемой/канонической поверхности до VERIF-3.
- **Порядок внутри раздела:** SPEC-1 → SPEC-2 → SPEC-3 → VERIF-1 → {VERIF-2, VERIF-3, FC-7} → BASE-1 → BASE-2 → CONF-1 → WIRE-1. Schema-заморозку (SPEC-2) делать ПОСЛЕ стабилизации набора leaf-ов кодом (FC-5 добавляет `policy_version`, DV4/DV5 — temporal/citation-классы конфликтов), иначе вынужденный bump до v2.

### Главные риски этого раздела

| Риск | Митигация |
|---|---|
| Тезис непоглощаемости слабее заявленного: обёртка воспроизводит ~80% receipt | Верификатор шага доступа структурно привязан к движку — пере-оценивает подписанную `bitmap_program`; + прозрачный лог против эквивокации. ЭТИ ДВА, не подпись, — ров (P0). |
| Эквивокация недофинансирована (повторно «stub/optional») | Промотировать append-only лог `pack_root` до first-class gated-deliverable; явно заявить caveat в v0.5, чтобы claim был «внутренне согласован и третье-сторонне проверяем», а не «византийски-стойкий». |
| Каноникализация двусмысленна ⇒ DB и верификатор молча расходятся | Зафиксировать RFC-8785 JCS / явную целочисленную байт-форму в замороженном нормативном доке + кросс-языковые golden-векторы; верификатор пере-выводит каноникализацию из спеки, НЕ импортирует модуль движка. |
| Head-to-head назовут подтасовкой | Оси 1-4 на признанной методологии (ALCE/TREC/ConflictBank), где baseline честно выигрывает/ничья; оси 5-6 — единственный СТРУКТУРНЫЙ дифференциатор; harness + верификатор опубликованы для воспроизведения. |
| Конкурентный матрикс (XL, flaky) тащит чистую infra-задачу в заложники маркетинга | Один живой baseline gating; Zep/Mem0/Cognee — nightly-корроборация; не в быстрой release-полосе. |
| VERIF-1/FC-7 недооценены как «L» | Пере-оценить в XL, разделить VERIF-1 на (lib) + (no-engine-dep-гейт). |

---

Раздел готов (Markdown выше). Объём в пределах целевого диапазона (~14-16k символов).

Ключевые сверки с источником, на которые опирается текст: cosine `dot.abs()` подтверждён на `metric.rs:44` (с дополнительным замечанием, что `dot * 65_535` — `i64` до приведения к `u64`, что усугубляет риск переполнения на высокоразмерных i16-векторах); fail-closed seed `[PushAgentAllowed, PushLive, And]` с `WHERE` только через `ops.push(BitmapOp::And)` подтверждён на `binder.rs:137-145`.

Две содержательные поправки, внесённые в раздел относительно входных пилларов (взяты из критики SEQUENCING): (1) непоглощаемость держится не на самой подписи (её обёртка воспроизводит), а на привязке `access_root` к пере-оцениваемой `bitmap_program` + анти-эквивокационном прозрачном логе — поэтому верификатор обязан пере-вычислять план, а не доверять leaf-у `allowed`; (2) живой 4-конкурентный матрикс сокращён до одного gating-baseline как overscope-театр.

## 6. Исправления, риски, метрики

> Северная звезда раздела: **подотчётность ответа (answer accountability)** — это контракт, по которому каждый ContextPack/VERIFY несёт детерминированную, криптографически проверяемую квитанцию (receipt), которую независимая третья сторона проверяет **не доверяя БД**. Подписанная квитанция над повреждёнными или неполными доказательствами — это не «гарантия», а *нотариально заверенная ложь*. Поэтому набор исправлений ниже — не «технический долг», а **предусловия допуска**: без них receipt-слой запрещено собирать.

### 1. Почему это раздел про предусловия, а не про качество

Receipt связывает в подписанный корень шесть вещей: scope-решения, цитаты/провенанс, вердикт VERIFY + конфликты, бюджет токенов, хэш детерминизма и (через `access_root`) факт fail-closed-исполнения. Каждый подтверждённый дефект разрушает ровно один из этих листьев:

| Дефект | Что подписывает receipt | Чем это делает квитанцию ложью | Источник |
|---|---|---|---|
| cosine `dot.abs()` | «самые релевантные допущенные ячейки» | анти-коррелированный вектор (v vs −v) получает идеальный скор → ранжирование, которое заверяет `verification_root`/grounding, искажено | `crates/cortex-engine/src/search/hnsw/metric.rs:44` |
| persisted-ANN `allowed` из одних `readable_scopes` | `access_root` = «допущено ровно то, что разрешил план» | физический путь фильтрует по более широкому базису, чем доказанная алгебра плана (нет `PushLive`/WHERE) → `access_root` заверяет ре-деривацию, а не enforcement | `crates/cortex-engine/src/search/access.rs:9-17` |
| guarded-ANN post-filter под общим визит-бюджетом | «контекст полон» | sparse-scope агент молча получает неполный pack; `budget_exceeded` вычислен, но не доходит до ContextPack | `crates/cortex-engine/src/search/hnsw/search_impl.rs:90,74-77` |
| (project,metric) эвристика без нормализации | `conflict_commitment` (GCE-инвариант 5) | `$1.2M` vs `1,200,000 USD` vs `1.2 million` = ложный 3-сторонний конфликт; форматно-разные истинные конфликты пропущены | `crates/cortex-engine/src/context/conflicts.rs:12-41` + `dedup.rs:53-70` |
| 28-bit vs 31-bit slot-width | привязка ячейки к агенту | feedback/session молча `&`-усекают agent_id → два агента коллапсируют в один cell-id; memory держит их раздельно | `cell_ids.rs:6,13-20` vs `session.rs:156,164`, `feedback.rs:83,90` |
| XOR-FNV «шифрование», FNV «auth tag», FNV-1a64 audit-цепь | подпись / целостность at-rest / неизменность аудита | подделываемы любым, кто может запустить тот же код; в воркспейсе **ноль** крипто-крейтов | `backup/encrypted/crypto.rs:3-45`; `audit_chain.rs:8-9,42-48` |
| `elapsed_nanos`/`Instant::now()` в отчёте VERIFY | хэш детерминизма | wall-clock в хэшируемой поверхности → «same inputs ⇒ byte-identical» недостижимо | `verification/operator.rs:28,183,188,203` |

Вывод: **receipt-сборка (фаза v0.4) не имеет права мёрджиться, пока зелёные `correctness-prerequisites-check` + `crypto-foundation-check` + `canonical-serialization-check` не в release-lane.** Это жёсткий гейт, а не рекомендация.

### 2. Каталог обязательных исправлений (с exit-гейтами)

Имена гейтов следуют принятому в репозитории соглашению `<area>-<facet>-check` (cargo-тест + `scripts/<name>_check.py --report <JSON>`, .PHONY в `mk/phony.mk`, путь отчёта в `mk/vars-*.mk`).

#### P-1. Знак и переполнение cosine
- **Действие.** Заменить ветку `Self::Cosine` (`metric.rs:28-45`): вернуть `0` при знаковом `dot <= 0` (анти-коррелированные и отрицательно-ортогональные не должны быть совпадениями); расширить шаг `* 65_535` числителя до `i128/u128` — сейчас квадраты норм уже в `u128` (стр. 39), но числитель `dot.abs() * 65_535` остаётся `i64` и переполняется на высокоразмерных `i16`. Эталон уже есть в крейте: `context/dedup.rs:20-51` (`cosine_similarity_q16`: отбрасывает `dot<=0`, `i128/u128` + `isqrt`). У `metric.rs` нет ни одного `#[test]`, а фикстура `ann_metric_matrix` использует только ортогональные/положительные векторы — баг невидим всем текущим гейтам.
- **Exit-гейт.** `make cosine-metric-correctness-check`: `cosine(v,−v)==0`, `cosine(v,v)==max`, ранжирование `{v, ⊥, −v}` ставит `v` первым / `−v` последним, нет паники на максимально-магнитудном высокоразмерном векторе. **Гейт ПАДАЕТ, если `dot.abs()` возвращается.**
- **Доп.** `make` под-проверка: `metric.rs`-cosine и `dedup.rs`-cosine дают побайтно равный Q16 на общей батарее — две реализации не должны снова разойтись.
- **Размер/риск.** S; единственная чистая функция, монотонный контракт сохранён. Риск: ANN-recall-бейзлайны, настроенные на багнутые скоры, легитимно сдвинутся — переснять `ann-metric-matrix-check`.

#### P-2. Паритет fail-closed на физическом пути
- **Действие.** `allowed_candidates` (`access.rs:9-17`) объединяет **только** `readable_scopes` и потребляется в `search/database/persisted.rs:53` и `search/evaluation.rs:42`, обходя `PushLive`(status) и WHERE, которые доказанно навязывает bitmap-program биндера (`binder.rs:137-145`). Пробросить `BitmapProgram` плана (или его `RoaringBitmap` через `eval_bitmap_program`) в persisted-точки входа и пересечь с множеством vector-bearing ячеек, чтобы базис ANN/lexical **доказуемо равнялся** базису биндера, а не был отдельной, более широкой поверхностью enforcement.
- **Exit-гейт.** `make ann-scope-parity-check`: для матрицы scope×status×WHERE — `persisted allowed == eval_bitmap_program(plan) ∩ vector_cells`; включить кейс, где `live=false`/WHERE-исключённая, но readable-scope ячейка раньше допускалась, а теперь исключена.
- **Размер/риск.** M; трогает несколько search-точек входа; не регрессировать changed-after-checkpoint логику (`persisted.rs:38-47`); проверить единственность источника фильтрации против `PermissionFilter`.

#### P-3. Захват реального access-решения (не ре-деривация в pack-time)
- **Действие.** `context/pack/access.rs:8-46` повторно вызывает `PolicyRewrite::allows_scope` на этапе сборки pack — это может дрейфовать от того, что реально сделал биндер/скан, и не несёт версии политики. Изменить путь scan (`exec/scans.rs` `PermissionFilter:80-92`, разрешение кандидатов в `database/read.rs`), чтобы он эмитил захваченный `ContextAccessDecision` с дайджестом AgentView / `policy_version`, допустившим ячейку; свернуть (ограниченно) WHERE/scope-исключённые ячейки в структуру denied-set, которую receipt-слой потом хэширует. На успешном AQL-retrieve ни одна допущенная ячейка не должна быть `NotRecorded`.
- **Exit-гейт.** `make context-access-decision-capture-check`: у каждой допущенной ячейки `decision==Allowed` с непустыми `policy_version`/`agent_view_digest`; решение происходит из scan-пути (подделать scope ячейки после скана и убедиться, что репортится захваченное решение, не ре-деривация).
- **Размер/риск.** M→L (трогает горячий retrieve-путь, несколько крейтов); нести компактный decision-token, не полную структуру, до сборки pack; формат дайджеста согласовать с receipt-слоем заранее, чтобы не переделывать.

#### P-4. Числовая нормализация конфликтов через существующий модуль
- **Действие.** `context/conflicts.rs::measure` группирует по lowercased сырой строке `(project,metric)` и помечает `values.len() > 1`; экстрактор `dedup.rs:53-70` ловит только строки, буквально начинающиеся с `project=/metric=/value=`. **Рычаг: готовый числовой модуль уже в дереве и не используется здесь** — `verification/numeric/parse.rs` (`extract_numeric_values`, `parse_currency_code`, `parse_unit_code`, `parse_magnitude_suffix`) и `value.rs` (`NumericValue::normalized_eq`, `conflicts_with`, `compare_numeric_values`, всё integer/Q16-детерминированное). Маршрутизировать сравнение значений через него; строковый fallback — только для нечисловых значений. Расширить экстрактор за пределы литерального `key=value` или явно задокументировать поддерживаемые форматы. Это прямой вход в `conflict_visibility_q16`/`VisibleConflict`, которые заверяет receipt.
- **Exit-гейт.** `make conflict-normalization-check` (зеркало `context-pack-conflict-visibility-check`): на размеченной фикстуре с ≥2 форматами payload нормализованно-равные значения **не** помечаются, истинные конфликты помечаются; репортятся recall и precision; под-инвариант детерминизма: одинаковый вход ⇒ одинаковый conflict-set + `conflict_visibility_q16`. **Не писать новый парсер.**
- **Размер/риск.** M; бейзлайн `context-pack-conflict-visibility-check` сдвинется (ложные конфликты исчезнут, истинные появятся) — переснять осознанно и задокументировать дельту recall/precision. `extract_project_metric_value` также используется в `is_redundant` (`dedup.rs:72-103`) — менять только шаг нормализованного сравнения в `conflicts.rs`, не трогая short-circuit редундантности.

#### P-5. Единая ширина cell-id slot и отказ вместо усечения
- **Действие.** Выбрать одну задокументированную ширину agent-slot (рекомендация: 31 бит, `0x7fff_ffff`, как у memory, которая уже защищена `memory_cell_id`, возвращающим `None` при переполнении). Маршрутизировать session (`session.rs:156,164`) и feedback (`feedback.rs:83,90`) через общий хелпер в `cell_ids.rs`, который **возвращает None/ошибку** при over-width вместо текущего молчаливого `& 0x0fff_ffff`. Feedback не имеет ни цикла избегания коллизий, ни guard — это худший случай. Топ-ниблы (memory `0x8..`, feedback `0x9..`, session `0xA..`) уже разделяют подсистемы, так что меняется только внутри-подсистемная раскладка бит агента.
- **Exit-гейт.** `make cell-id-collision-check`: property/exhaustive-тест — различные `(agent_id, sequence)` в задокументированном домене никогда не коллизируют между memory/session/feedback; over-width id возвращает None/ошибку вместо усечения; feedback больше не маскирует молча.
- **Размер/риск.** M; **меняет on-disk-кодировку feedback/session** ⇒ schema-version bump + миграция/refuse-to-read guard, иначе тихо ломается «same-inputs ⇒ byte-identical» инвариант между версиями. Координировать с `migration-compatibility-check`.

#### P-6. Раскрытие неполноты ретрива как аномалии ContextPack
- **Действие.** `budget_exceeded` вычислен (`search/ann/search.rs:116-143`, `search_impl.rs:75,119`), доходит до `AnnSearchReport`, но никогда не пересекает границу в ContextPack; `ContextPackAnomalyCode` (`context/mod.rs:163-184`) объявляет `ScopeMismatch`/`InsufficientContext`, но пушится только `TokenOverload` (`pack/builder.rs:147`). Добавить `RetrievalIncomplete` (или переиспользовать `InsufficientContext`) и пушить аномалию при ANN `budget_exceeded` с выключенным fallback или при scope/WHERE-сужении, обронившем кандидатов. Это дешёвая, ценная половина guarded-ANN: полную ре-архитектуру pre-filter/partitioning **отложить** в отдельный retrieval-pillar.
- **Exit-гейт.** `make ann-budget-disclosure-check` / `context-pack-retrieval-incomplete-check`: sparse-scope, budget-исчерпанный, fallback-off retrieve даёт ContextPack с аномалией `RetrievalIncomplete`, проходящей round-trip через все export-форматы; полный retrieve — без неё.
- **Размер/риск.** Low-M; additive optional поле (context_pack.v1 разрешает до v2); текст аномалии не должен сам утекать scope-детали (покрыто scope-leak-бенчем).

#### P-7. Реальная криптография (закрывает два из четырёх дефектов)
- **Действие.** В воркспейсе **ноль** крипто-крейтов (`grep` по всем `Cargo.toml` даёт только `roaring`). Нет `[workspace.dependencies]` — деклария по крейтам. Под feature-флагом ввести аудированные RustCrypto: `blake3`(или `sha2`), `ed25519-dalek`, `chacha20poly1305`, `argon2`, `getrandom`, `zeroize`, `subtle`. Создать один общий модуль примитивов (hash/AEAD/KDF/MAC/sign) с закреплёнными KAT-векторами. Заменить: (a) XOR-FNV backup + поддельный FNV `auth_tag` (`backup/encrypted/crypto.rs:3-45`) на XChaCha20-Poly1305 + Argon2id, отказ читать legacy `xor-fnv64-stream.v1`; (b) unkeyed FNV-1a64 audit-цепь (`audit_chain.rs:42-48`, дублированную в `cortex-cli/src/cli_audit_chain.rs`) на SHA-256 + HMAC/Ed25519 с общим writer/verifier; (c) привязать per-answer receipt-hash в audit-record (сейчас коммитятся только HTTP-метаданные + счётчики).
- **Exit-гейты.**
  - `make crypto-deps-policy-check`: одобренные крейты присутствуют; grep по `crates/*/src` (кроме tests/benches) подтверждает **ноль** FNV/XOR integrity-рутин в backup/audit production-путях (нет хитов на `apply_keystream`/`auth_tag`/`0xcbf29ce4...`/`0x100000001b3`).
  - `make encrypted-backup-check` (v2 AEAD): round-trip; tamper любого байта ciphertext/tag/nonce/salt/AAD ⇒ `open()` ошибка, ноль утечки plaintext; неверный passphrase падает чисто; KAT совпадают. + `encrypted-backup-legacy-refuse-check`: v1 XOR-FNV архив отвергнут на чтении.
  - `make audit-chain-check` (keyed): tamper прошлого события падает; без MAC-ключа подделать цепь нельзя; cortex-cli верифицирует server-написанную цепь через общий модуль.
- **Размер/риск.** M каждый; on-disk-форматы (backup-header, audit-цепь) ломаются ⇒ version-bump + refuse-or-migrate. Риск зависимостной политики/энтропии CI — фронт-лоадить `crypto-deps-policy-check` раньше всего downstream.

#### P-8. Канонизация + исключение wall-clock (фундамент детерминизма)
- **Действие.** Вынести `total_elapsed_nanos`/`elapsed_nanos` из любой хэшируемой/сериализуемой поверхности (`verification/operator.rs:28,183,188,203`) в неподписываемый perf-канал. Реализовать нормативный `canonical_bytes()` (RFC 8785 JCS либо явная байт-форма: только integers/Q16, рекурсивно сортированные ключи, без float/timestamp, domain-tags) для ContextPack и VerificationReport. Существующий `json_export.rs` `json!` фиксирует порядок ключей source-order, но **не** сортирует рекурсивно и не нормализует числа — это не receipt-grade. Этот модуль — единственная точка согласия БД и верификатора; владелец — один (reproducibility-pillar владеет, receipt-pillar потребляет), иначе две расходящиеся схемы хэширования.
- **Exit-гейт.** `make canonical-serialization-check`: `canonical_bytes` побайтно стабилен между двумя прогонами и под перестановкой порядка вставки ключей; field-exclusion-тест грепает хэшируемую поверхность и падает при наличии `elapsed_nanos`/`SystemTime`. Крипто здесь не требуется.
- **Размер/риск.** M, но это самая высокорычажная точка: неоднозначная канонизация молча форкает байты БД и верификатора. Закрепить нормативным doc + cross-language golden-векторами; вес выше, чем намекает тег «M».

### 3. Сводная таблица гейтов и порядок

| ID | Гейт | Размер | Зависит от | Ломает формат |
|---|---|---|---|---|
| P-1 | `cosine-metric-correctness-check` | S | — | нет |
| P-2 | `ann-scope-parity-check` | M | — | нет |
| P-3 | `context-access-decision-capture-check` | M→L | P-2 | нет |
| P-4 | `conflict-normalization-check` | M | — | нет |
| P-5 | `cell-id-collision-check` | M | — | **да** (session/feedback cell-id) |
| P-6 | `ann-budget-disclosure-check` | L→M | P-1,P-2 | additive поле |
| P-7 | `crypto-foundation-check` (агрегат) | M×3 | crypto-deps | **да** (backup, audit) |
| P-8 | `canonical-serialization-check` | M | — | нет |
| — | `correctness-prerequisites-check` (агрегат P-1,P-3,P-4,P-5,P-6) | S | все выше | — |

**Порядок (по находкам sequencing):** сначала самодостаточные без слома формата (P-1, P-4, P-8) параллельно; затем P-2 → P-3 (паритет до захвата решения); P-5 и P-7 несут version-bump — пустить вместе под **одним консолидирующим миграционным гейтом** (P-5 cell-id, P-7 backup-v2, P-7 audit-v2 — три on-disk-слома должны выйти когерентно, иначе тихо ломается byte-identical инвариант между версиями). MUST-FIX-трек (P-1..P-6) и крипто-трек (P-7) **не делят код** и идут параллельно; их сводит только receipt-сборка, которой нужны оба.

### 4. Метрики успеха (измеримые)

- **Корректность доказательств:** `cosine(v,−v)==0` и `metric.rs==dedup.rs` на общей фикстуре; нормализованно-равные значения дают **0** ложных конфликтов, истинные numeric-конфликты помечаются на ≥2 форматах payload; на размеченном корпусе VERIFY — **conflict recall ≥ 0.90, false-conflict ≤ 0.05** (заменяет alpha-дисклеймер `docs/VERIFY_FACT.md:297-308`).
- **Целостность enforcement:** `persisted allowed == eval_bitmap_program(plan) ∩ vector_cells` на матрице scope×status×WHERE; у каждой допущенной ячейки `decision==Allowed` с непустым `policy_version`; **0** ячеек вне scope в sparse-scope-кейсах; recall@k для самой разрежённой scope-децили в пределах фиксированного epsilon от exact-пути.
- **Честность неполноты:** при ANN `budget_exceeded` (fallback-off) pack несёт `RetrievalIncomplete`, проходящую через все export-форматы.
- **Реальная крипта:** **ноль** FNV/XOR integrity-рутин в production backup/audit (grep-гейт); tamper любого байта ciphertext/tag/nonce/salt/AAD проваливает `open()` без утечки plaintext; tamper прошлого audit-события и подделка цепи без ключа детектируются; legacy v1-форматы отвергаются на чтении, никогда не доверяются молча.
- **Детерминизм-фундамент:** `canonical_bytes()` стабилен под перестановкой ключей; **ни одного** wall-clock-байта в хэшируемой поверхности; все три on-disk-слома (cell-id, backup, audit) — за version-bump с миграцией/refuse-to-read.
- **Гейт допуска:** `correctness-prerequisites-check` + `crypto-foundation-check` + `canonical-serialization-check` зелёные и в release-lane **до** мёрджа любой receipt-сборки.

### 5. Топ-риски этого блока

1. **Подпись над повреждёнными доказательствами** — худший режим всей программы: баг превращается в нотариально заверенную ложь. *Митигейт:* P-1..P-6 — блокирующие для эмиссии receipt, не параллельные «к релизу».
2. **Неоднозначная канонизация** молча форкает байты БД↔верификатора. *Митигейт:* закрепить JCS/явную integer-форму в нормативном doc с golden-векторами; верификатор ре-деривирует канонизацию из спеки, не импортируя engine-модуль.
3. **Скрытый недетерминизм** (итерация HashMap, форматирование float, локаль, остаточный wall-clock) протекает в хэш-корень. *Митигейт:* только `BTreeMap/BTreeSet`-источники через единый canonical-модуль; field-exclusion grep; прогон гейта детерминизма 3× cross-process в CI.
4. **Первые крипто-зависимости** стопорятся на allowlist/вендоринге/энтропии CI-песочницы, блокируя весь спайн crypto→receipt→verifier. *Митигейт:* `crypto-deps-policy-check` — самая первая крипто-задача; подтвердить allowlist + энтропию до downstream; feature-флаг, чтобы не-крипто-сборки не страдали.
5. **Некогерентные on-disk-сломы** (backup-v2, audit-v2, cell-id) выходят вразнобой и тихо нарушают cross-version byte-identity, либо оставляют legacy XOR-FNV/FNV молча доверяемыми. *Митигейт:* один консолидирующий migration-gate; legacy-форматы **refuse-on-read** с типизированной ошибкой.
6. **Преждевременная заморозка схемы receipt** до финализации листьев вынуждает v2-bump, когда P-3 добавит `policy_version`, P-6 — `RetrievalIncomplete`, нормализация — temporal/citation conflict-kinds. *Митигейт:* не замораживать `accountability_receipt.v1`, пока MUST-FIX и P-3 не приземлились; ревью `context/mod.rs` + `verification/types.rs` поле-за-полем.

## Приложение A. Детализация столпов: задачи, exit-гейты, усилия, зависимости

### A.1 Verifiable Accountability Receipt (the category-defining artifact) · **категориообразующий**

**Текущее состояние:** CortexDB emits most of the SEMANTIC inputs a receipt must bind, but has ZERO of the verifiability layer.

WHAT EXISTS (semantic substrate, partial-to-strong):
- Per-cell access decision is a typed first-class output: `ContextAccessDecision{cell_id,decision,policy,reason,scope,scope_id,agent_id}` with outcome enum `Allowed|NotRecorded` (crates/cortex-engine/src/context/mod.rs:108-132). It is exported (crates/cortex-engine/src/context/export/json_export.rs:25-33). DEFECT: it is a pack-time RE-DERIVATION via `PolicyRewrite::allows_scope` (crates/cortex-engine/src/context/pack/access.rs:14-15), not a captured record of the binder's actual enforcement, and `NotRecorded` carries no re-checkable denial reason or policy version.
- Span provenance is typed and exported: `ContextSpanProvenance{source_cell_id,source_byte_start/end,source_line_start/end,source_ref}` (context/mod.rs:134-142, built in crates/cortex-engine/src/context/span.rs:49-72). DEFECT: only populated when `span_level_packing` trims an over-budget cell (span.rs:32-38); full-cell packs ship `provenance:None`, and citation is a free-text string never bound to source bytes.
- VERIFY FACT is deterministic and typed: `VerificationReport{fact,status,confidence_q16,evidence,contradicting_evidence,guards,numeric_conflicts}` (crates/cortex-engine/src/verification/types.rs:60-103). DEFECT: the execution report embeds wall-clock `total_elapsed_nanos`/`elapsed_nanos` via `Instant::now()` (crates/cortex-engine/src/verification/operator.rs:1,28,37,188,203,207) — non-deterministic, MUST be excluded from any hashed surface.
- Fail-closed binder is genuinely strong: every retrieve plan is seeded `[PushAgentAllowed, PushLive, And]` and user WHERE is only AND-ed in (crates/cortex-aql/src/binder.rs:137-145), producing a `BitmapProgram{ops,max_stack_depth}` (binder.rs:30-45). This is the property the receipt's access leaves must attest. It produces NO signed/hashed attestation today.
- ContextPack JSON is stable-ish and versioned: `to_json` uses fixed-key `json!` with `schema_version:"context_pack.v1"` and omits clocks (json_export.rs:68-81). The frozen schema explicitly permits additive optional fields until v2 (docs/schemas/context_pack.v1.json:5). BUT `json!` does NOT recursively sort map keys or canonicalize numbers/strings — there is no receipt-grade canonicalizer.
- Determinism is regression-tested as in-repo snapshots only: crates/cortex-engine/tests/determinism.rs asserts pack/verification snapshots are repeatable and survive checkpoint, and there is a make-gate `engine-determinism-check` (mk/core-contracts.mk:95-96). No EXPORTED determinism hash, no cross-process artifact.
- AgentView (the principal projection) has stable BTreeSet ordering: agent_id, readable_brains/scopes, allowed_modes/memory_types, budgets, allow_verify_fact, private_scope (crates/cortex-aql/src/agent_view.rs:6-24).

WHAT IS MISSING (the entire verifiability closure):
- NO canonical serialization, NO content hash, NO signature over any pack/report. `to_json` is a convenience string.
- ZERO real cryptographic primitives in the whole workspace: only `roaring` appears in any Cargo.toml; no sha2/blake3/ed25519/hmac/ring/getrandom anywhere (confirmed grep across all crates/*/Cargo.toml and root Cargo.toml). Every integrity surface is hand-rolled FNV-1a64: audit chain `event_hash` is unkeyed FNV-1a64 (crates/cortex-server/src/audit_chain.rs:8-9,42-65) and commits only HTTP-envelope metadata + COUNTS (never cell_ids, citations, verdict, or a pack hash); encrypted backup is XOR+FNV keystream with a forgeable FNV `auth_tag` and a single-pass FNV `KDF` (crates/cortex-engine/src/backup/encrypted/crypto.rs:3-66) — not AEAD, not a MAC.
- Cell identity is not DB-authenticated: `content_hash` is an OPTIONAL self-asserted metadata string parsed from payload (crates/cortex-core/src/cell/descriptor.rs:20; crates/cortex-engine/src/query/metadata/types.rs:31), not a computed digest. A receipt cannot trust it.
- No standalone offline verifier exists. CLI dispatch lives in crates/cortex-cli (subcommands like cli_audit_chain.rs); there is no receipt verifier binary and no accountability_receipt schema.

Project gate culture is strong and well-defined: make-gates named `<area>-<facet>-check`, each backed by a Python script emitting a JSON report and/or `cargo test`, aggregated into `alpha-check`/`release-check` lanes (mk/core-contracts.mk, mk/release.mk:10-46). New gates must follow this convention to land in the release lane.

**Целевое состояние:** Every ContextPack and VerificationReport carries an additive, optional top-level `accountability_receipt` (schema `accountability_receipt.v1`) that is a deterministic, BLAKE3-Merkle-committed, Ed25519-signed bundle, plus a published spec and a standalone offline verifier binary (`cortex-receipt-verify`) that links NONE of the engine internals.

The receipt header (the only signed bytes, fixed-size) is `{schema_version:"accountability_receipt.v1", hash_alg:"blake3-256", sig_alg:"ed25519", db_instance_id, key_id, created_unix_seconds, access_root, provenance_root, cell_set_root, verification_root, budget_commitment, conflict_commitment, pack_root, determinism_hash, signature}`.

It binds, as ordered Merkle trees (leaves sorted by cell_id then fixed sub-key; leaf = blake3(domain_tag_byte || JCS_canonical_bytes(claim))):
1. ACCESS leaf per cell from ContextAccessDecision {cell_id,decision,policy,scope_id,agent_id,policy_version}; verifier REJECTS any pack cell whose access leaf != "allowed".
2. PROVENANCE leaf per cell {cell_id,source_cell_id,byte_start,byte_end,line_start,line_end,source_ref,citation,cell_content_hash}.
3. CELL_SET leaf per admitted cell {cell_id, cell_content_hash = blake3(canonical cell bytes)} — DB-computed identity anchor, NOT the payload-embedded content_hash.
4. VERIFICATION leaves {status,confidence_q16} + per-evidence {cell_id,match_kind,match_score_q16,source_trust_q16} + per-conflict {cell_id,metric,left,right}, with all elapsed_nanos EXCLUDED.
5. BUDGET commitment over {token_budget_tokens, estimated_tokens=sum(cell.estimated_tokens), truncated, per-cell (cell_id,estimated_tokens)}.
6. CONFLICT commitment over {conflict_visibility_q16, visible_conflict_count, anomalies[]} so VisibleConflict preservation is attested.

pack_root = blake3(access_root||provenance_root||cell_set_root||verification_root||budget_commitment||conflict_commitment) binds OUTPUT. determinism_hash = blake3(domain||canonical(query)||canonical(AgentView projection)||canonical(ContextPackOptions)||bitmap_program) binds INPUT. The signature over the header binds both, so "same inputs => byte-identical pack+receipt" is externally checkable.

Threat model (what a malicious/buggy DB MUST NOT be able to forge, each caught by a specific verifier step): admit an unreadable cell; cite a span not in the referenced bytes; drop/hide a VisibleConflict or overstate the verdict; overspend/misreport budget; claim determinism while returning a different pack; replay another query's receipt; tamper any field post-hoc. OUT OF SCOPE (documented): DB equivocation (two contradictory-but-individually-valid signed receipts) — mitigated optionally by an append-only transparency log of pack_root; and the receipt proves internal consistency, NOT that underlying cells are factually true.

A new make-gate `accountability-receipt-check` (composed of schema-freeze, determinism, tamper, and independent-verifier sub-gates) is wired into the alpha/release lane. This is the closure a thin pgvector+policy library structurally cannot emit.

| ID | Задача | Exit-гейт | Усилие | Зависит | Риск |
|---|---|---|---|---|---|
| `AR-1` | **Add real crypto deps + canonical serialization (JCS) module behind a feature flag** — Add blake3 (or sha2) and ed25519-dalek as the FIRST crypto dependencies in the workspace (root Cargo.toml workspace.dependencies; consumed by cortex-engine), gated behind a cargo feature `accountability-receipt`. Implement a normative canonical-bytes routine (RFC 8785 JCS or an explicit documented byte form: integers/Q16 only, recursively sorted keys, no floats, no timestamps) in a new module crates/cortex-engine/src/accountability/canonical.rs. This is the agreement point between DB and verifier. Do NOT reuse FNV from audit_chain.rs or the XOR-FNV backup. Define domain-tag bytes for each leaf category. | `make accountability-canonical-check (new): a golden-vector test asserts canonical(value) is byte-stable across two runs and across key-insertion-order permutations of the same logical object; cargo tree confirms blake3+ed25519-dalek are the only new crypto crates and only under the feature flag.` | M | — | Choosing a canonicalization that later proves ambiguous (e.g. unicode normalization, number forms) would silently fork DB vs verifier. Mitigate by pinning JCS/RFC 8785 and shipping cross-language golden vectors. |
| `AR-2` | **Define accountability_receipt.v1 schema + frozen JSON Schema, additive to context_pack.v1** — Write docs/schemas/accountability_receipt.v1.json describing the fixed-size signed header {schema_version, hash_alg, sig_alg, db_instance_id, key_id, created_unix_seconds, access_root, provenance_root, cell_set_root, verification_root, budget_commitment, conflict_commitment, pack_root, determinism_hash, signature} and the per-leaf claim shapes. Add `accountability_receipt` as ONE optional top-level field on context_pack.v1 (honoring the additive-until-v2 clause at docs/schemas/context_pack.v1.json:5). Provide a golden receipt fixture. Document the spec (what each root binds, the canonicalization, the threat model in/out of scope) in docs/. | `make accountability-receipt-schema-check: a Python script (scripts/accountability_receipt_schema_check.py, JSON report per project convention) validates the golden fixture against the schema, asserts the field is additive (a context_pack.v1-only validator still passes on a pack carrying the receipt), and freezes the schema like other docs/schemas gates.` | M | AR-1 | Schema lock-in before the leaf set is final forces a v2 bump. Mitigate by reviewing the full claim set against mod.rs/types.rs field-by-field before freezing. |
| `AR-3` | **Compute DB-authenticated cell_content_hash for admitted cells** — Add a DB-computed cell_content_hash = blake3(canonical cell bytes) per admitted cell, surfaced on ContextPackCell (additive field on crates/cortex-engine/src/context/mod.rs:97-106). Do NOT trust the optional payload-embedded content_hash (descriptor.rs:20). This is the identity anchor for the cell_set_root and the precondition for citation-span verification. | `make accountability-cell-hash-check: a test asserts cell_content_hash is deterministic for fixed bytes, differs on a one-byte payload change, and is independent of any payload-embedded content_hash string (set them inconsistent in a fixture and confirm the DB-computed value is used).` | S | AR-1 | Canonical cell-byte definition must match exactly what the verifier is handed; if the verifier receives raw stored bytes but the DB hashes a normalized form, hashes diverge. Pin one cell-byte canonical form and document it. |
| `AR-4` | **Build the receipt: five Merkle trees + pack_root + determinism_hash** — Implement crates/cortex-engine/src/accountability/receipt.rs that consumes a built ContextPack (+ optional VerificationReport + the BoundRetrievePlan's bitmap_program + AgentView + ContextPackOptions + query) and produces the receipt body: access_root, provenance_root, cell_set_root, verification_root, budget_commitment, conflict_commitment via ordered Merkle folds; pack_root over the six; determinism_hash over canonical(query, AgentView projection, ContextPackOptions, bitmap_program). EXCLUDE all elapsed_nanos/wall-clock (verification/operator.rs:28,188,203). Promote AnswerGroundingReport and the ANN budget_exceeded completeness flag into hashed pack outputs so they are inside the receipt. For full-cell packs (no span trim), still bind cell_content_hash and a whole-cell provenance leaf. | `make accountability-receipt-determinism-check: RETRIEVE CONTEXT + VERIFY FACT run twice on a fixed store yield byte-identical receipt bodies (all roots equal); a field-exclusion test greps the hashed surface and fails if any elapsed_nanos/SystemTime byte is present.` | L | AR-1, AR-2, AR-3 | Hidden non-determinism (HashMap iteration, float formatting, locale) leaks into a root. Mitigate by routing every hashed value through the AR-1 canonical module and asserting BTreeMap/BTreeSet-only sources. |
| `AR-5` | **Ed25519-sign the fixed-size header; key_id custody + rotation** — Sign ONLY the canonical header bytes (the roots + metadata) with Ed25519 using RFC 8032 deterministic nonces (ed25519-dalek). Establish a node keystore with a rotatable key_id and a way to publish/distribute the public key (file + optional transparency-anchor stub). Keep the signature O(1)-size; all leaf detail is re-derivable by the verifier from public data. Wire the signed receipt into the engine->server emission path so it attaches to the ContextPack/VerificationReport response (additive). | `make accountability-receipt-sign-check: same key + same inputs => byte-identical signature (deterministic-nonce assertion); a wrong/rotated key_id is detected; the signed receipt round-trips through the server JSON response and re-parses.` | M | AR-4 | Key custody for a single-node beta is weak; a leaked node key forges all receipts. Document key rotation + optional transparency log; do not block the pillar on full HA but state the equivocation caveat. |
| `AR-6` | **Capture real access-decision enforcement (not pack-time re-derivation) into the access leaves** — Replace the pack-time re-derivation in crates/cortex-engine/src/context/pack/access.rs (which re-calls allows_scope) with a CAPTURED record emitted by the binder/scan path: the actual scope decision + policy_version + AgentView digest that admitted (or excluded) each cell. Fold excluded/denied cells' decisions into the receipt so the trail reflects what retrieval truly did. Add policy_version to ContextAccessDecision (context/mod.rs:109-117). | `make accountability-access-capture-check: a test where the binder excludes a cell asserts the receipt's access trail records the exclusion with a reason a third party can re-evaluate, and that no admitted cell has decision==NotRecorded (verifier-reject condition); policy_version is present and non-empty on every access leaf.` | L | AR-4 | Threading a captured decision from binder.rs through the scan into the pack touches the hot retrieval path and several crates. Mitigate by carrying a compact decision token, not the full struct, until pack assembly. |
| `AR-7` | **Standalone offline verifier binary cortex-receipt-verify (+ spec-conformant algorithm)** — Add a new binary `cortex-receipt-verify` (own crate or a cortex-cli bin that does NOT link cortex-engine) implementing the 7-step verifier using ONLY {pack JSON, receipt, raw bytes/content-hashes of admitted cells, DB public key}: (1) verify Ed25519 over canonical header; (2) recompute every leaf + root from the JSON and assert they fold to the signed roots; (3) reject if any admitted cell's access leaf != allowed; (4) for each cited span assert 0<=byte_start<=byte_end<=len(cell) AND the span substring is present at that offset; (5) assert sum(estimated_tokens)<=token_budget_tokens and truncated consistency; (6) assert verdict/conflicts match bound verification leaves and reference only admitted cell_ids; (7) recompute determinism_hash and (if able to re-run) require byte-identity, else accept as commitment. It must re-implement canonicalization from the AR-1 spec, not import the engine. | `make accountability-receipt-verify-check: the binary accepts the golden fixture using only public inputs, exits non-zero on a tampered fixture, and a dependency-graph assertion in the gate script confirms it does not depend on cortex-engine/cortex-storage.` | L | AR-2, AR-4, AR-5 | If the verifier shares code with the engine it can mask engine bugs (both wrong the same way). Enforce the no-engine-dependency rule and re-derive canonicalization independently from the published spec. |
| `AR-8` | **Tamper/adversarial suite + compose accountability-receipt-check into the release lane** — Add a table-driven mutation suite mirroring crates/cortex-engine/tests/determinism.rs: flip one estimated_tokens, flip one access decision to NotRecorded, shift one source_byte_start, drop one VisibleConflict, swap the verdict against a different pack, replay a receipt under a different query/AgentView, flip one signature/ciphertext byte — verifier MUST reject each and accept the genuine pack. Add scripts/accountability_receipt_check.py emitting a JSON report. Compose the umbrella gate `accountability-receipt-check` = schema-check + determinism-check + tamper-check + verify-check + a grep gate proving no FNV/XOR-FNV backs the receipt; wire it into alpha-check in mk/release.mk (and add to mk/core-contracts.mk style). | `make accountability-receipt-check is green and present in alpha-check; the tamper suite shows 100% rejection of mutated receipts and acceptance of the genuine one; the no-FNV grep gate passes.` | M | AR-2, AR-4, AR-5, AR-7 | Gate could pass trivially if mutations are too weak (e.g. mutate a non-hashed field). Mitigate by asserting each mutation targets a hashed leaf and that removing the verifier check makes that case FAIL (mutation-of-the-mutation-test). |

**Измеримые критерии готовности столпа:**
- make accountability-receipt-check is green and wired into alpha-check (mk/release.mk), composed of the four sub-gates below.
- make accountability-receipt-schema-check: docs/schemas/accountability_receipt.v1.json exists, is frozen, validates a golden receipt fixture, and is confirmed additive-only against context_pack.v1 (a v1-only consumer ignores the new top-level field).
- make accountability-receipt-determinism-check: RETRIEVE CONTEXT and VERIFY FACT run twice on a fixed store produce byte-identical receipts including byte-identical Ed25519 signatures (RFC 8032 deterministic nonces); no elapsed_nanos or wall-clock byte appears anywhere in the hashed surface (asserted by a field-exclusion test).
- make accountability-receipt-tamper-check: a table-driven mutation suite (flip one estimated_tokens; flip one access decision Allowed->NotRecorded; shift one source_byte_start; drop one VisibleConflict; swap verdict; replay a receipt under a different query/AgentView; flip one ciphertext/signature byte) — the standalone verifier MUST reject EVERY case and accept the unmutated golden.
- make accountability-receipt-verify-check: the standalone cortex-receipt-verify binary, given ONLY {pack JSON + receipt + raw bytes (or content hashes) of admitted cells + DB public key}, validates 100% of fixture receipts and links NO cortex-engine internals (dependency-graph assertion in the gate script).
- Zero FNV-1a64 / XOR-FNV primitive backs any receipt leaf, root, determinism hash, or signature (grep gate over the receipt module).
- cargo grep confirms exactly one real hash crate (blake3 or sha2) and one signature crate (ed25519-dalek) are the sole crypto deps used by the receipt path; the feature is behind a cargo feature flag for the first release.

**Зависимости:** MUST-FIX prerequisites pillar: cosine dot.abs() bug (crates/cortex-engine/src/search/hnsw/metric.rs:44) and sparse-scope guarded-ANN recall (crates/cortex-engine/src/search/hnsw/search_impl.rs:33-120) must be fixed first OR their incompleteness surfaced as a hashed anomaly — a signed receipt over corrupted/incomplete evidence is a notarized lie. The budget_exceeded completeness flag must reach the ContextPack so the receipt can attest retrieval completeness.; Real-crypto foundation: this pillar introduces the first crypto crate (blake3/sha2 + ed25519-dalek) to the workspace; the audit-chain MAC and AEAD-backup pillars depend on the same keystore/key_id custody this pillar establishes (replacing audit_chain.rs FNV and backup/encrypted/crypto.rs XOR-FNV).; Determinism foundation: verification/operator.rs elapsed_nanos must be moved out of the hashed surface; this pillar depends on a canonical-serialization (JCS/RFC 8785) module being the normative wire format, since the existing json! export does not recursively sort keys.; Access-decision fidelity: full value of access_root depends on the binder/scan path emitting a CAPTURED enforcement record (with policy_version) rather than the current pack-time re-derivation in context/pack/access.rs.

### A.2 Provable fail-closed governance end-to-end · **категориообразующий**

**Текущее состояние:** Fail-closed is STRONG at the binder but UNPROVEN end-to-end, and there are two divergent enforcement paths that the receipt cannot currently reconcile.

(1) Binder (proven-by-construction): crates/cortex-aql/src/binder.rs:137-145 seeds every retrieve plan with [PushAgentAllowed, PushLive, And]; an optional WHERE is compiled then ops.push(BitmapOp::And) at :144 — user predicates can only intersect. eval_bitmap_program (crates/cortex-aql/src/executor_mock.rs:43-83) is a stack VM where And does `lhs &= &rhs` (:61), so the program cannot widen. This produces the `candidates` set used by the in-memory/exact retrieve path (crates/cortex-engine/src/exec/scans.rs:23 BitmapIndexScan; PermissionFilter:80-92 re-intersects agent_allowed).

(2) Persisted ANN/lexical (DIVERGENT, weaker): crates/cortex-engine/src/search/database/persisted.rs:53 and crates/cortex-engine/src/search/evaluation.rs:42 build `allowed` from crates/cortex-engine/src/search/access.rs:9-18 `allowed_candidates`, which unions ONLY readable_scopes bitmaps — it does NOT compose PushLive (status) or the WHERE narrowing the binder enforces. So the persisted path enforces scope but on a different, broader basis than the proven plan algebra; the two enforcement surfaces are not provably equal.

(3) ANN is a scope-blind graph walk with a POST-traversal filter: crates/cortex-engine/src/search/hnsw/search_impl.rs:33-119 expands neighbors regardless of scope (:94-116) and only applies `allowed.contains` at scoring time (:90), under a shared visit budget max_visited (:74-77). A sparse-scope agent burns budget on out-of-scope nodes and can return few/zero in-scope hits. budget_exceeded IS computed and surfaced into AnnSearchReport (search/ann/search.rs:116-143, 224) and triggers an exact fallback when policy.fallback is on — but with fallback off (the no-fallback production track) it returns incomplete silently.

(4) Incompleteness is NOT disclosed in the result: ContextPackAnomalyCode (crates/cortex-engine/src/context/mod.rs:163-184) has ScopeMismatch and InsufficientContext variants but ONLY TokenOverload is ever pushed (crates/cortex-engine/src/context/pack/builder.rs:147); budget_exceeded never crosses into the ContextPack, so an answer cannot say 'retrieval may be incomplete'.

(5) access_decision is a pack-time RE-DERIVATION, not captured enforcement: crates/cortex-engine/src/context/pack/access.rs:8-46 re-runs PolicyRewrite::allows_scope at pack time and emits Allowed/NotRecorded; it can drift from what the binder/scan actually did and carries no policy/AgentView version.

(6) Cosine soundness bug feeds every scope-sensitive ranking: crates/cortex-engine/src/search/hnsw/metric.rs:44 uses dot.abs() so anti-correlated vectors score as perfect matches; a correct signed-clamp cosine already exists at crates/cortex-engine/src/context/dedup.rs:20-51.

Gate culture is strong (hundreds of `*-check` gates; the existing scope gate is context-pack-private-scope-check -> crates/cortex-engine/tests/context_pack_private_scope.rs + scripts/context_pack_private_scope_check.py), and determinism is snapshot-tested (crates/cortex-engine/tests/determinism.rs) — both are the right hooks to extend.

**Целевое состояние:** A single, formally-modeled fail-closed invariant that holds byte-for-byte from binder through every physical scan and ANN, is adversarially benchmarked, and is disclosed when it cannot be fully satisfied — so the receipt's access_root attests an enforcement that provably happened.

Concretely: (a) the persisted ANN/lexical `allowed` set is derived from the SAME bitmap program the binder produced (live + WHERE composed), so the two enforcement paths are provably equal, not merely both scope-correct; (b) ANN no longer post-filters under a shared budget — sparse-scope agents get a pre-filtered/partitioned subgraph or an exact top-k fallback when |allowed| is small, with an independent budget, so recall does not collapse for the most security-sensitive agents; (c) budget_exceeded and any scope-narrowing-induced incompleteness surface as a first-class ContextPack anomaly (InsufficientContext / a new RetrievalIncomplete) that is part of the (eventually signed) pack, so 'this answer may be incomplete' is disclosed, never silent; (d) the cosine metric is fixed so scope-admitted ranking is sound; (e) a scope-leak benchmark over EVERY output surface (pack cells, citations, explain, anomalies, numeric_conflicts/VERIFY evidence, and error messages) proves zero forbidden-scope bytes across query shapes, export formats, checkpoint/compact, and budget-exhaustion adversarial cases; (f) a machine-checked formal-invariant model (property test / proof harness) establishes 'no bound plan and no scan can admit a cell outside [PushAgentAllowed AND PushLive AND WHERE]', and its model hash is exported so the receipt attests the property, not just asserts it; (g) access_decision is a captured enforcement record (with policy/AgentView version) emitted by the scan path, with denied/excluded cells foldable into the receipt.

| ID | Задача | Exit-гейт | Усилие | Зависит | Риск |
|---|---|---|---|---|---|
| `FC-1` | **Fix cosine metric soundness (dot.abs bug) for scope-admitted ranking** — In crates/cortex-engine/src/search/hnsw/metric.rs:28-45 the Cosine arm returns ((dot.abs() * 65_535) / norm.abs()), so anti-correlated vectors score as perfect matches and corrupt which in-scope cells are surfaced. Replace with the signed-clamp logic already proven in crates/cortex-engine/src/context/dedup.rs:20-51: return 0 when dot<=0, widen the *65_535 step to i128/u128 to avoid overflow on high-dim i16 vectors, clamp to 65_535. Keep output monotonic (higher=better) so all callers (search_impl.rs:59,82,87) are unaffected. This is a fail-closed concern, not just quality: a wrong metric silently changes which scope-admitted evidence the receipt attests as 'most relevant'. | `make hnsw-cosine-correctness-check (new): cosine(v,-v)==0, cosine(v,v)==max, ranked search over {v, orthogonal, -v} orders v first / -v last, and a max-magnitude high-dim vector does not overflow; metric.rs and dedup.rs cosine agree on a shared fixture.` | S | — | Low. Single pure function; ranking is monotonic so callers are unaffected. Minor risk: existing ANN recall snapshots (ann_recall_tests.rs) shift and need re-baselining — acceptable since current baselines encode the bug. |
| `FC-2` | **Derive persisted ANN/lexical allowed-set from the bound bitmap program, not readable_scopes alone** — crates/cortex-engine/src/search/access.rs:9-18 allowed_candidates unions ONLY readable_scopes, and is consumed by crates/cortex-engine/src/search/database/persisted.rs:53 and crates/cortex-engine/src/search/evaluation.rs:42. This bypasses PushLive (status) and the user WHERE that the binder's bitmap program (binder.rs:137-145) enforces, making the persisted path a parallel, weaker enforcement surface than the proven plan algebra. Plumb the BoundRetrievePlan's bitmap_program (or its evaluated RoaringBitmap via eval_bitmap_program_bitmap) into the persisted search entry points and intersect it with the vector-bearing cell set to form `allowed`, so the ANN/lexical admitted basis is provably equal to the binder's. Keep allowed_candidates only for code paths that genuinely have no plan (and document that). | `make ann-scope-parity-check (new): for a matrix of scope/status/WHERE fixtures, assert persisted ANN and lexical `allowed` == eval_bitmap_program(plan) ∩ vector_cells; include a case where a live=false / WHERE-excluded but readable-scope cell is currently admitted by the old path and is now excluded.` | M | — | Medium. Threads plan/bitmap through several search entry points; must not regress the changed-after-checkpoint skip logic (persisted.rs:38-47). Risk of double-filtering vs PermissionFilter — verify single source of truth. |
| `FC-3` | **Eliminate ANN post-filter recall collapse for sparse-scope agents** — crates/cortex-engine/src/search/hnsw/search_impl.rs:33-119 walks the graph scope-blind (:94-116) and applies allowed.contains only at scoring (:90), under a shared visit budget (max_visited, :74-77). A narrow-scope (most security-sensitive) agent exhausts budget on out-of-scope nodes and returns few/zero in-scope hits. Implement: (a) an exact brute-force top-k fallback when \|allowed\| is below a documented threshold (cheap — that is exactly the sparse case; search_persisted_vectors already exists and is used by the fallback in search/ann/search.rs:173); and/or (b) give the allowed-set traversal an independent budget so out-of-scope visits do not starve in-scope discovery. Surface the chosen strategy in AnnSearchReport. | `make ann-sparse-scope-recall-check (new): across scope-density deciles, recall@k for the sparsest decile >= exact-path recall@k minus a fixed epsilon, with no budget-induced silent drop; \|allowed\| below threshold provably routes to exact top-k.` | L | FC-1, FC-2 | Medium-High. Touches the hot ANN path and budget accounting; exact fallback for small allowed-sets is cheap, but the independent-budget path risks latency regressions on broad-scope agents. Full per-scope subgraph partitioning (index build change) is deliberately deferred to a follow-up. |
| `FC-4` | **Surface retrieval incompleteness as a first-class ContextPack anomaly** — budget_exceeded is computed (search/ann/search.rs:116-143) and reaches AnnSearchReport but never crosses into the ContextPack; ContextPackAnomalyCode (context/mod.rs:163-184) defines ScopeMismatch and InsufficientContext but only TokenOverload is ever pushed (context/pack/builder.rs:147). Add a RetrievalIncomplete code (or reuse InsufficientContext) and push an anomaly whenever ANN budget_exceeded with fallback disabled, or when scope/WHERE narrowing dropped candidates that could not be re-derived. Thread the AnnSearchReport completeness signal into the ContextPackBuilder. Ensure it serializes through every export format (context/export/json_export.rs and the explain mapping at context/explain.rs:109). | `make context-pack-retrieval-incomplete-check (new): a sparse-scope, budget-exhausted, fallback-off retrieve yields a ContextPack with a RetrievalIncomplete/InsufficientContext anomaly that round-trips through every ContextPackExportFormat; a complete retrieve yields none.` | M | FC-3 | Low-Medium. Mostly plumbing a boolean/enum up. Risk: anomaly text could itself leak scope detail — keep messages scope-byte-free (covered by FC-6). |
| `FC-5` | **Capture access_decision from the enforcement path with policy/AgentView version** — crates/cortex-engine/src/context/pack/access.rs:8-46 RE-DERIVES the decision at pack time via PolicyRewrite::allows_scope, which can drift from what the binder/scan actually enforced and carries no policy/AgentView version. Change the retrieve/scan path (exec/scans.rs PermissionFilter:80-92 and the candidate resolution in database/read.rs) to emit a captured ContextAccessDecision tagged with the AgentView digest / policy version that admitted the cell, and have the builder consume that captured record instead of re-deriving. Fold a (bounded) record of WHERE/scope-excluded cells into a structure the receipt pillar can hash as the denied set. On a successful AQL retrieve, no admitted cell should ever be NotRecorded. | `make context-access-decision-capture-check (new): every admitted cell on the AQL path has decision==Allowed with a non-empty policy_version/agent_view_digest; assert the decision originates from the scan path (e.g. tamper a cell's scope post-scan and confirm the captured decision, not a re-derivation, is reported).` | M | FC-2 | Medium. Requires carrying a version/digest from AgentView through scans; must stay deterministic (no clocks) so the receipt hash is stable. Coordinate the digest definition with the receipt pillar to avoid rework. |
| `FC-6` | **Scope-leak benchmark across EVERY output surface** — Extend the existing private-scope harness (crates/cortex-engine/tests/context_pack_private_scope.rs + scripts/context_pack_private_scope_check.py, gate context-pack-private-scope-check) into a full scope-leak benchmark that plants a unique sentinel (like PRIVATE_SCOPE_SHOULD_NOT_LEAK) in forbidden-scope cells and asserts the sentinel appears in ZERO of: pack cell payloads, citations, provenance source_ref (url/json_path), explain why_selected/matched_terms/score_components, anomalies (message/why_excluded), VERIFY evidence + numeric_conflicts (verification/export.rs), and EngineError safe_message. Run the matrix over multiple agents x query shapes (broad, explicit-forbidden, WHERE-narrowed) x all export formats x {in-memory, post-checkpoint, post-compact} x {fallback on/off, budget exhausted}. Reuse the determinism harness pattern for stable snapshots. | `make scope-leak-bench-check (new): 0 sentinel occurrences across all surfaces for >=200 (agent x query x format x persistence x budget) combinations; report JSON written like other context-pack gates; wired under context-pack quality and the new fail-closed aggregate gate.` | L | FC-2, FC-4, FC-5 | Medium. Largest test-surface effort; main risk is missing a leak surface (e.g. a new export field) — mitigate by enumerating output fields programmatically and failing on any un-scanned string field. |
| `FC-7` | **Machine-checked fail-closed formal-invariant model with exported model_hash** — Build a property/proptest harness (in cortex-aql, alongside crates/cortex-aql/tests/binder_tests.rs, plus an engine-level scan property test) that, over randomized catalogs, AgentViews, status sets, and WHERE clauses, asserts the invariant: admitted_set ⊆ agent_allowed ∩ live ∩ where for BOTH the bitmap-program path and the persisted ANN/lexical path (proving FC-2's equality holds generally, not just on fixtures). Model the binder seed [PushAgentAllowed, PushLive, And] and the And-only WHERE composition as the spec. Emit a stable, content-addressed model_hash over the invariant statement + the bitmap-op semantics so the receipt pillar can bind 'this answer was produced under the proven fail-closed model' rather than asserting it. This is the keystone that turns fail-closed from code into an attested property. | `make fail-closed-invariant-model-check (new): proptest finds no counterexample to admitted ⊆ agent_allowed ∩ live ∩ where over N randomized cases for both paths, and prints a deterministic model_hash that a unit test pins.` | L | FC-2, FC-3 | Medium. Writing a faithful generator for catalogs/views/WHERE is non-trivial; risk of a vacuous test (always-empty admitted set) — require positive cases where in-scope cells ARE admitted. model_hash format must be agreed with the receipt pillar. |
| `FC-8` | **Aggregate fail-closed end-to-end gate and wire into release lane** — Add a single umbrella gate make fail-closed-end-to-end-check that runs FC-1..FC-7's gates plus the existing context-pack-private-scope-check and engine-determinism-check, writes a consolidated report, and is added to the beta release lane (mk/core-retrieval-context.mk and/or mk/core-security-ops.mk) next to the existing context-pack quality aggregation. This makes 'no byte outside scope, measured, end-to-end' a single release blocker consistent with the repo's gate culture. | `make fail-closed-end-to-end-check is green, listed in .PHONY, invoked by the beta release aggregate, and its report path is registered in mk/vars-core.mk like CONTEXT_PACK_PRIVATE_SCOPE_REPORT.` | S | FC-1, FC-2, FC-3, FC-4, FC-5, FC-6, FC-7 | Low. Pure orchestration. Risk: CI time growth from the >=200-combination benchmark — mitigate with a CI-safe mini subset (like the existing balanced_50 pattern) and a nightly full run. |

**Измеримые критерии готовности столпа:**
- make fail-closed-end-to-end-check is green and wired into the beta release lane (mk/core-retrieval-context.mk or mk/core-security-ops.mk), aggregating all sub-gates below.
- make scope-leak-bench-check proves 0 forbidden-scope bytes across ALL output surfaces (pack payloads, citations, provenance source_ref, explain why_selected/matched_terms, anomalies, VERIFY evidence/numeric_conflicts, and EngineError safe_message) for >=200 adversarial (agent x query-shape x export-format) combinations, including budget-exhaustion and no-fallback cases, before AND after checkpoint+compact.
- make ann-scope-parity-check proves the persisted ANN/lexical allowed-set equals eval_bitmap_program(plan) restricted to vector-bearing cells for a fixture matrix of WHERE/status/scope combinations (i.e. the persisted path composes PushLive and WHERE, not just readable_scopes).
- make ann-sparse-scope-recall-check shows recall@k for the sparsest scope decile is >= recall@k of the exact path minus a fixed epsilon (e.g. <=1 Q16 recall bucket), with no silent budget-induced drop; |allowed| below a documented threshold returns exact top-k.
- make context-pack-retrieval-incomplete-check asserts that whenever ANN budget_exceeded or a scope/budget drop occurs with fallback disabled, the ContextPack carries a RetrievalIncomplete/InsufficientContext anomaly (and it round-trips through every export format).
- make fail-closed-invariant-model-check (property/proptest harness) finds no counterexample over randomized catalogs/views/WHERE clauses to 'admitted ⊆ agent_allowed ∩ live ∩ where', and emits a stable model_hash.
- make hnsw-cosine-correctness-check asserts cosine(v,-v)==0, cosine(v,v)==max, ranked order over {v, orthogonal, -v} puts v first and -v last, and high-dim overflow is bounded (parity with context/dedup.rs).
- make context-access-decision-capture-check asserts every admitted cell's access_decision is Allowed with a recorded policy/AgentView version and that no cell is ever NotRecorded on a successful AQL retrieve path.
- Determinism preserved: make engine-determinism-check stays green (no wall-clock/elapsed_nanos enters any scope/leak/incomplete output that the receipt will hash).

**Зависимости:** Receipt pillar (Verifiable accountability receipt): consumes this pillar's access_root inputs (captured access_decision + denied set), the model_hash from fail-closed-invariant-model-check, and the RetrievalIncomplete anomaly. The receipt cannot honestly bind access_root until ann-scope-parity-check and context-access-decision-capture-check are green.; MUST-FIX correctness pillar: the cosine dot.abs() fix (hnsw-cosine-correctness-check) is shared with the correctness track; sequence it here because scope-admitted ranking soundness is a fail-closed concern (a wrong metric corrupts which in-scope cells are surfaced).; Real-crypto pillar: only the EXPORT/signing of the model_hash and access_root into the receipt depends on a real hash/signature; the invariant model, parity, recall, and leak benchmarks are independent and can land first.; Existing fixtures: crates/cortex-engine/tests/context_pack_private_scope.rs and scripts/context_pack_private_scope_check.py (extend, do not replace); crates/cortex-engine/tests/determinism.rs; the ANN recall fixtures in crates/cortex-engine/src/search/ann_recall_tests.rs and ann_corpus.rs.

### A.3 Real cryptography (replace obfuscation) · **категориообразующий**

**Текущее состояние:** CortexDB has ZERO cryptographic primitives in the entire workspace. `grep` over every Cargo.toml returns only `roaring` (no sha2/blake3/ed25519/hmac/chacha20poly1305/aes-gcm/argon2/getrandom/zeroize). Every surface that calls itself "encryption", "auth tag", or "integrity" is hand-rolled FNV-1a64 or an XOR-FNV keystream, all reproducible (therefore forgeable) by anyone who can run the same code.

Three concrete obfuscation surfaces, confirmed in-file:
1. At-rest/backup "encryption" — crates/cortex-engine/src/backup/encrypted/crypto.rs: CIPHER_SUITE="cortexdb.xor-fnv64-stream.v1", KDF="cortexdb.fnv64-passphrase.v1". The keystream is FNV(passphrase,nonce,counter) XORed byte-wise (apply_keystream, lines 10-17) — a repeating 8-byte word per 8 plaintext bytes, trivially broken with known plaintext and key-reused across counter blocks. The "KDF" is a single FNV pass over the passphrase (no salt, no iteration, no memory-hardness). The "auth_tag" (lines 33-45) is FNV(passphrase, nonce, FNV(plaintext), FNV(ciphertext)) — not a MAC; anyone with the passphrase or this code path forges a tag for tampered ciphertext. The on-disk container is defined in crates/cortex-engine/src/backup/encrypted/codec.rs:18-30 (ArchiveHeader{schema_version,cipher_suite,kdf,nonce:u64,...,plaintext_hash,ciphertext_hash,auth_tag}) and verified at codec.rs:207-240 (validate_ciphertext/validate_plaintext recompute the FNV tag). validate_passphrase (mod.rs:95-101) only enforces length >=12.
2. Audit hash-chain — crates/cortex-server/src/audit_chain.rs:8-9,42-48: unkeyed FNV-1a64 event_hash over delimiter-joined fields, chained via prev_hash. No secret key exists anywhere. The IDENTICAL FNV implementation and constants are duplicated in the verifier crates/cortex-cli/src/cli_audit_chain.rs:6-7,30+ (event_hash_for_record). The record (crates/cortex-server/src/audit.rs:97-120,192-220) commits only HTTP-envelope metadata + COUNTS (method, path, status, principal_id, scope_decision, duration) — never the admitted cell_ids, citations, verdict, or any pack hash. The chain is keyless, so any party who can read the log recomputes every hash and rewrites history undetectably. Wired into release via audit-chain-check (mk/core-security-ops.mk:23-28) and security-gate-v2-check.
3. Other FNV "integrity" — ANN report digest (crates/cortex-engine/src/search/ann_report.rs:158) and content_hash is an optional self-asserted metadata string (crates/cortex-core/src/cell/descriptor.rs), not a DB-computed digest.

Key-management/rotation: there is NO key surface in crates/cortex-server/src/config.rs (grep for passphrase/secret/key_id/signing/keystore returns nothing); the backup passphrase is passed per-call and turned straight into FNV. No node keypair, no key_id, no rotation story, no transparency anchor.

Dependency/build reality: there is no [workspace.dependencies] table (root Cargo.toml only lists members + workspace.package); each crate declares deps directly (e.g. cortex-engine: flate2, thiserror, csv, serde, serde_json), so RustCrypto crates must be added per-crate. The repo already vendors many external crates (serde/axum/clap/flate2/tower) so adding audited RustCrypto crates is policy-consistent.

Docs make at-rest claims that are currently dishonest: docs/SECURITY_MODEL.md:77 ("encrypted at-rest backups and secret management integrations") and the archived docs/archive/ENCRYPTED_BACKUPS_DESIGN.md describe envelope encryption/AEAD as the model while the shipped code is XOR-FNV.

Gate culture is strong and is the enforcement substrate: every capability is a `<area>-<facet>-check` target that runs `cargo test -p <crate> <filter>` plus a `scripts/<name>_check.py --report <JSON>` evidence emitter (see mk/core-security-ops.mk, mk/storage-ops.mk:1-21). Existing slots to replace/extend: encrypted-backup-check, encrypted-backup-rotation-check, audit-chain-check, secrets-check, security-gate-v2-check, security-release-report-check.

**Целевое состояние:** Every integrity/confidentiality surface that the accountability receipt depends on is backed by an audited, standard cryptographic primitive, and every public security claim is honest and gate-enforced. Concretely:

1. At-rest/backup: the backup container uses real AEAD (XChaCha20-Poly1305, RustCrypto `chacha20poly1305`) with a random 256-bit data key, a random 24-byte nonce, and a real password-based KDF (Argon2id, `argon2` crate) over the passphrase + a random 16-byte salt with documented, pinned parameters (m,t,p). The AEAD tag authenticates ciphertext + header AAD (cipher_suite, kdf params, nonce, salt, schema_version). Tampering ANY byte of ciphertext, tag, nonce, salt, or AAD makes open() fail with no plaintext leak; a wrong passphrase fails cleanly. Cipher suite string bumps to e.g. "cortexdb.xchacha20poly1305-argon2id.v2"; the legacy XOR-FNV suite is refused on read (never silently trusted) behind a schema-version gate. The FNV `auth_tag`, FNV `hash_hex`, and XOR keystream are deleted.

2. Audit chain: events are committed with a real hash (SHA-256, `sha2`, or BLAKE3) and the chain head is authenticated by a keyed MAC (HMAC-SHA-256, `hmac`+`sha2`) OR per-checkpoint Ed25519 signature (`ed25519-dalek`) using a node key the operator manages — so the log is tamper-evident against the operator, not merely truncation. The same primitive is shared by the cortex-server writer and the cortex-cli verifier (single source of truth, no duplicated FNV). The record additionally commits the per-answer receipt hash (binding to the Real-crypto receipt pillar) so the chain attests WHAT was returned, not just that an endpoint was called.

3. Receipt/signing trust root: a node keypair exists with a stable key_id; the public key is exportable for offline verification; signing uses Ed25519 with RFC-8032 deterministic nonces (so signatures over identical canonical bytes are byte-identical, preserving the determinism invariant). Key custody, generation, and rotation (key_id bump + dual-trust window + re-anchor) are documented and have a CLI/ops flow. An optional append-only transparency anchor for chain checkpoints is specified to mitigate equivocation.

4. Key management/rotation: cortex-server config gains an explicit signing-key / MAC-key source (file or env, never logged), key_id, and rotation procedure; secrets-check is extended to prove keys/passphrases are never echoed or written to logs.

5. Honest claims: docs/SECURITY_MODEL.md, BACKUP_RESTORE.md, and the encrypted-backup design state the real cipher suite, KDF, AEAD guarantee, threat model (what a holder of the passphrase/key can and cannot do), and the rotation story — gate-enforced so claims cannot drift from code.

All of the above is enforced by new/upgraded `*-check` gates wired into the existing security and storage release lanes (security-gate-v2-check, encrypted-backup-check, audit-chain-check), each emitting a report JSON, following the project's established gate pattern. Known-answer test (KAT) vectors are pinned for AEAD/KDF/MAC so the primitives are reproducible and regression-locked.

| ID | Задача | Exit-гейт | Усилие | Зависит | Риск |
|---|---|---|---|---|---|
| `CRY-1` | **Add audited RustCrypto dependencies + dependency-policy gate** — Add the standard audited crates to the crates that need them, declared directly (there is NO [workspace.dependencies] table — root Cargo.toml only has members + workspace.package, and crates list deps inline like cortex-engine's flate2/serde). cortex-engine (backup AEAD + receipt hash/sign): chacha20poly1305, argon2, getrandom, zeroize, subtle, sha2 (or blake3), ed25519-dalek. cortex-server + cortex-cli (audit MAC/verify): hmac, sha2 (and ed25519-dalek if signing checkpoints). Pin versions and run `cargo deny`/license check (license is Apache-2.0 across workspace). Write scripts/crypto_deps_policy_check.py emitting a report JSON: asserts the approved crates are present where required AND greps crates/*/src (excluding tests/benches) to prove the FNV/XOR integrity routines are gone from production backup+audit paths (zero hits on apply_keystream/auth_tag/the two FNV constants in those modules). | `make crypto-deps-policy-check passes (crates present, no production FNV/XOR integrity routines remain in backup/audit src)` | S | — | RustCrypto crates may not yet be on the project's dependency allowlist/vendor mirror; pulls in getrandom (OS entropy) which must be available in CI/build sandbox. Mitigate by confirming the allowlist policy and vendoring before starting downstream tasks. |
| `CRY-2` | **Define a single shared crypto primitives module (hash, AEAD, KDF, MAC, sign/verify, key types)** — Create one cortex-crypto surface (either a small new workspace crate `cortex-crypto` or a `crypto` module in cortex-core re-exported where needed) that wraps: hash = SHA-256 (or BLAKE3) with a domain-tag helper; aead_seal/aead_open = XChaCha20-Poly1305 with explicit nonce + AAD; kdf = Argon2id(passphrase, salt, params) -> 32-byte key with pinned m/t/p constants; mac_sign/mac_verify = HMAC-SHA-256; sign/verify = Ed25519 (deterministic RFC-8032 nonces) with KeyId + public-key export; NodeKeyPair load/generate; zeroize secret material; use subtle for constant-time tag/MAC comparison. This is the ONE place the receipt pillar, audit chain, and backup all call into, so the FNV duplication (server + cli) is replaced by a single implementation. Pin KAT vectors for each primitive in this crate's tests. | `make crypto-primitives-check passes (KAT vectors for SHA-256/XChaCha20-Poly1305/Argon2id/HMAC/Ed25519 match; constant-time compare unit test; sign/verify round-trip and deterministic-signature test)` | M | CRY-1 | API shape must serve both the receipt pillar (hash+sign) and ops (AEAD+KDF+MAC) without churn; a wrong abstraction forces rework. Keep it thin (no bespoke constructions) and depend only on standard primitives. |
| `CRY-3` | **Replace backup obfuscation with XChaCha20-Poly1305 + Argon2id (v2 format) and refuse legacy** — Rewrite crates/cortex-engine/src/backup/encrypted/crypto.rs to use CRY-2: derive a 256-bit key via Argon2id(passphrase, random 16-byte salt, pinned params); encrypt the plaintext archive with XChaCha20-Poly1305 using a random 24-byte nonce; authenticate ciphertext with the header as AAD. Update codec.rs ArchiveHeader (currently {cipher_suite,kdf,nonce:u64,plaintext_hash,ciphertext_hash,auth_tag}) to v2: {schema_version:'cortexdb.encrypted_backup.v2', cipher_suite:'cortexdb.xchacha20poly1305-argon2id.v2', kdf:'cortexdb.argon2id.v1'+params, salt, nonce:[u8;24], aead_tag, file_count, ...}; drop plaintext_hash/ciphertext_hash/FNV auth_tag. validate_*: AEAD open authenticates AAD+ciphertext+tag in one step (no recompute-FNV path). On read of a v1 'cortexdb.xor-fnv64-stream.v1' archive, return a typed StorageInvariant error refusing to decode it. Tighten validate_passphrase guidance. Delete apply_keystream/auth_tag/hash_hex/stream_word. Pin AEAD+KDF KAT vectors. | `make encrypted-backup-check passes on v2 (round-trip; per-byte tamper of ciphertext/tag/nonce/salt/AAD => open error, zero plaintext leak; wrong passphrase fails; KAT match) AND make encrypted-backup-legacy-refuse-check passes (v1 XOR-FNV archive refused on read)` | M | CRY-2 | On-disk format break: existing v1 archives become unreadable. Must ship a documented migration/refuse-to-read gate and CLI guidance (re-create backups under v2); coordinate with backup-drill/offsite gates. Argon2id params trade restore latency vs hardness — pin and document; expose so very large backups don't time out drills. |
| `CRY-4` | **Replace audit FNV hash-chain with real hash + keyed MAC (shared writer/verifier)** — Replace the unkeyed FNV-1a64 in crates/cortex-server/src/audit_chain.rs:42-48 (event_hash) AND the duplicated FNV in crates/cortex-cli/src/cli_audit_chain.rs with CRY-2 primitives: event_hash = SHA-256 over canonical ordered fields; the chain head/checkpoint is authenticated by HMAC-SHA-256 under a node MAC key (or Ed25519-signed checkpoints). Move the shared field-canonicalization + chain logic into one module both crates depend on (eliminate the copy-paste FNV in server and cli). Add a node-key source to cortex-server config (CRY-6). Preserve sequence + prev_hash linkage and rotation (sink.rs). Update AUDIT_CHAIN_ID to a v2 string and add a refuse/verify path for v1 FNV chains. Keep tail()/is_hex_hash adapted to the new digest width. | `make audit-chain-check passes against keyed v2 chain (tamper of any past event fails verify; forgery without MAC key impossible; cortex-cli verifies cortex-server-written chain via the shared module; sequence/prev_hash linkage intact)` | M | CRY-2, CRY-6 | audit_event_hash is referenced by server sink (write) and cli verifier (read) plus the audit-chain/audit-productization/security-gate-v2 gates; a digest-width/format change touches all. On-disk chain format breaks => version-bump + refuse-or-migrate. MAC key must be present in CI for gates to run. |
| `CRY-5` | **Commit per-answer receipt hash into the audit record** — Extend crates/cortex-server/src/audit.rs AuditRecord (currently commits only HTTP envelope + counts: method/path/status/principal/scope_decision/duration) to additionally carry and hash the accountability-receipt hash (pack_root / receipt header hash) produced by the receipt pillar for /v1/context, /v1/verify, /v1/aql responses, and ideally the ordered admitted cell_ids + verdict. Fold this field into event_hash so the chain attests WHAT was returned, not just that an endpoint was called. Add a verifier check that an audit record's committed receipt hash matches the referenced ContextPack's hash. | `make audit-receipt-binding-check passes (audit record commits the receipt hash; verifier rejects a record whose committed receipt hash != recomputed ContextPack receipt hash)` | M | CRY-4, Receipt/canonical-serialization-and-determinism pillar | Hard cross-pillar dependency: the receipt hash format must be frozen by the receipt pillar first, else churn. Receipt computation must not add non-deterministic/wall-clock fields to the hashed surface. Plumbing the hash from engine response into the server audit path crosses crate boundaries. |
| `CRY-6` | **Node key management: keypair/MAC key, key_id, config source, rotation flow** — Add a key surface to cortex-server config (none exists today in crates/cortex-server/src/config.rs): a signing keypair (Ed25519) for receipts/checkpoints and a MAC key for the audit chain, sourced from a file path or env (never logged, zeroized in memory via CRY-2). Assign a stable key_id; export the public key for offline verification. Implement generate + rotate: on rotation, bump key_id, keep a dual-trust window so historical receipts/chain segments verify under the prior key, and re-anchor the chain. Provide a CLI/ops command (align with existing auth-rotation-check pattern). Document custody and rotation in OPERATIONS/SECURITY docs. Specify (design-doc level) an optional append-only transparency anchor for chain checkpoints to mitigate equivocation, tracked as a follow-up. | `make key-management-check passes (key generation + key_id assignment; rotation keeps historical verification valid under dual-trust window; public-key export verifies a signed sample) and secrets-check still passes (keys/passphrases never echoed to logs/audit)` | M | CRY-2 | Key custody is an operational footgun: a lost MAC/sign key breaks audit verification; a leaked key forges receipts. Single-node beta has no HA, so key+log loss on one node is real — document recovery and the transparency-anchor option. Rotation must not retroactively invalidate prior receipts. |
| `CRY-7` | **Make at-rest/crypto claims honest and gate-enforce them** — Update docs/SECURITY_MODEL.md (line 77 'encrypted at-rest backups...'), docs/BACKUP_RESTORE.md, and the encrypted-backup design doc to state the real cipher suite (XChaCha20-Poly1305), KDF (Argon2id + params), the AEAD integrity guarantee, the threat model (a passphrase/key holder CAN decrypt; tampering is detected; the DB operator CANNOT silently rewrite the MAC'd audit chain), the key custody + rotation story, and the explicit non-goals (e.g. no live at-rest encryption of the running store unless implemented; no KMS unless integrated). Remove any wording that calls XOR-FNV 'encryption'. Add scripts/crypto_claims_honesty_check.py that lints these docs for required real-cipher/KDF/rotation mentions and forbids unbacked 'encrypted at rest' claims, following the existing doc-lint gate pattern. | `make crypto-claims-honesty-check passes (docs name the real cipher/KDF/AEAD + rotation; no unbacked at-rest-encryption claim; XOR-FNV-as-encryption wording removed)` | S | CRY-3, CRY-6 | Tendency to overclaim (e.g. 'compliance-grade', 'KMS-backed') beyond what shipped. Keep claims strictly to what CRY-2/3/4/6 deliver; flag KMS/transparency-log as explicit future work, consistent with the project's public-claims discipline. |
| `CRY-8` | **Aggregate crypto-foundation gate wired into the security release lane** — Create a crypto-foundation-check aggregate target (mk/core-security-ops.mk) that depends on crypto-deps-policy-check, crypto-primitives-check, encrypted-backup-check (v2), encrypted-backup-legacy-refuse-check, audit-chain-check (v2), audit-receipt-binding-check, key-management-check, and crypto-claims-honesty-check, each emitting its report JSON like existing gates. Wire crypto-foundation-check into security-gate-v2-check / security-release-report-check so the beta/security release lane fails if any cryptographic foundation regresses. Add an aggregate report script mirroring scripts/security_gate_v2_check.py that consumes the sub-reports. | `make crypto-foundation-check passes and make security-gate-v2-check (which now includes it) passes end-to-end` | S | CRY-1, CRY-3, CRY-4, CRY-5, CRY-6, CRY-7 | Adds runtime (Argon2id KDF, AEAD round-trips, MAC verify) to an already large release lane; keep KAT/tamper fixtures small and CI-safe. Aggregation must not mask a failing sub-gate — assert each sub-report's pass flag explicitly. |

**Измеримые критерии готовности столпа:**
- make crypto-deps-policy-check passes: a script asserts the approved RustCrypto crates are present in the relevant crate Cargo.toml files AND that no production module under crates/*/src (excluding tests/benches) defines or calls an FNV/XOR routine for an integrity-or-confidentiality purpose (grep gate over apply_keystream/auth_tag/0xcbf29ce4/0x100000001b3 in backup + audit paths returns zero hits).
- make encrypted-backup-check passes against the v2 AEAD format: cargo round-trip seal/open test green; a tamper test that flips each of {one ciphertext byte, the AEAD tag, the nonce, the salt, one AAD/header field} makes open() return an error and leaks zero plaintext; a wrong-passphrase test fails cleanly; pinned XChaCha20-Poly1305 + Argon2id KAT vectors match.
- make encrypted-backup-legacy-refuse-check passes: a v1 XOR-FNV archive is REFUSED on read with a typed error (never silently decoded), proving no silent trust of the obfuscation format.
- make audit-chain-check passes against the keyed chain: forgery test (no MAC key => cannot produce a verifying chain), tamper test (edit any past event => verify fails), key-rotation test (rotated key_id still verifies historical segments under the prior key), and cross-tool test (cortex-cli verifies a cortex-server-written chain using the shared MAC implementation, not a duplicated FNV).
- make audit-receipt-binding-check passes: an audit record commits the per-answer receipt hash and the verifier rejects a record whose committed receipt hash does not match the referenced ContextPack.
- make crypto-claims-honesty-check passes: a doc-lint asserts docs/SECURITY_MODEL.md and BACKUP_RESTORE.md name the real cipher suite/KDF/AEAD and the rotation story, and contain NO claim of 'encrypted at rest' that is not backed by the AEAD path; the dishonest XOR-FNV-as-encryption wording is gone.
- make key-management-check / auth-rotation-check passes: node signing/MAC key has a configured source + key_id, rotation procedure is tested, and secrets-check proves keys and passphrases are never echoed to logs or audit events.
- All new gates are wired into security-gate-v2-check (or a new crypto-foundation-check aggregate) so they run in the beta/security release lane; security-release-report-check still passes.

**Зависимости:** Receipt/canonical-serialization-and-determinism pillar — consumes the Ed25519 signer and the hash primitive this pillar introduces; this pillar must land the signer + hash before receipts can be signed. The audit-chain-binds-receipt-hash task here depends on that pillar defining the receipt hash.; MUST-FIX correctness pillar (cosine dot.abs(), guarded-ANN recall, conflict normalization) — independent of this pillar and can proceed in parallel; a signed-but-wrong receipt is worse than none, so those fixes must land before the receipt is published, but they do not block the crypto work itself.; External: audited RustCrypto crates (chacha20poly1305, aes-gcm, argon2, sha2, hmac, ed25519-dalek, getrandom, zeroize, subtle) must be approved for the dependency allowlist / vendoring policy.

### A.4 Deterministic verification at strength: robust, measured, LLM-free contradiction detection · **категориообразующий**

**Текущее состояние:** Deterministic LLM-free VERIFY FACT exists and is reasonably structured, but its conflict coverage is heuristic and unmeasured, undermining the accountability thesis.

CONFLICT EXTRACTION IS NARROW AND DUPLICATED ACROSS TWO INCOMPATIBLE PATHS:
- VERIFY-path (engine truth): `crates/cortex-engine/src/verification/numeric/fact_claim.rs:52-80,186-216` builds `FactClaimStore`/`NumericFactIndex` keyed on `MetricIndexKey{scope, metric, project}` (fact_claim.rs:225-230). Conflict surfacing (`add_verify_matches`, fact_claim.rs:104-170) uses `normalized_numeric_equal`/`numeric_conflict` from `verification/numeric/value.rs`.
- ContextPack-path (what the pack/receipt sees): `crates/cortex-engine/src/context/conflicts.rs:12-41` groups by lowercased `(project, metric)` and flags `>1` distinct RAW STRING value, using `extract_project_metric_value` (`crates/cortex-engine/src/context/dedup.rs:53-70`) which only matches literal `project=`/`metric=`/`value=` line prefixes. So `$1.2M` vs `1,200,000 USD` vs `1.2 million` register as a 3-way FALSE conflict, and format-equal values can be missed. These two paths can disagree.

NO UNIT/CURRENCY CONVERSION (only equality):
- `crates/cortex-engine/src/verification/numeric/value.rs:64-86` `compare_numeric_values`: different currencies => `Conflict` (no FX is correct, but it cannot recognize cross-currency as Incomparable vs Conflict policy), different units => `Incomparable` with NO conversion, same unit/currency compares `scaled_value` exact-equal only. `parse_unit_code` (`parse.rs:39-50`) only aliases hr->h, sec->s; there is no m<->km, h<->min, kg<->g magnitude conversion, so "60 min" vs "1 h" is silently Incomparable (a missed conflict or missed agreement).

MULTI-VALUE CELLS ARE DROPPED:
- `fact_claim.rs:390-394` `single_numeric_value` returns None when a cell yields >1 numeric value, so any cell with multiple numbers is excluded from the numeric fact index entirely (zero conflict recall on multi-value cells).

TEMPORAL CONFLICTS UNHANDLED, AND TEMPORAL FACTS SKIP NUMERIC CONFLICTS:
- `fact_claim.rs:112` and `:172-175`: `add_verify_matches`/`indexed_cell_ids_for_fact` return EARLY when `extract_temporal_query_range(fact).is_some()`, so a dated fact gets NO numeric conflict detection.
- `crates/cortex-engine/src/verification/temporal.rs` only does validity-window staleness as a guard (`TemporalStaleReason::Expired/NotYetValid`); there is no "same metric, two values at the SAME effective date" temporal contradiction, and no citation-vs-citation conflict surfaced into `numeric_conflicts` or the conflict index.

DETERMINISM HAZARD IN THE REPORT:
- `crates/cortex-engine/src/verification/operator.rs:28,183,188,203,207` embeds wall-clock `total_elapsed_nanos` and per-operator `elapsed_nanos` in `VerificationExecutionReport`. These are non-deterministic and must be excluded from any hashed/receipt surface.

NO CONTRADICTION-RECALL BENCHMARK:
- The only datasets are `examples/eval/verification_cases.jsonl` (small scenario fixture, `VERIFICATION_QUALITY_FIXTURE`) and `crates/cortex-engine/fixtures/context_verify_quality_v1.cells` (the single 1.2B/1.4B KZT case, documented in docs/VERIFY_FACT.md:317-333 as "deterministic alpha fixture, not a measured accuracy benchmark"). `make verification-quality-check` / `context-verify-quality-check` assert presence on a handful of cases; NO labeled corpus measures conflict recall or false-conflict rate. docs/VERIFY_FACT.md:297-308 itself lists unit/magnitude/currency/temporal as "Limitations (Alpha)".

STRONG PRIMITIVES ALREADY PRESENT (mostly wiring needed): the numeric module already exposes `parse_currency_code`, `parse_unit_code`, `parse_magnitude_suffix`, `NumericValue{raw,scaled_value,currency,unit,magnitude}`, `compare_numeric_values`, `normalized_numeric_equal`, `numeric_conflict` (`crates/cortex-engine/src/verification/numeric/{parse.rs,value.rs,mod.rs}`); a durable `ConflictRecord`/`ConflictIndexStore` exists (`crates/cortex-engine/src/verification/conflict_index.rs`); the project has a strong `*-check` gate culture (mk/core-retrieval-context.mk, mk/performance-dashboard.mk) with python fixture-driven checks and Rust integration tests.

**Целевое состояние:** VERIFY FACT detects contradictions across a much broader, MEASURED surface while remaining integer-only/Q16-deterministic and LLM-free, and the ContextPack conflict-visibility path is unified with the VERIFY numeric path so the receipt attests one coherent conflict set:

1. ONE canonical conflict extractor. `context/conflicts.rs` and `context/dedup.rs::extract_project_metric_value` no longer string-compare raw values; both the ContextPack visibility path and the VERIFY path normalize values through the same `verification/numeric` primitives before comparison, with a string fallback only for non-numeric values. `$1.2M`, `1,200,000`, `1.2 million` collapse to one value (no false conflict); genuine numeric differences flag.

2. Unit/currency normalization with explicit, integer-only conversion. `value.rs` converts within a compatibility class (length m<->km<->cm, mass kg<->g, time h<->min<->s, plus the existing magnitude B/M/K) to a canonical base unit before comparison; cross-class stays `Incomparable`; cross-currency stays `Conflict` (no FX invented) but is explicitly labeled so the receipt can distinguish "currency-mismatch conflict" from "value conflict". All conversions are integer math (no f32/f64), preserving bit-reproducibility.

3. Multi-value extraction. Cells with multiple numbers are indexed per (metric, value) instead of being dropped; `single_numeric_value` is replaced by a metric-scoped selection so multi-value cells contribute conflict recall.

4. Temporal & citation conflicts surfaced as first-class. Dated facts no longer short-circuit numeric conflict detection; "same metric/project, two distinct values valid at the same effective date" is emitted as a temporal contradiction, and "two cells cite the SAME source_ref but disagree on the value" is emitted as a citation conflict. Both appear in `numeric_conflicts`/`VerificationMatchKind` and the conflict index.

5. Determinism preserved and proven. `elapsed_nanos`/`total_elapsed_nanos` are excluded from any hashed/serialized conflict surface; a determinism test asserts the conflict set + ordering + Q16 are byte-identical across two runs.

6. MEASURED recall. A labeled contradiction benchmark (>=150 cases spanning magnitude/unit/currency/temporal/citation/format variants, with both true-conflict and must-NOT-conflict "agreement" cases) reports conflict recall, false-conflict rate, and precision, gated in CI. The headline claim becomes "contradiction recall = X% / false-conflict = Y%", replacing the alpha disclaimer in docs/VERIFY_FACT.md.

| ID | Задача | Exit-гейт | Усилие | Зависит | Риск |
|---|---|---|---|---|---|
| `DV1` | **Unify ContextPack conflict visibility onto the VERIFY numeric normalizer** — Replace the raw-string grouping in crates/cortex-engine/src/context/conflicts.rs:12-41 and broaden crates/cortex-engine/src/context/dedup.rs::extract_project_metric_value (dedup.rs:53-70) so value comparison routes through verification/numeric (extract_numeric_values + compare_numeric_values / normalized_numeric_equal). Group by (project, metric); for each group, fold numerically-equal values into one canonical value and only count a VisibleConflict when normalized values genuinely differ. Keep a string-equality fallback ONLY for non-numeric values. Preserve all-integer/Q16 determinism and the existing conflict_visibility_q16 intensity curve (conflicts.rs:55-62). Outcome: ContextPack conflict set and VERIFY numeric_conflict set agree on a shared corpus. | `make context-pack-conflict-visibility-check (extended with a $1.2M/1,200,000/'1.2 million' = 0 conflicts fixture and a cross-path-agreement assertion)` | M | DV2 | extract_project_metric_value is also used by is_redundant (dedup.rs:72-103); changing its semantics could alter redundancy/dedup behavior. Mitigate by keeping the raw extractor signature and adding a separate normalized-compare step in conflicts.rs only, not changing dedup's redundancy short-circuit. |
| `DV2` | **Add integer-only unit/currency normalization+conversion in value.rs** — Extend crates/cortex-engine/src/verification/numeric/value.rs:64-94 and parse.rs:39-50 with a compatibility-class model: define canonical base units (length->mm or cm, mass->g, time->s) and convert scaled_value within a class using integer multipliers before compare_scaled. Cross-class remains Incomparable; cross-currency remains Conflict but tag it distinctly (e.g. extend NumericComparison or carry a reason) so a currency-mismatch conflict is distinguishable from a value conflict for the receipt. NO f32/f64 — all conversion via integer multipliers (mirror Magnitude::multiplier at value.rs:38-47). Add reverse aliases in parse_unit_code so '60 min' and '1 h' normalize to the same base. | `make verify-numeric-normalization-check (new gate: unit-class round-trips, 60min==1h, 1h vs 2h conflict, cross-class Incomparable, cross-currency labeled Conflict, grep asserts no f32/f64 added)` | M | — | Choosing too-small a base unit can overflow u64 on large magnitudes (e.g. km->mm * billions). Mitigate by using u128 intermediates for conversion and saturating, and pick base units conservatively; add an overflow unit test. |
| `DV3` | **Support multi-value extraction in the numeric fact index** — Replace single_numeric_value (crates/cortex-engine/src/verification/numeric/fact_claim.rs:390-394) which drops any cell with >1 value. Index each extracted NumericValue under its (scope, metric, project) key, or scope selection to the value nearest the metric token when a cell has multiple metrics. Ensure NumericFactIndex insert/remove (fact_claim.rs:246-294) and tombstone paths stay consistent for multi-value records. Preserve determinism (BTreeMap/BTreeSet ordering). | `make verify-multivalue-extraction-check (new gate: a fixture cell with >=2 numbers yields a detected conflict the current single-value path misses; insert/patch/tombstone round-trip stays consistent)` | M | DV2 | Naive multi-value indexing can explode false conflicts (every number in a paragraph compared against the metric). Mitigate by binding each value to its nearest metric/unit context during extraction and only comparing within the same (metric) group. |
| `DV4` | **Surface temporal contradictions and stop temporal facts from skipping numeric conflicts** — Remove the early returns at fact_claim.rs:112 and :172-175 that bail when extract_temporal_query_range(fact).is_some(), so dated facts still get numeric conflict detection. Add a temporal-contradiction rule: for one (project, metric), two distinct normalized values whose validity windows (crates/cortex-engine/src/verification/temporal.rs TemporalValidity) overlap at the same effective date are a contradiction emitted into numeric_conflicts with a new/existing VerificationMatchKind. Keep existing stale-window behavior as guards (no regression). | `make verify-temporal-conflict-check (new gate: dated fact still produces numeric conflicts; same-date contradictory values flagged; stale-window still a guard) AND make verification-quality-check unchanged-green` | L | DV2, DV3 | Overlap logic can mis-flag legitimately superseded values (old value valid_to < new valid_from). Mitigate by only flagging when windows genuinely overlap; add agreement (non-overlap supersession) test cases. |
| `DV5` | **Add citation-conflict detection (same source, disagreeing value)** — Using the source_ref/source/citation already on NumericFactRecord (fact_claim.rs:31-32) and ConflictRecord (conflict_index.rs:18-29), detect cases where two cells cite the SAME normalized source_ref for the same (project, metric) but carry numerically-distinct values, and emit a citation conflict (distinct from a cross-source numeric conflict). Fold it into the conflict index and numeric_conflicts so the receipt can attest it. | `make verify-citation-conflict-check (new gate: two cells with identical source_ref + same metric + different normalized value flag a citation conflict; same source + equal value does not)` | M | DV2 | source_ref normalization is fuzzy (url vs page vs row). Mitigate by comparing on the canonical SourceRef tuple already parsed in metadata, and treat missing/partial source_ref as 'not a citation conflict' (fall back to ordinary numeric conflict). |
| `DV6` | **Exclude wall-clock fields from the hashable verification surface and prove determinism** — Move total_elapsed_nanos/elapsed_nanos (operator.rs:28,183,188,203,207) out of any serialized/hashed conflict/verdict surface (keep them only in an unhashed perf-trace side channel). Define a canonical, ordering-stable serialization of {status, confidence_q16, ordered numeric_conflicts (cell_id, metric, left, right, kind), conflict_visibility} with integers/Q16 only and no timestamps. Add a test running VERIFY twice on a fixed store and asserting byte-identical canonical bytes. | `make verify-determinism-check (new gate, also wired into engine-determinism-check): two VERIFY runs on a fixed store produce byte-identical canonical conflict serialization` | M | DV2, DV3, DV4, DV5 | Hidden non-determinism from HashMap iteration or candidate truncation tie-breaks. Mitigate by auditing for BTree* usage and adding a stable cell_id tie-break before truncate (operator.rs:72-74 already sorts by cell_id — verify all conflict paths do too). |
| `DV7` | **Build the labeled contradiction-recall benchmark and CI gate** — Create a labeled corpus (>=150 cases) extending examples/eval/verification_cases.jsonl format with spans: magnitude variants (1.2B vs 1,200,000,000), unit-class (60min vs 1h; 1km vs 1000m), currency (1.2B KZT vs 1.2B USD), temporal (same metric, two dates), citation (same source disagreeing), format variants ($1.2M vs 1.2 million), AND 'agreement' must-NOT-conflict cases to measure false-conflict rate. Add a python check (mirroring scripts/verification_quality_check.py) computing conflict recall, precision, and false-conflict rate against labels, plus a Rust integration test driving VERIFY over the corpus. Ship a small CI-safe 'mini' subset and a larger downloadable 'full' set, following the repo's 'tune the 50 first, then promote' convention. | `make verify-conflict-recall-check (new gate): conflict recall >= 0.90, false-conflict rate <= 0.05 on the mini corpus, report JSON emitted, regression-gated` | L | DV1, DV2, DV3, DV4, DV5, DV6 | Labels can encode the implementation's own heuristics (teaching to the test). Mitigate by authoring cases from the documented limitation classes (docs/VERIFY_FACT.md:297-308) and including adversarial format/unit variants the current code provably misses; have the benchmark author distinct from the extractor changes where possible. |
| `DV8` | **Update VERIFY FACT docs with measured coverage and supported conflict classes** — Replace the 'Limitations (Alpha)' section (docs/VERIFY_FACT.md:297-333) with the measured recall/false-conflict numbers from DV7 and an explicit enumeration of supported conflict classes (numeric/magnitude, unit-class with conversion, currency-mismatch, temporal same-date, citation same-source). Keep honest scope notes (no FX conversion, cross-class Incomparable). Update the quality-gate section to point at verify-conflict-recall-check. | `make docs-claims-check / public-claims-policy gate green with the updated numbers (or the repo's existing docs-consistency gate); DV7 report numbers match the doc` | S | DV7 | Docs drift from measured numbers over time. Mitigate by having verify-conflict-recall-check emit the numbers into a report the docs gate cross-checks, consistent with the repo's PUBLIC_CLAIMS_POLICY discipline. |

**Измеримые критерии готовности столпа:**
- make verify-conflict-recall-check passes: on the labeled AAB-style contradiction corpus (>=150 cases), conflict recall >= 0.90 and false-conflict rate <= 0.05, both printed to a report JSON and asserted by the gate (regression-gated thereafter).
- make verify-numeric-normalization-check passes: unit-class conversions (m/km/cm, kg/g, h/min/s) and magnitude (B/M/K) are integer-only and round-trip; '60 min' vs '1 h' agree (no conflict), '1 h' vs '2 h' conflict, cross-class stays Incomparable, cross-currency stays Conflict and is labeled.
- make context-pack-conflict-visibility-check (extended) passes with the SAME numeric normalizer as VERIFY: a fixture proving $1.2M / 1,200,000 / '1.2 million' produce ZERO visible conflicts while 1.2M vs 1.4M produce exactly one, and that the ContextPack conflict set equals the VERIFY numeric_conflict set on a shared corpus.
- make verify-temporal-conflict-check passes: a dated fact still produces numeric conflicts (no temporal short-circuit), same-date contradictory values for one (project,metric) are surfaced as a temporal contradiction, and stale-window cases remain guards (no regression in verification-quality-check).
- make verify-multivalue-extraction-check passes: cells with >=2 numeric values are indexed and contribute at least one detected conflict on a fixture that the current single-value path misses.
- make verify-determinism-check passes: VERIFY FACT run twice on a fixed store yields byte-identical canonical conflict serialization (elapsed_nanos excluded); wired into engine-determinism-check.
- No f32/f64 introduced in verification/numeric or context/conflicts (grep gate in verify-numeric-normalization-check); all existing gates still pass: make verification-quality-check, make context-verify-quality-check.
- docs/VERIFY_FACT.md 'Limitations (Alpha)' updated to state measured recall/false-conflict numbers and the supported unit/currency/temporal/citation conflict classes.

**Зависимости:** Accountability receipt pillar (consumer): the receipt's verification_root / conflict_commitment binds this pillar's numeric_conflicts + conflict_visibility set; the unified, deterministic conflict set produced here is the input it hashes. This pillar must land its determinism fix (elapsed_nanos exclusion) before the receipt can claim byte-identical verification.; MUST-FIX correctness prerequisites pillar (upstream): the HNSW cosine dot.abs() bug (crates/cortex-engine/src/search/hnsw/metric.rs:44) corrupts the semantic-evidence retrieval that feeds VERIFY candidate scanning; while numeric conflict detection is text/index-based and not directly dependent, the verification candidate set quality and any semantic-entailment evidence depend on a correct metric. Recommend sequencing the cosine fix before publishing recall numbers so the benchmark is not measured over corrupted retrieval.; No new external crate dependency is required for this pillar (numeric/temporal primitives already in-tree); it does NOT depend on the crypto/Ed25519 work, so it can proceed in parallel with the receipt's crypto track.

### A.5 Reproducibility guarantee + learned-but-deterministic ranker · **категориообразующий**

**Текущее состояние:** Two halves exist but are disconnected and unproven.

RANKER (today = hand-tuned magic constants, no learned path into the engine):
- The ContextPack final score is a hardcoded additive blend: `base_bm25 + source_trust_bonus + source_freshness_bonus - redundancy_penalty + feedback_bonus` with magic coefficients (redundancy_penalty = max_jaccard_q16 * 10_000 / 65536; MMR uses `3 * relevance - redundancy`). See crates/cortex-engine/src/context/pack/builder.rs:215-232 and crates/cortex-engine/src/context/pack/ordering.rs:80.
- Search rerank weights are hand-authored integer constants: WeightedScoreReranker defaults (lexical_weight=2, vector_weight=2, anchor_payload_bonus=25_000, ...) at crates/cortex-engine/src/search/rerank/types.rs:71-97, and per-question-type overrides at crates/cortex-engine/src/search/rerank/calibration.rs:17-87 (e.g. Basic sets lexical=4/vector=1; ProjectRelated sets lexical_q16=24_000). HybridRrfWeights presets at types.rs:29-50.
- A `database.learned_ranking.enabled` toggle (crates/cortex-engine/src/search/database/ranking.rs:43-55) only switches between `fixed_default()` and `enterprise_rag_calibrated()` — both are still hand-written constants; "learned" is a misnomer today.
- Offline LTR is PROTOTYPED IN PYTHON ONLY and not connected to the engine: scripts/learned_ranking_calibration_check.py grid-searches {lexical_weight,vector_weight} per question_type over fixtures/enterprise_rag_bench/learned_ranking/offline_v1.jsonl (8 rows), enforces train/heldout split non-overlap (split_leakage), and gates on heldout MRR lift >= 2500 bps and win-rate >= 75% (make learned-ranking-calibration-check, mk/core.mk:38). It emits a report (schema cortexdb.learned_ranking_calibration.v1) but NEVER compiles the selected profile into the Rust constants — there is no frozen weight artifact, no codegen, no drift check binding script output to engine constants.

REPRODUCIBILITY (today = snapshot-tested, not hashed):
- crates/cortex-engine/tests/determinism.rs asserts repeatability by comparing human-readable STRING snapshots of pack/search/verify across a checkpoint; it does NOT compute or compare a canonical byte serialization or any hash.
- make engine-determinism-check (mk/core-contracts.mk:95) is a static lint: scripts/engine_determinism_check.py only bans HashMap/HashSet tokens and requires doc/test marker strings; the doc it references (docs/ENGINE_DETERMINISM.md) is archived (docs/archive/ENGINE_DETERMINISM.md), so the gate's doc-token check is stale.
- ContextPack JSON (crates/cortex-engine/src/context/export/json_export.rs) is a serde_json::json! convenience string with fixed source-order keys but NO recursive key sorting, NO number/string normalization, NO canonicalizer — it omits clocks (good) but is not a committed receipt and has no fingerprint.
- VerificationExecutionReport embeds wall-clock elapsed_nanos (per the audit), which is provably non-deterministic and must be excluded from any hashed surface.
- Confirmed: ZERO crypto crates in any Cargo.toml (no blake3/sha2/ed25519); the determinism hash and receipt signing are a hard dependency on the receipt pillar landing real crypto.
- Strong gate culture confirmed: `<area>-<facet>-check` targets each writing a JSON report under target/, registered in mk/phony.mk and parameterized in mk/vars-core.mk (e.g. LEARNED_RANKING_* vars at mk/vars-core.mk:135-138).

**Целевое состояние:** A single, audited weight artifact and a tested bit-reproducibility guarantee:

1. FROZEN WEIGHTS ARTIFACT: A versioned, checked-in JSON file (e.g. fixtures/ranking/frozen_weights.v1.json, schema cortexdb.ranking_weights.v1) holds ALL ranking coefficients as integer Q16 values: the additive pack-blend weights (replacing the magic 10_000/65536 and 3x MMR constants), the per-question-type rerank weights (replacing calibration.rs constants), and HybridRrfWeights presets. The Rust engine LOADS these constants from a single generated module compiled from the artifact; no ranking coefficient is hand-edited in .rs files anymore.

2. OFFLINE LTR PIPELINE COMPILES TO THE ARTIFACT: scripts/learned_ranking_calibration_check.py (extended) is the trainer; its selected per-type profiles are SERIALIZED into the frozen_weights artifact via an explicit "compile" step, and a drift gate proves the Rust-loaded constants byte-match the artifact the trainer produced (no silent divergence). Training stays offline, deterministic, LLM-free, with train/heldout split enforcement and a heldout-MRR-lift floor; the engine only ever ships frozen integers.

3. CANONICAL SERIALIZATION + DETERMINISM HASH: A canonical_bytes() routine (stable recursive key order, integers/Q16 only, NO timestamps/elapsed_nanos) over ContextPack and over VerificationReport, plus a determinism_hash over canonical (query, AgentView projection, ContextPackOptions, frozen_weights_version). elapsed_nanos is moved out of the hashed surface.

4. BYTE-IDENTICAL GUARANTEE IS TESTED AND GATED: A determinism harness runs RETRIEVE CONTEXT and VERIFY FACT twice on a fixed store and asserts byte-identical canonical pack + determinism_hash; CI gates on it. A weights-version-change test proves the determinism_hash changes iff the frozen weights change (so the receipt binds the exact ranker version).

5. INVARIANTS PRESERVED: All weights remain integer Q16 (no f64 in the scoring hot path), every score component stays in the explain trail (explainability), no LLM enters core, and the learned ranker measurably beats the balanced baseline on a held-out fixture before any profile is accepted.

| ID | Задача | Exit-гейт | Усилие | Зависит | Риск |
|---|---|---|---|---|---|
| `RANK-1` | **Extract all ranking coefficients into a single frozen Q16 weights artifact + generated Rust module** — Define schema cortexdb.ranking_weights.v1 and a checked-in artifact fixtures/ranking/frozen_weights.v1.json holding every ranking coefficient as an integer Q16: (a) the pack additive-blend weights currently hardcoded in crates/cortex-engine/src/context/pack/builder.rs:215-222 (redundancy_penalty scale 10_000/65536, trust/freshness/feedback weights) and the MMR `3*relevance` factor in crates/cortex-engine/src/context/pack/ordering.rs:80; (b) WeightedScoreReranker defaults (crates/cortex-engine/src/search/rerank/types.rs:71-97); (c) per-question-type overrides (crates/cortex-engine/src/search/rerank/calibration.rs:17-87); (d) HybridRrfWeights presets (types.rs:29-50). Add a generated module (e.g. crates/cortex-engine/src/ranking_weights/generated.rs) compiled from the artifact by a build script or codegen script, and replace the inline constants with reads from it. Keep all values integer; no f64. | `make ranking-frozen-weights-check (new): lint asserts the four named modules contain no bare ranking magic-constants outside the generated module, and the engine compiles loading every coefficient from fixtures/ranking/frozen_weights.v1.json; report written to target/ranking/frozen-weights/report.json` | L | — | Behavior-changing if extraction subtly alters arithmetic (e.g. rounding when re-expressing 10_000/65536 as Q16). Mitigate by snapshotting current pack/search outputs before refactor and asserting byte-identical outputs after extraction with the SAME numeric values (pure refactor, weights unchanged in this task). |
| `RANK-2` | **Make the offline LTR trainer COMPILE its selected profiles into the frozen artifact + drift gate** — Extend scripts/learned_ranking_calibration_check.py so that, after grid-search selects per-question-type profiles on the train split and clears the heldout floors, it emits a frozen-weights artifact (the same cortexdb.ranking_weights.v1 schema as RANK-1) as a build output, and add a 'compile/freeze' step that writes it to fixtures/ranking/frozen_weights.v1.json. Add a drift check (scripts/ranking_weights_drift_check.py) that regenerates the artifact from the trainer and diffs it against the checked-in artifact AND against the engine-loaded constants, failing on any mismatch. Record the artifact content hash in both reports. Keep training deterministic, integer-only, LLM-free; preserve existing split-leakage and floor enforcement. | `make ranking-weights-drift-check (new) passes: trainer-emitted artifact == checked-in artifact == engine-loaded constants (byte-diff clean), content hash recorded; AND make learned-ranking-calibration-check still passes its existing floors` | M | RANK-1 | Trainer (Python float MRR) and engine (integer Q16) could disagree on tie-breaks. Mitigate by having the trainer emit ONLY integer Q16 weights and by asserting the engine reproduces the trainer's heldout ranking on the same fixture (cross-language determinism test). |
| `REPRO-1` | **Implement canonical_bytes() for ContextPack and VerificationReport (no clocks, recursive key order, Q16 ints)** — Add a canonical serialization routine (stable recursive key ordering, integers/Q16 only, explicit byte form documented as the normative wire format) over ContextPack and VerificationReport. It must exclude all wall-clock fields: move VerificationExecutionReport total_elapsed_nanos/elapsed_nanos out of the hashed surface (keep them in a separate, non-hashed telemetry struct). This routine is shared with the receipt pillar (it feeds pack_root/verification_root there). Do NOT reuse the existing json_export.rs json! string (it does not recursively sort keys). | `make canonical-serialization-check (new): property test asserts canonical_bytes is invariant under BTreeMap/insertion-order permutation of inputs and that no elapsed_nanos/timestamp byte appears in the output; report to target/canonical-serialization/report.json` | M | — | Easy to accidentally include a non-deterministic field (e.g. an iteration-order-dependent Vec). Mitigate with an explicit allowlist of hashed fields and a test that fails if a new field is added to ContextPack/VerificationReport without being classified hashed/non-hashed. |
| `REPRO-2` | **Compute determinism_hash binding (query, AgentView projection, options, frozen_weights_version)** — Add determinism_hash = H(domain_tag \|\| canonical(query) \|\| canonical(minimized AgentView: sorted readable_scopes/readable_brains/allowed_modes/budgets/allow_verify_fact/private_scope) \|\| canonical(ContextPackOptions) \|\| frozen_weights_version_and_hash). Use the hash primitive from the receipt pillar if it has landed; otherwise use a clearly-namespaced non-cryptographic content hash (e.g. label it cortexdb.determinism.contenthash.noncrypto.v0) behind a TODO gate that MUST flip to the real cryptographic hash before the receipt is signed. Surface frozen_weights_version into the AgentView/options projection so the hash binds the exact ranker. | `make weights-version-binding-check (new): mutating one Q16 weight in fixtures/ranking/frozen_weights.v1.json changes determinism_hash; reverting restores it; report to target/determinism-hash/binding-report.json` | M | REPRO-1, RANK-1 | If the receipt pillar's hash choice changes later, two hashing schemes could ship. Mitigate by isolating the hash call behind one function and naming the interim non-crypto hash explicitly so it cannot be mistaken for the signed primitive. |
| `REPRO-3` | **Byte-identical determinism harness + CI gate over pack and verify** — Add a test in crates/cortex-engine/tests/ (alongside determinism.rs) that seeds a fixed store, runs RETRIEVE CONTEXT twice and VERIFY FACT twice, and asserts byte-identical canonical_bytes() and identical determinism_hash across runs AND across a checkpoint/restart (reuse the existing determinism.rs seeding pattern). Replace the stale doc-token requirement in scripts/engine_determinism_check.py (it points at the archived docs/archive/ENGINE_DETERMINISM.md) with a real determinism doc and wire this byte-identical test into a make gate. The existing string-snapshot tests stay as readable secondary checks. | `make pack-determinism-hash-check (new) passes: two pack runs + two verify runs are byte-identical and survive checkpoint; gate registered in mk/phony.mk and a release lane; report to target/pack-determinism/report.json` | M | REPRO-1, REPRO-2 | Hidden non-determinism (float in a dependency, system locale, HashMap somewhere off the audited path) could make the gate flaky. Mitigate by running the harness 3x in CI and by keeping the engine_determinism_check HashMap/HashSet lint as a guard. |
| `RANK-3` | **Wire frozen learned weights into the engine ranking path and prove the lift on held-out data** — Replace the misleading `database.learned_ranking.enabled` two-way toggle (crates/cortex-engine/src/search/database/ranking.rs:43-55) so that 'learned' actually means 'use the frozen artifact weights' rather than another hand-tuned constant set. Ensure both the search rerank path and the ContextPack additive blend consume the artifact. Add an engine-side regression that reproduces the trainer's heldout ranking improvement (the same MRR-lift the Python gate measures) so the lift is proven in Rust, not only in Python. | `make ranking-learned-lift-check (new): engine-side test on fixtures/enterprise_rag_bench/learned_ranking/offline_v1.jsonl reproduces heldout MRR lift >= 2500 bps and win-rate >= 75% vs the balanced baseline, matching the Python gate; report to target/ranking/learned-lift/report.json` | L | RANK-1, RANK-2 | Overfitting to an 8-row fixture; weights may not generalize. Mitigate by keeping the heldout floor, expanding the fixture with real-embedding candidates AFTER the cosine/ANN MUST-FIX bugs land, and treating the frozen weights as versioned (v1) so they can be reproven and bumped. |
| `RANK-4` | **Preserve explainability under frozen weights (score == sum of explained components)** — Ensure every frozen weight that contributes to a score is reflected as a named component in the explain trail (crates/cortex-engine/src/context/pack/scoring.rs score_components + ContextExplain in context/mod.rs). Add a test asserting the final integer score equals the sum of explain component contributions computed from the frozen weights, so the explanation is not just narrative but a faithful decomposition. | `make ranking-explain-faithfulness-check (new): for each cell in a fixture pack, asserted score == sum(explain.score_components.contribution) under frozen weights; report to target/ranking/explain-faithfulness/report.json` | S | RANK-1 | Some components (e.g. clamped/saturating arithmetic) may not sum exactly. Mitigate by making the explain decomposition track the same saturating ops the score uses, or by documenting and testing the exact clamp points. |
| `RANK-5` | **Promote ANN-budget-exceeded and grounding into hashed pack outputs so the ranker's completeness is attestable** — The learned ranker silently inherits incompleteness from guarded-ANN budget exhaustion (search_impl.rs) and from grounding being a caller-side artifact. Surface the already-computed budget_exceeded signal as a ContextPackAnomaly and include the AnswerGroundingReport in ContextPack so both are inside canonical_bytes() and thus bound by determinism_hash/receipt. This ensures the reproducible pack also reproducibly discloses when retrieval may be incomplete. | `make pack-completeness-signal-check (new): a fixture that trips the ANN visit budget yields a pack whose canonical_bytes() contains a budget-exceeded anomaly, and grounding report fields are present and hashed; report to target/pack-completeness/report.json` | M | REPRO-1 | Touches ContextPack schema (additive optional fields per context_pack.v1 'additive until v2' rule) and the search/pack boundary. Mitigate by adding only optional fields and bumping no schema major; coordinate with the receipt pillar's conflict_commitment so completeness signals are also covered there. |

**Измеримые критерии готовности столпа:**
- make ranking-frozen-weights-check passes: every ranking coefficient in crates/cortex-engine/src/context/pack/ and crates/cortex-engine/src/search/rerank/ is loaded from the frozen weights artifact (a lint asserts no bare integer ranking magic-constants remain in those modules outside the generated weights module).
- make ranking-weights-drift-check passes: the engine-loaded weights byte-match the artifact emitted by the offline trainer (regenerate-and-diff is clean), and the artifact carries a content hash recorded in the report.
- make learned-ranking-calibration-check still passes with the existing floors (heldout MRR lift >= 2500 bps, heldout win-rate >= 75%, zero heldout regressions, train/heldout split non-overlap) AND now additionally emits the frozen Q16 profile that is compiled into the artifact.
- make pack-determinism-hash-check passes: two RETRIEVE CONTEXT runs and two VERIFY FACT runs on a fixed store produce byte-identical canonical serializations and identical determinism_hash; the test lives in crates/cortex-engine/tests/ and is wired into a make gate.
- A canonical_bytes() routine exists for ContextPack and VerificationReport with a property test proving stability under map-insertion-order permutation and proving elapsed_nanos/wall-clock fields are excluded from the hashed surface.
- make weights-version-binding-check passes: changing one Q16 weight in the artifact changes the determinism_hash (and the receipt header's frozen_weights_version), and reverting restores the prior hash — proving the hash binds the exact ranker version.
- All four scoring modules retain full per-component explain output (no component dropped); a test asserts score == sum of explain component contributions under the frozen weights so explainability is preserved.
- No f64 appears in the ranking score hot path (lint), and no crypto/LLM dependency is added to cortex-engine by THIS pillar (the determinism_hash uses the hash primitive provided by the receipt pillar; until it lands, this pillar uses a clearly-namespaced non-cryptographic content hash for the determinism harness and a TODO gate that flips to the real hash).

**Зависимости:** Accountability-receipt pillar (provides the real cryptographic hash primitive, e.g. blake3/sha2, that the determinism_hash and receipt header must use; this pillar's determinism_hash must adopt that primitive once available rather than shipping a second hashing scheme).; Canonical-serialization work is shared with the receipt pillar: this pillar OWNS canonical_bytes() for ContextPack and VerificationReport, and the receipt pillar consumes it for pack_root/verification_root. Sequence canonical_bytes() here first.; MUST-FIX correctness pillar (cosine dot.abs() at hnsw/metric.rs:44 and guarded-ANN post-filter recall): the learned ranker is trained on retrieval candidates; if the cosine/ANN bugs corrupt the candidate set, the learned weights overfit to a broken signal. Land those fixes before freezing production weights from real-embedding fixtures.; Existing offline gate infrastructure: scripts/learned_ranking_calibration_check.py, fixtures/enterprise_rag_bench/learned_ranking/offline_v1.jsonl, and mk/vars-core.mk LEARNED_RANKING_* vars are extended, not replaced.

### A.6 Correctness prerequisites (must-fix bugs) — making evidence trustworthy before the accountability receipt is built on top of it

**Текущее состояние:** CortexDB v0.2.0-beta.2 contains confirmed correctness defects that silently corrupt the very evidence an accountability receipt would attest, so they MUST land before the receipt is built (a signed-but-wrong receipt is a notarized lie). Confirmed in-file:

1. COSINE BUG (crates/cortex-engine/src/search/hnsw/metric.rs:44): `Some(((dot.abs() * 65_535) / norm.abs()) as u64)`. Taking `dot.abs()` makes an anti-correlated vector (v vs -v) score as a PERFECT match, and the intermediate `dot.abs() * 65_535` is i64 and can overflow on high-dimension i16 vectors. A correct reference implementation already exists in the same crate at crates/cortex-engine/src/context/dedup.rs:20-51 (`cosine_similarity_q16`: returns 0 when `dot_product <= 0`, uses i128/u128 widening via `integer_sqrt`). The two cosine implementations disagree — a latent inconsistency. `metric.rs` has NO unit tests (`grep` for `#[test]` in the file is empty), and the existing `ann-metric-matrix-check` fixture (crates/cortex-engine/src/search/ann_metric_matrix/tests.rs) only uses orthogonal/positively-correlated vectors, so the bug is invisible to every current gate. Cosine is selected at graph.rs:33, persisted_rrf.rs:10, ann/runtime.rs:61 and consumed monotonically (higher=better) by search_impl.rs.

2. CELL-ID SLOT-WIDTH COLLISION: memory uses a 31-bit agent slot `MEMORY_AGENT_SLOT_MASK = 0x7fff_ffff` with a GUARDED constructor `memory_cell_id` that returns `None` on overflow (crates/cortex-engine/src/cell_ids.rs:6,13-20). But session (crates/cortex-engine/src/session.rs:156,164) and feedback (crates/cortex-engine/src/feedback.rs:83,90) both mask the agent to 28 bits `0x0fff_ffff` and shift under a 32-bit sequence. Two agents differing only in bits 28-30 produce the SAME session/feedback cell-id while memory keeps them distinct — so a receipt could bind a cell to the wrong agent. Worse asymmetry: session.rs:163-173 at least has a collision-avoidance loop (`get_latest_cell_descriptor` re-probe, capped at u32::MAX attempts), but feedback.rs:82-92 has NO loop and NO guard — it SILENTLY `&`-mask-truncates (feedback.rs:83 `agent_id.0 & 0x0fff_ffff`, line 90 `sequence & 0xffff_ffff`). The three subsystems use distinct top-nibble namespaces (memory 0x8.., feedback 0x9.., session 0xA..) so cross-subsystem collision is impossible, but WITHIN feedback/session two agents collide.

3. CONFLICT-DETECTION CORRUPTS EVIDENCE (crates/cortex-engine/src/context/conflicts.rs:12-41 + extractor at dedup.rs:53-70): `measure()` groups by a lowercased raw-STRING (project,metric) and flags `values.len() > 1`. So `$1.2M` vs `1,200,000 USD` vs `1.2 million` register as a 3-way FALSE conflict, and format-equal-but-true conflicts in any non-`key=value` payload yield ZERO conflicts (the extractor only matches lines literally starting with `project=`/`metric=`/`value=`). This directly produces `conflict_visibility_q16`/`VisibleConflict` (conflicts.rs:43-62), which a receipt would attest. CRITICAL LEVERAGE: a complete numeric-normalization module ALREADY EXISTS and is unused here — crates/cortex-engine/src/verification/numeric/parse.rs (`extract_numeric_values`, `parse_currency_code`, `parse_unit_code`, `parse_magnitude_suffix`) and value.rs (`NumericValue::normalized_eq`, `conflicts_with`, `compare_numeric_values` with currency/unit/magnitude handling, all integer/Q16-deterministic). The fix is mostly wiring, not new parsing.

4. (Adjacent, accountability-relevant) guarded ANN applies the allowed-set as a POST-traversal filter under a shared visit budget (search_impl.rs:33-119, filter at :90, budget cutoff :74-77); `budget_exceeded` is already computed but never surfaced into the ContextPack, so a sparse-scope (most security-sensitive) agent can silently get an incomplete pack that a receipt would certify as complete.

Project has a STRONG gate culture: 280 `*-check`/`*-report` make targets following `<area>-<facet>-check`, typically `cargo test -p <crate> --test <name> && python3 scripts/<name>_check.py --report $(...)` (see mk/core-retrieval-context.mk:67-69 for context-pack-conflict-visibility-check, mk/core-contracts.mk:95 for engine-determinism-check, mk/ann.mk:19 for ann-metric-matrix-check).

**Целевое состояние:** Every evidence-bearing signal the accountability receipt will commit is correct, deterministic, and gate-protected before the receipt layer is built:

1. Cosine similarity is sign-correct and overflow-safe: cosine(v, -v) == 0, cosine(v, v) == max, a ranked search over {v, orthogonal, -v} orders v first and -v last, and no overflow on max-magnitude high-dimension i16 vectors. metric.rs and dedup.rs share ONE cosine implementation (or are proven equal by test) so the two can never drift again. A regression gate with an anti-correlation fixture fails if dot.abs() is ever reintroduced.

2. Cell-id encoding is collision-free and consistent: memory, session, and feedback all use ONE documented slot width via a shared `cell_ids` helper that REJECTS (returns None / errors) over-width ids instead of silently truncating. Distinct (agent_id, sequence) pairs in the documented domain provably never collide within any subsystem; feedback no longer silently `&`-mask-truncates. The width is documented as an invariant and any encoding change ships behind a schema-version/migration gate.

3. Conflict detection is numeric-aware: context/conflicts.rs reuses the existing verification/numeric module so normalized-equal values (e.g. $1.2M == 1,200,000 USD == 1.2 million) do NOT flag, true numeric conflicts DO flag across >=2 payload formats, currency/unit mismatches are handled as today's module specifies, non-numeric values fall back to string comparison, and the whole path stays integer/Q16-deterministic. Conflict recall/precision is MEASURED on a labeled unit/currency/magnitude fixture (no longer "unmeasured/heuristic").

4. Retrieval-incompleteness is honest: the already-computed `budget_exceeded` flag is surfaced into the ContextPack as an explicit completeness anomaly so a sparse-scope agent's pack discloses it may be incomplete (the minimal, cheap half of the guarded-ANN fix; full re-architecture is deferred to a separate pillar).

All four are locked behind new `*-check` make gates wired into the beta/release lane, following the project's existing gate convention.

| ID | Задача | Exit-гейт | Усилие | Зависит | Риск |
|---|---|---|---|---|---|
| `CP-1` | **Fix cosine dot.abs() sign bug and overflow in hnsw/metric.rs** — In crates/cortex-engine/src/search/hnsw/metric.rs:28-45, replace the `Self::Cosine` arm so it (a) returns 0 when the signed dot product <= 0 (anti-correlated and orthogonal-negative vectors must NOT score as matches), and (b) widens the *65_535 scaling step to i128/u128 to avoid i64 overflow on high-dimension i16 vectors. The proven-correct pattern already exists in crates/cortex-engine/src/context/dedup.rs:20-51 (cosine_similarity_q16: rejects dot<=0, i128 dot, u128 norms, integer_sqrt). Do NOT change DotProduct/L2 arms beyond what's needed; callers (search_impl.rs, index.rs, vector_index.rs) treat the output as a monotonic higher=better score, so the contract is preserved. Keep the i16 input / u64 output signature. | `make cosine-metric-correctness-check (new gate: cargo test -p cortex-engine --test cosine_metric_correctness asserting cosine(v,-v)==0, cosine(v,v)==max, ranked {v,orthogonal,-v} order, and no overflow on a max-magnitude high-dim vector; plus a python scripts/cosine_metric_correctness_check.py emitting a report). Gate FAILS if dot.abs() reappears.` | S | — | Low. Single pure function, monotonic contract preserved. Minor risk: an existing recall baseline (ann-metric-matrix / ann-recall fixtures) was tuned against the buggy scores; re-run ann-metric-matrix-check and refresh any cosine baseline that legitimately shifts. The current matrix fixture uses only orthogonal vectors so is unlikely to move. |
| `CP-2` | **Unify the two cosine implementations (metric.rs and dedup.rs) on one source of truth** — After CP-1, eliminate the latent divergence between hnsw/metric.rs cosine and context/dedup.rs::cosine_similarity_q16. Either (preferred) extract a single shared `cosine_similarity_q16(u,v)->u16` helper into a common module (e.g. a math/metric util) and have both call sites use it, or — if call-site signatures differ (u64 vs u16) — add a test that feeds an identical fixture battery to both and asserts byte-identical Q16 results. This prevents the bug from being half-fixed and silently re-diverging. | `Part of make cosine-metric-correctness-check: a test asserts metric.rs cosine and dedup.rs cosine produce identical Q16 output for a shared fixture set (anti-correlated, orthogonal, identical, high-dim). Passes only when implementations agree.` | S | CP-1 | Low. Refactor-only. Risk is touching dedup.rs's redundancy-threshold behavior; pin existing dedup tests (context_pack scoring tests) as regression guards before refactoring. |
| `CP-3` | **Unify cell-id slot width and reject (not truncate) over-width ids for memory/session/feedback** — Pick ONE documented agent-slot width (recommend 31 bits, matching memory's existing 0x7fff_ffff, since memory is already guarded) and route session (crates/cortex-engine/src/session.rs:156,164) and feedback (crates/cortex-engine/src/feedback.rs:83,90) through a shared helper in crates/cortex-engine/src/cell_ids.rs that mirrors memory_cell_id: it must return None / an EngineResult error on over-width agent_slot or sequence instead of the current silent `& 0x0fff_ffff` truncation. Feedback currently has NO collision loop and NO guard (the worst case); at minimum it must reject over-width ids; session already re-probes via get_latest_cell_descriptor and can keep that loop on top of the guarded constructor. Document the chosen width as an invariant in cell_ids.rs. Because this changes the on-disk feedback/session cell-id encoding, gate it behind a schema-version bump with a migration or refuse-to-read guard (the namespaces 0x8../0x9../0xA.. already separate subsystems, so only the in-subsystem agent-bit layout changes). | `make cell-id-collision-check (new gate: cargo test -p cortex-engine --test cell_id_collision — property/exhaustive test that distinct (agent_id,sequence) in the documented domain never collide across memory/session/feedback, and that an over-width id returns None/error instead of truncating; plus scripts/cell_id_collision_check.py report). Includes assertion that feedback no longer silently masks.` | M | — | Medium. Changes a persisted encoding -> needs a schema-version/migration gate so existing feedback/session cells remain readable; without it the receipt's byte-identical-across-versions invariant breaks. Mitigate with refuse-to-read-old-version guard + documented migration. Coordinate with the 'Migration policy' gates (migration-compatibility-check already exists). |
| `CP-4` | **Wire numeric normalization into conflict detection (context/conflicts.rs + dedup extractor)** — Make crates/cortex-engine/src/context/conflicts.rs::measure numeric-aware by reusing the EXISTING crates/cortex-engine/src/verification/numeric module (DO NOT write a new parser). Before comparing values within a (project,metric) group, parse each value with extract_numeric_values / build NumericValue and compare via NumericValue::normalized_eq / conflicts_with (value.rs:21-27, handles currency/unit/magnitude). Treat normalized-equal values as NON-conflicting (fixes the $1.2M vs 1,200,000 USD false 3-way conflict); count a group as a VisibleConflict only when at least two values genuinely conflict numerically; fall back to the current string comparison only for non-numeric values. Separately, broaden crates/cortex-engine/src/context/dedup.rs:53-70 extract_project_metric_value beyond literal `project=`/`metric=`/`value=` line prefixes so non-key=value payloads can surface conflicts (or document the supported formats explicitly). Preserve full integer/Q16 determinism — no floats. This directly improves the conflict_visibility_q16 / VisibleConflict signal the receipt will attest (GCE invariant 5). | `make conflict-normalization-check (new gate, mirroring context-pack-conflict-visibility-check: cargo test -p cortex-engine --test conflict_normalization over a labeled unit/currency/magnitude fixture with >=2 payload formats — asserts normalized-equal values do NOT flag, true conflicts DO, reports recall+precision numbers, and asserts identical inputs => identical conflict set + conflict_visibility_q16; plus scripts/conflict_normalization_check.py). Existing context-pack-conflict-visibility-check must still pass.` | M | — | Medium. The existing context-pack-conflict-visibility-check baseline may shift once false conflicts disappear and true ones appear; refresh that baseline deliberately and document the recall/precision delta. Edge cases: mixed currency vs unit, multi-value cells (numeric module's single-value extraction limit) — document remaining limits rather than over-reaching scope. |
| `CP-5` | **Surface guarded-ANN budget_exceeded as a ContextPack completeness anomaly** — The guarded ANN path already computes `budget_exceeded` (crates/cortex-engine/src/search/hnsw/search_impl.rs:33-119) but discards it. Thread that flag up so the ContextPack emits an explicit anomaly (reuse the ContextPackAnomaly mechanism in context/conflicts.rs:43-53 and context/mod.rs anomaly types — add a code like RetrievalIncomplete/BudgetExceeded) whenever the visit budget was hit during a scope-filtered search. This is the cheap, high-value HALF of the guarded-ANN issue: it does NOT re-architect the post-filter/pre-filter traversal (deferred to a separate retrieval-recall pillar), it only makes the existing incompleteness HONEST and receipt-attestable, so a sparse-scope agent's pack discloses it may be incomplete instead of certifying completeness falsely. | `make ann-budget-disclosure-check (new gate: cargo test -p cortex-engine --test ann_budget_disclosure — constructs a sparse-allowed-set search that hits max_visited and asserts the resulting ContextPack carries the budget-exceeded/RetrievalIncomplete anomaly; plus a scripts report). Asserts the flag is plumbed end-to-end into json_export.` | M | — | Low-Medium. Touches ContextPack schema (new anomaly code) -> additive optional field, must stay backward-compatible with context_pack.v1 (the schema permits additive optional fields until v2). Does not fix recall itself — must be clearly scoped as disclosure-only to avoid being mistaken for the full guarded-ANN remediation. |
| `CP-6` | **Aggregate the correctness gates into the beta/release lane** — Create the four new make targets (cosine-metric-correctness-check, cell-id-collision-check, conflict-normalization-check, ann-budget-disclosure-check) in the appropriate mk/*.mk files following the existing convention (cargo test + python scripts/*_check.py --report $(VAR), .PHONY registration in mk/phony.mk, report-path vars in mk/vars-*.mk). Add an aggregate target (e.g. correctness-prerequisites-check) that runs all four, and wire it into the beta release lane (mk/release.mk / beta-* aggregates) so these are blocking before any receipt work. Ensure existing ann-metric-matrix-check, context-pack-conflict-visibility-check, and engine-determinism-check are run alongside and still pass (refresh any baselines that legitimately shift due to CP-1/CP-4, documenting the delta). | `make correctness-prerequisites-check passes in CI and is referenced from the beta release aggregate; the four sub-gates and the three pre-existing related gates all green.` | S | CP-1, CP-3, CP-4, CP-5 | Low. Plumbing only. Risk is forgetting .PHONY/report-var wiring (the repo is strict about this — see mk/phony.mk); follow an existing check as a template (context-pack-conflict-visibility-check). |

**Измеримые критерии готовности столпа:**
- make cosine-metric-correctness-check passes: asserts cosine(v,-v)==0, cosine(v,v)==65535-or-clamped-max, ranked order v > orthogonal > -v, and no panic/overflow on a max-magnitude high-dim i16 vector; the gate FAILS if metric.rs is reverted to dot.abs()
- metric.rs and dedup.rs cosine are unified or a test proves byte-identical Q16 output for a shared fixture battery (no divergent implementations remain)
- make cell-id-collision-check passes: property/exhaustive test proves distinct (agent_id,sequence) in the documented domain never collide across memory/session/feedback, and that an over-width id returns None/errors instead of truncating; feedback no longer silently &-mask-truncates
- make conflict-normalization-check passes: on a labeled fixture with >=2 payload formats, normalized-equal values (e.g. $1.2M / 1,200,000 USD / 1.2 million) do NOT flag and true numeric conflicts DO flag, with reported recall and precision numbers; determinism sub-assertion: identical inputs => identical conflict set + identical conflict_visibility_q16
- ContextPack surfaces a budget-exceeded/retrieval-incomplete anomaly when the guarded-ANN visit budget is hit (asserted by test), so incompleteness is disclosed rather than hidden
- all new gates are wired into the existing beta/release lane (e.g. referenced from mk/release.mk or the beta-* aggregate) and pass in CI; existing ann-metric-matrix-check, context-pack-conflict-visibility-check, and engine-determinism-check still pass (no regression)
- any on-disk encoding change (feedback/session cell-id) ships behind a schema-version bump with a migration or refuse-to-read guard so the receipt's same-inputs=>byte-identical invariant is not silently broken across versions

**Зависимости:** None external — this pillar is the FIRST in the roadmap sequence and blocks the 'Accountability Receipt' pillar (receipt must bind trustworthy evidence); The 'Real cryptography' pillar (BLAKE3/Ed25519, replacing FNV) is INDEPENDENT of this pillar and can proceed in parallel; the receipt pillar depends on BOTH this and the crypto pillar; Conflict-normalization task reuses the existing crates/cortex-engine/src/verification/numeric module (already in-tree, no new dependency)

### A.7 Absorption proof + open GCE specification · **категориообразующий**

**Текущее состояние:** CortexDB already ships the SEMANTIC inputs a receipt would bind but NONE of the closure that makes them third-party-checkable, and there is zero published category spec. Concretely, verified in-tree: (1) ContextPack is a first-class typed result with per-cell access_decision (crates/cortex-engine/src/context/mod.rs:108-132; produced at context/pack/access.rs:8-46), span provenance (context/mod.rs:134-142; context/span.rs:29-73), conflict_visibility_q16/visible_conflict_count (context/mod.rs:152-153; context/conflicts.rs:12-41), and token-budget accounting (context/mod.rs:147-149). (2) The JSON contract is frozen as context_pack.v1 with an EXPLICIT 'additive optional fields allowed until v2' clause (docs/schemas/context_pack.v1.json:5), emitted by a fixed-key json! macro that omits clocks (context/export/json_export.rs:6-81) — so a receipt can be added as one additive top-level field without breaking v1. (3) Fail-closed-by-construction is real and is the genuine moat: the binder seeds every retrieve plan with [PushAgentAllowed, PushLive, And] and only ever AND-s the user WHERE (crates/cortex-aql/src/binder.rs:137-145), so plan-algebra widening is impossible — strictly stronger than the Cerbos/OPA 'app applies the filter out-of-band' model. (4) Determinism exists only as in-repo snapshot tests (crates/cortex-engine/tests/determinism.rs:27-70) with NO exported determinism hash and NO canonical serializer. NOW THE GAPS: (a) zero real crypto in the workspace — every integrity surface is FNV-1a64 (audit_chain.rs:8-9,42-48) or XOR+FNV keystream with a forgeable FNV 'auth tag' (backup/encrypted/crypto.rs:3-45); no sha2/blake3/ed25519/hmac as direct deps (confirmed: only transitive ring/getrandom via TLS). (b) NO receipt/merkle/signature type or schema anywhere (grep across crates+docs/schemas returns only an unrelated graph_signature string). (c) NO open category spec — docs/ describes CONTEXT_PACK.md and VERIFY_FACT.md as product docs, not a normative result-type spec others can implement to. (d) The baseline_comparison_check.py harness (mk/core-retrieval-context.mk:5-9; scripts/baseline_comparison_*.py) compares a STATIC feature matrix, never running a live pgvector+policy stack, so the absorption claim is asserted not benchmarked. (e) VerificationExecutionReport embeds wall-clock total_elapsed_nanos/elapsed_nanos (verification/operator.rs:28,188,203) which would break any byte-identical receipt. (f) cosine uses dot.abs() (search/hnsw/metric.rs:44) so the evidence signal a receipt would attest is corrupt. The project has a STRONG gate culture (per-gate Python self-test + cargo test + JSON evidence report, wired into release lanes via mk/*.mk; pattern e.g. context-pack-private-scope-check at mk/core-retrieval-context.mk:62-65 with scripts/context_pack_private_scope_check.py) which this pillar must reuse.

**Целевое состояние:** The ContextPack result type and accountability_receipt.v1 are PUBLISHED as an open, versioned, normative specification (frozen JSON Schemas + prose threat model + canonicalization rules) under docs/schemas/ and docs/spec/, such that an independent implementer can build a conforming GCE and a third party can verify any CortexDB answer WITHOUT trusting the database. A standalone open-source verifier binary (cortex-receipt-verify) that does NOT link cortex-engine internals validates a (pack.json + receipt.json + raw admitted cell bytes + public key) tuple offline and exits 0/non-0. A published, reproducible head-to-head benchmark (AAB head-to-head: CortexDB vs a real pgvector + OPA/Cedar + RAG-library baseline) demonstrates the structural result: the baseline can match retrieval/citation/token metrics but scores UNRANKED on receipt-verifiability and determinism because it cannot (i) emit a third-party-checkable receipt, (ii) bind access decisions forced by plan algebra, or (iii) prove byte-identical reproducibility — the Oso closure made empirical, not asserted. A public conformance + adversarial test suite (scope-widening attempts, fabricated citations, dropped VisibleConflict, forged audit entries, anti-correlation traps) is shipped so 'just wrap pgvector' is a strictly weaker, demonstrably non-conforming product. All of this is gated: new make-*-check gates (spec freeze, standalone verifier, tamper rejection, baseline matrix, conformance) join the existing release lanes with JSON evidence reports, consistent with the project's gate-and-evidence discipline and PUBLIC_CLAIMS_POLICY (claims stay scoped to single-node beta; the spec/verifier/benchmark are described as the absorption-resistance evidence, not production guarantees).

| ID | Задача | Exit-гейт | Усилие | Зависит | Риск |
|---|---|---|---|---|---|
| `SPEC-1` | **Publish the open GCE contract specification (ContextPack result type + six invariants)** — Write docs/spec/GCE_CONTRACT.md as the NORMATIVE category specification (not product marketing): define the ContextPack result type field-by-field referencing context/mod.rs:97-192, the six GCE invariants (result=compiled governed context; deterministic LLM-free Q16 governance; fail-closed by plan algebra per binder.rs:137-145; provenance+verification as first-class outputs; conflict preservation not LWW; TTL/decay as ranking signal), and the conformance obligations an implementer must meet. Cross-link the frozen docs/schemas/context_pack.v1.json. Keep claims inside PUBLIC_CLAIMS_POLICY (single-node beta scope). | `make gce-spec-doc-check (new): a Python self-test asserts docs/spec/GCE_CONTRACT.md contains every required section/term (the six invariants, ContextPack field list cross-checked against context/mod.rs, conformance-obligation list) and emits a JSON evidence report; mirrors scripts/context_pack_private_scope_check.py term-coverage pattern.` | M | — | Spec over-promises beyond what the engine actually guarantees, violating PUBLIC_CLAIMS_POLICY. Mitigate by deriving every spec clause from a cited source file and gating on term-coverage against the code. |
| `SPEC-2` | **Freeze the accountability_receipt.v1 JSON Schema as an additive field on the pack contract** — Define docs/schemas/accountability_receipt.v1.json with the signed fixed-size header {schema_version, hash_alg:'blake3-256', sig_alg:'ed25519', db_instance_id, key_id, pack_root, cell_set_root, access_root, provenance_root, verification_root, budget_commitment, conflict_commitment, determinism_hash, signature} plus the leaf structures. Add it as ONE additive optional top-level 'accountability_receipt' field, honoring context_pack.v1.json:5 ('additive optional fields allowed until v2'). Extend json_export.rs to emit it. Specify the canonicalization (RFC 8785 JCS or explicit byte form) as the normative wire format since json! does not recursively sort keys. | `make accountability-receipt-spec-check (new): cargo test asserts json_export emits a receipt object that validates against accountability_receipt.v1.json; a schema-freeze diff gate fails on breaking changes (mirror context-pack-schema-contract-check, mk/core.mk:33). JSON evidence report written.` | M | SPEC-1 | Receipt emission depends on the sibling Accountability Receipt pillar's crypto primitives; if those slip, the schema can be frozen but the emit/validate test stays red. Mitigate by landing schema + canonicalizer first and stubbing emit behind a feature flag. |
| `SPEC-3` | **Specify and document the offline verifier algorithm + threat model** — In docs/spec/GCE_CONTRACT.md (or a sibling docs/spec/RECEIPT_VERIFIER.md), write the precise offline verifier algorithm (inputs: pack.json + receipt.json + raw bytes of each admitted cell + DB public key; steps: verify Ed25519 over canonical header; recompute cell_content_hash and rebuild all leaves/roots; REJECT any admitted cell whose access leaf != allowed; assert every cited span lies within and matches the referenced cell bytes; assert sum(estimated_tokens) <= token_budget_tokens; assert verdict/conflicts reference only admitted cell_ids; recompute determinism_hash). Enumerate the explicit threat model: what a malicious/buggy DB MUST NOT be able to forge (admit unreadable cell, fabricate citation, drop VisibleConflict, overspend budget, claim false determinism, reuse another query's receipt) and the documented out-of-scope item (issuer equivocation -> optional transparency log). | `make receipt-threat-model-check (new): self-test asserts the doc enumerates all 7 verifier steps and all named forgery classes, each mapped to the schema field that defends it; JSON evidence report.` | M | SPEC-2 | Threat model claims protection the implementation does not yet deliver. Mitigate by gating each claim to a corresponding tamper test in VERIF-2 (every documented forgery class must have a passing tamper test). |
| `VERIF-1` | **Build the standalone cortex-receipt-verify crate (no engine dependency)** — Create a new workspace member crates/cortex-receipt-verify (binary + lib) that implements the SPEC-3 algorithm using ONLY the receipt JSON, pack JSON, raw cell bytes, and the public key. It MUST NOT depend on cortex-engine, cortex-server, or cortex-aql (proving 'verify without trusting the DB'). Depends only on serde_json + blake3 + ed25519-dalek. Add it to the workspace members list in Cargo.toml. | `make accountability-receipt-verify-check (new): the binary validates 100% of a fixture corpus of genuine packs+receipts; a cargo-metadata dependency assertion (script) proves the crate's dependency closure excludes cortex-engine/cortex-server/cortex-aql. JSON evidence report.` | L | SPEC-3 | Verifier accidentally re-derives a value the DB should have committed (trust leak) or links engine code transitively. Mitigate with the explicit cargo dependency-closure gate and a code review that the verifier reads only public inputs. |
| `VERIF-2` | **Adversarial tamper-rejection test suite over the receipt** — Add a test crate/fixture set producing genuine (pack, receipt) pairs, then apply each mutation class from SPEC-3's threat model and assert cortex-receipt-verify REJECTS: (1) flip one estimated_tokens; (2) flip one access_decision Allowed->NotRecorded; (3) alter one provenance byte_start; (4) drop one VisibleConflict from the pack; (5) rewrite/truncate the bound audit chain head; (6) swap the verdict against a different pack; (7) substitute one admitted cell's raw bytes (content-hash mismatch). Each mutation maps 1:1 to a documented forgery class. | `make accountability-receipt-tamper-check (new): all 7 mutation classes cause verifier exit non-zero; a 0-false-negative assertion (genuine pack accepted). JSON evidence report with per-mutation pass/fail.` | M | VERIF-1 | A mutation class is undetectable because the corresponding field is not actually bound into a signed root (e.g. dropped conflict not in conflict_commitment). Mitigate by treating any undetectable mutation as a SPEC-2 schema bug and adding the missing commitment. |
| `VERIF-3` | **Cross-process byte-identical determinism gate for pack+receipt** — Extend the determinism harness (crates/cortex-engine/tests/determinism.rs) and add a script that runs RETRIEVE CONTEXT + VERIFY FACT twice in two independent OS processes on the same fixed store and asserts byte-identical canonical pack+receipt and identical determinism_hash. Prove wall-clock fields (verification/operator.rs:28,188,203 total_elapsed_nanos/elapsed_nanos) are excluded from the canonical/hashed surface (move them out of the hashed report or strip in canonicalization). | `make accountability-receipt-determinism-check (new): two independent processes yield identical determinism_hash and byte-identical receipts; a static assertion that elapsed_nanos fields are absent from the canonical serializer. Extends engine-determinism-check (mk/core-contracts.mk:95). JSON evidence report.` | M | VERIF-1 | Hidden non-determinism (HashMap iteration, float, locale) leaks into canonical bytes. Mitigate by mandating BTreeMap/integer-only canonical form and gating on cross-process (not just same-process) equality. |
| `BASE-1` | **Build a live pgvector + policy-engine baseline harness (real stack, not a feature matrix)** — Replace the static feature-matrix comparison (scripts/baseline_comparison_*.py, mk/core-retrieval-context.mk:5-9) with a runnable baseline: Postgres+pgvector for retrieval, OPA or Cedar as the policy engine applied app-side (the Cerbos query-plan model), and a RAG library for citations/conflicts. Ingest the SAME corpus used by CortexDB gates (examples/real_domains). Capture its outputs in a comparable JSON shape so the same scorer can rate both systems. | `make aab-baseline-stack-check (new): the baseline harness ingests the shared corpus and answers a fixed query set, emitting per-answer artifacts (admitted docs, citations, conflicts, token count) plus a self-test; JSON evidence report. (Containerized/optional-in-CI like continuous-benchmark gates.)` | L | — | Building/operating a live pgvector+OPA stack in CI is heavy and flaky. Mitigate by running it in the nightly lane (like continuous-benchmark-hosted-gate) with a small checked-in fixture for CI, full corpus nightly. |
| `BASE-2` | **Head-to-head accountability scoring + absorption-proof report** — Implement a six-axis scorer (scope-leak, citation precision/recall, contradiction recall + false-conflict rate, tokens-to-answer at fixed budget, receipt-verifiability, determinism) that runs BOTH CortexDB and the BASE-1 baseline at a fixed token budget. Headline metric is GATED by axes 5+6: a system with receipt-verifiability < 100% or any undetected tamper is UNRANKED. Produce docs-bound report showing the baseline competitive on axes 1-4 but UNRANKED on 5-6 (it emits no third-party-checkable receipt and cannot fail-closed by plan construction), making the Oso closure empirical. | `make aab-baseline-matrix-report (new): produces a report where the live pgvector+policy baseline scores 0/UNRANKED on receipt-verifiability + determinism while CortexDB passes all six axes; scorer self-test included. JSON + Markdown evidence (mirrors baseline-comparison-check outputs).` | L | BASE-1, VERIF-1, VERIF-3 | Reviewers dismiss it as a rigged comparison. Mitigate by making axes 1-4 reuse established methodology (ALCE/TREC citation, ConflictBank conflicts) so the baseline genuinely wins those, leaving 5-6 as the sole, structural differentiator; publish the harness so others can reproduce. |
| `CONF-1` | **Publish the public conformance + adversarial test suite for GCEs** — Assemble a self-contained, downloadable conformance suite that any candidate GCE (including a documented 'thin pgvector wrapper' reference) is run against: scope-widening attempts (must be impossible by plan algebra / rejected), fabricated-citation cases (verifier must reject), dropped-VisibleConflict cases, forged-audit-entry cases, and anti-correlation traps (depends on the cosine fix). Include the standalone verifier as the conformance oracle. Document pass criteria so 'wrap pgvector' is demonstrably non-conforming. | `make aab-conformance-check (new): CortexDB passes all conformance cases; the bundled thin-wrapper reference provably fails >=3 axes (scope-widening, receipt-verifiability, determinism). JSON evidence report enumerating per-case results.` | M | VERIF-2, BASE-2 | Anti-correlation trap gives false results until cosine dot.abs() is fixed (MUST-FIX pillar). Mitigate by ordering this after that fix lands and asserting cosine(v,-v)==0 as a precondition in the suite setup. |
| `WIRE-1` | **Wire the new accountability gates into release lanes and the gate index** — Add the new gates (gce-spec-doc-check, accountability-receipt-spec-check, receipt-threat-model-check, accountability-receipt-verify-check, accountability-receipt-tamper-check, accountability-receipt-determinism-check, aab-conformance-check) into the beta release lane, and the heavier nightly ones (aab-baseline-stack-check, aab-baseline-matrix-report) into the nightly/continuous lane. Follow the existing <area>-<facet>-check naming and the self-test + cargo test + JSON-report convention. Add a new mk include (e.g. mk/accountability-receipt.mk) and aggregate into a parent accountability-check target. | `make accountability-check (new aggregate) runs all CI-safe receipt gates green and is referenced from the release lane; a phony-target audit (mk/phony.mk pattern) confirms all new targets are declared .PHONY.` | S | SPEC-2, VERIF-2, VERIF-3, CONF-1 | CI time blows up if heavy baseline gates land in the fast lane. Mitigate by splitting CI-safe (mini fixtures) from nightly (full corpus + live baseline stack), matching the existing continuous-benchmark lane split. |

**Измеримые критерии готовности столпа:**
- make accountability-receipt-spec-check passes: docs/schemas/accountability_receipt.v1.json and docs/schemas/context_pack.v2.json (or v1 additive) are frozen, validate against the live json_export output, and a schema-freeze diff gate fails on any breaking change (mirrors context-pack-schema-contract-check at mk/core.mk:33).
- make accountability-receipt-verify-check passes: the standalone cortex-receipt-verify crate (NOT depending on cortex-engine) validates 100% of a fixture corpus of genuine packs+receipts offline using only (pack.json, receipt.json, raw cell bytes, public key); a cargo dependency-graph assertion proves the verifier does not link cortex-engine/cortex-server.
- make accountability-receipt-tamper-check passes: for each mutation class (flip one estimated_tokens; flip one access_decision Allowed->NotRecorded; alter one provenance byte_start; drop one VisibleConflict; truncate/rewrite the audit chain; swap a verdict against a different pack) the verifier MUST reject (exit non-zero), proving soundness against the documented threat model.
- make accountability-receipt-determinism-check passes: RETRIEVE CONTEXT and VERIFY FACT run twice on a fixed store over >=2 independent processes produce byte-identical canonical pack+receipt and identical determinism_hash; wall-clock fields (operator.rs total_elapsed_nanos/elapsed_nanos) are provably excluded from the hashed surface (extends engine-determinism-check at mk/core-contracts.mk:95).
- make aab-baseline-matrix-report produces a reproducible head-to-head report where a live pgvector + OPA/Cedar + RAG baseline is run on the same corpus and scores UNRANKED (0) on the receipt-verifiability and determinism axes while CortexDB scores >=target on all six axes — making absorption resistance an empirical, sourced finding (extends baseline-comparison-check at mk/core-retrieval-context.mk:5).
- make aab-conformance-check passes: a published conformance + adversarial suite (scope-widening, fabricated-citation, dropped-conflict, forged-audit, anti-correlation) is run; CortexDB passes all and a documented 'thin wrapper' reference attempt provably fails >=3 axes, with the result written to a JSON evidence report.
- An open GCE specification doc (docs/spec/GCE_CONTRACT.md) is published defining: the ContextPack result type, the six GCE invariants, the accountability_receipt.v1 leaf/root/signature structure, the canonicalization (RFC 8785 JCS or explicit byte form), and the exact offline verifier algorithm + threat model (what a malicious DB MUST NOT be able to forge) — sufficient for an independent party to implement a conforming GCE.

**Зависимости:** Accountability Receipt pillar: must land real crypto primitives (BLAKE3/SHA-256 hash + Ed25519 signature, replacing FNV-1a64 in audit_chain.rs and XOR-FNV in backup/encrypted/crypto.rs) and emit the canonical receipt object (pack_root, determinism_hash, signed header). This pillar's verifier, tamper, and determinism gates consume those primitives and CANNOT pass on FNV.; MUST-FIX correctness prerequisites pillar: cosine dot.abs() fix (metric.rs:44) and guarded-ANN budget_exceeded surfacing — a receipt that signs a corrupt evidence signal is 'a notarized lie'; the conformance suite's anti-correlation trap depends on the cosine fix.; Determinism prerequisite: wall-clock fields in verification/operator.rs (total_elapsed_nanos/elapsed_nanos) must be moved out of the hashed/canonical surface before the determinism gate can pass.

### A.8 Accountability benchmark + production scale · **категориообразующий**

**Текущее состояние:** CortexDB has a surprisingly mature distributed substrate that the headline "single-node beta, HA = research track" understates, but it is gated as research and is NOT proven to preserve the accountability invariants across a cluster, and there is NO accountability benchmark or competitor comparison at all.

DISTRIBUTED/CONSENSUS (more built than ground-truth claimed): crates/cortex-engine/src/replication/ contains a full Raft-style stack — consensus.rs, election.rs, log.rs, log_matching.rs, membership.rs, snapshot.rs, install.rs, recovery.rs, repair/, rotation.rs, runtime.rs, tcp.rs, transport.rs, peer.rs. There are ~25 replication tests (crates/cortex-engine/tests/replication_*.rs incl. replication_log_matching, replication_election, replication_membership, replication_partition_matrix, replication_consensus_hardening, replication_failure_injection, crash_consistency_fault_injection) plus full_stack_consistency.rs. Gates exist: distributed-consensus-check, consensus-partition-soak-check, consensus-failover-slo-check, consensus-rejoin-check, aggregated by distributed-consensus-research-check (mk/core-security-ops.mk:129-148), and replication-partition-check / replication-lifecycle-check (mk/storage-ops.mk:88-92). CRITICAL: only replication-lifecycle-check is wired into release-check (mk/release.mk:53); the partition-soak/failover-SLO/rejoin suite sits under distributed-consensus-research-check and is NOT in the release lane. There is NO test that the fail-closed binder seed [PushAgentAllowed, PushLive, And] (crates/cortex-aql/src/binder.rs:137-145) is preserved on a follower read, across a leader failover, or during a partition — i.e. cluster-level fail-closed is unproven.

MULTI-AGENT CONSISTENCY: crates/cortex-engine/tests/multi_agent_consistency.rs (257 lines) is SINGLE-NODE agent-transaction conflict semantics only (no leader/follower/replica/partition references). crates/cortex-engine/src/multi_agent_consistency.rs defines MemoryConsistencyLevel but no test exercises a chosen consistency level across replicas (read-your-writes / monotonic-read across nodes is unproven).

ACCOUNTABILITY BENCHMARK: does not exist. scripts/ has LongMemEval + LoCoMo retrieval adapters and a SQLite-FTS5/dense/hybrid baseline-comparison harness (scripts/baseline_comparison_*.py, mk/core-retrieval-context.mk:5-9) that scores RETRIEVAL/QA only — no scope-leak, citation precision/recall (NLI), contradiction recall/false-conflict, tokens-to-answer (real tokenizer), receipt-verifiability, or determinism axes. There are NO Zep/Mem0/Cognee/pgvector-policy adapters anywhere.

RECEIPT/CRYPTO/DETERMINISM (prerequisites this pillar consumes): no receipt/Merkle/signature scaffolding exists (grep for receipt/merkle/determinism_hash/pack_root across crates + docs/schemas returns nothing). ZERO crypto deps in any Cargo.toml (only roaring). Audit chain is unkeyed FNV-1a64 (crates/cortex-server/src/audit_chain.rs:42-48). Determinism is in-repo snapshot strings only (crates/cortex-engine/tests/determinism.rs) with no exported determinism_hash. ContextPackAnomalyCode (crates/cortex-engine/src/context/mod.rs:164-171) has variants RedundantCell/MissingCitation/TokenOverload/ScopeMismatch/InsufficientContext/VisibleConflict but NO retrieval-incomplete/budget-exceeded code, and HNSW's budget_exceeded (crates/cortex-engine/src/search/hnsw/search_impl.rs:75,119) is computed but never surfaced into the pack. Confirmed accountability-corrupting bugs remain: cosine dot.abs() (search/hnsw/metric.rs:44 vs correct ref context/dedup.rs:35), guarded-ANN post-filter under shared budget (search_impl.rs:90), and conflict heuristic with no unit/currency normalization (context/conflicts.rs).

**Целевое состояние:** A public, reproducible Answer Accountability Benchmark (AAB-1) is published and CI-gated, scoring CortexDB AND at least four rival stacks (Zep/Graphiti, Mem0, Cognee, pgvector+policy-engine) on six axes — scope-leak@budget, citation precision/recall, contradiction recall + false-conflict rate, tokens-to-answer, receipt-verifiability, determinism — with the headline score GATED so any system that cannot emit a third-party-verifiable receipt or be byte-deterministic is UNRANKED, not merely low. The benchmark ships a checked-in CI-safe AAB-mini and a downloadable AAB-full, a standalone open-source verifier binary that validates a pack+receipt WITHOUT the database, and a signed public leaderboard pack. CortexDB self-reports only AFTER the four prerequisite accountability bugs are fixed (per the repo's PUBLIC_CLAIMS_POLICY discipline).

On the production-scale side, the existing Raft/replication stack is PROMOTED from research-track into the release lane: distributed-consensus-research-check's partition-soak/failover-SLO/rejoin gates are wired into release-check, multi-agent consistency is proven ACROSS NODES (read-your-writes and monotonic-read at a declared MemoryConsistencyLevel survive leader failover and partition), and — the keystone — the fail-closed binder invariant [PushAgentAllowed, PushLive, And] is proven to hold on follower reads, across leader failover, and during partitions, so no node can ever widen scope or serve an unfiltered/stale-widened answer. The accountability receipt is emitted identically by every node (same inputs => byte-identical pack+receipt regardless of which replica served it), and the audit chain head is committed into the receipt so the answer's provenance survives node loss. Net: the accountability contract is independently measurable, competitively crowned, and holds under production failure modes — not just on a single beta box.

| ID | Задача | Exit-гейт | Усилие | Зависит | Риск |
|---|---|---|---|---|---|
| `AAB-1` | **Define and freeze the AAB-1 benchmark spec + scoring + JSON Schema** — Author docs/AAB.md and docs/schemas/aab_run.v1.json defining the six axes (scope-leak@budget, citation precision/recall via NLI, contradiction recall + false-conflict rate, tokens-to-answer at a fixed reference tokenizer, receipt-verifiability, determinism), the fixed token budgets B in {2k,4k,8k}, and the GATING rule: any system that fails receipt-verifiability (<100%) or determinism (not byte-identical) is UNRANKED, not low-ranked. Specify the headline as a harmonic combination gated by axes 5-6. Reuse established methodology (ALCE TRUE/NLI for citations, ConflictBank/MAGIC for conflicts, permission-aware-RAG leakage for scope, LongMemEval/LoCoMo as QA substrate) and document exactly what is reused vs new. Freeze field order for determinism. | `make aab-spec-check validates docs/schemas/aab_run.v1.json against a fixture run, asserts schema_version 'aab_run.v1', and rejects any run JSON missing one of the six axes or the gating verdict` | M | Pillar: Accountability receipt + real crypto | Methodology disputes from reviewers if axes are not grounded in prior art; mitigate by citing ALCE/TREC-RAG/ConflictBank explicitly and shipping the spec as versioned/frozen so later changes are visible. |
| `AAB-2` | **Build AAB-mini fixture corpus with governance overlays** — Create a checked-in CI-safe 50-100 query fixture (mirroring existing balanced_50 subsets) layering scope/agent ACLs onto LongMemEval/LoCoMo sessions with injected 'forbidden-but-relevant' distractors (highest-value answer in an unreadable scope), gold citation/attribution labels (ALCE-style), and labeled numeric/unit/currency contradictions (to exercise the conflict normalization fix). Keep it deterministic and small enough for the alpha-check budget. Stage AAB-full (downloadable, non-CI) separately. | `make aab-mini-fixture-check validates the fixture: every query has a scope ACL, >=1 forbidden-but-relevant distractor exists, >=1 gold-citation label and >=1 labeled numeric conflict are present, and the fixture loads deterministically` | M | AAB-1 | Corpus licensing/redistribution for LongMemEval/LoCoMo/ALCE; mitigate by shipping only derived governance-label files + loader scripts that pull upstream, not the raw corpora. |
| `AAB-3` | **Implement the AAB harness + four core CortexDB axes (scope-leak, citation, contradiction, tokens)** — Build scripts/aab/ harness that runs CortexDB through RETRIEVE CONTEXT + VERIFY on the AAB-mini fixture at each budget and computes: scope-leak rate (forbidden-scope cells in pack OR cited, target 0.000), citation precision/recall via an NLI entailment scorer plus TREC-RAG sentence-support, contradiction recall + false-conflict rate against labeled conflicts, and tokens-to-answer using one pinned reference tokenizer (cl100k/o200k) with the deterministic profile as secondary. Extends context-pack-private-scope-check for scope. Emits an aab_run.v1 JSON. | `make aab-scope-leak-check (0 forbidden cells across queries and all export formats at B in {2k,4k,8k}), make aab-citation-attribution-check (NLI precision/recall reported on fixture), make aab-contradiction-recall-check (recall + false-conflict rate reported), and make aab-tokens-to-answer-check (reference-tokenizer headline) all green` | L | AAB-1, AAB-2, Pillar: MUST-FIX correctness prerequisites | NLI scorer introduces an LLM/model dependency that could be nondeterministic; pin model + seed and report it as an external judge, keep CortexDB's own path LLM-free. Contradiction recall will be honest-low until the conflict-normalization prerequisite lands. |
| `AAB-4` | **Wire receipt-verifiability + determinism axes into AAB** — Integrate the standalone verifier binary (from the receipt pillar) into the harness: for every AAB query, emit the pack+receipt, run the out-of-process verifier with only (pack.json, receipt.json, cell bytes, public key), and score receipt-verifiability as % validated (binary: 100% or DISQUALIFIED). Add a tamper sub-score: mutate one access decision / one cited byte offset / one budget number and assert the verifier REJECTS. Score determinism by running each query twice in independent processes and asserting byte-identical pack+receipt (exported determinism_hash equal). | `make aab-receipt-verifiability-check (100% of fixture receipts validate out-of-process AND every tamper mutation is rejected) and make aab-determinism-check (byte-identical pack+receipt + equal determinism_hash across two independent processes) both green` | M | AAB-1, AAB-3, Pillar: Accountability receipt + real crypto | If the receipt pillar slips, these axes cannot be scored honestly; hard-block CortexDB self-reporting (not the harness) until the verifier binary exists. Determinism may expose hidden nondeterminism (map iteration, elapsed_nanos) — budget time to canonicalize. |
| `AAB-5` | **Build competitor adapters: Zep/Graphiti, Mem0, Cognee, pgvector+policy** — Implement scripts/aab/adapters/ for each rival running the SAME AAB-mini queries: Zep/Graphiti, Mem0, Cognee, and a pgvector+(OPA/Cedar)+RAGAS+LangChain 'thin library' stack. Each adapter scores axes 1-4 where the rival supports them, and scores axes 5-6 as 0/UNRANKED (none emit a third-party-verifiable receipt; none can fail-closed by binder construction). Produce the head-to-head matrix that demonstrates the structural (not leaderboard-delta) finding. | `make aab-baseline-matrix-report emits a table with all five systems on all six axes; the report assertion passes only if every rival is UNRANKED on receipt-verifiability AND determinism while CortexDB is RANKED` | XL | AAB-3, AAB-4 | Heavy external-dependency surface (4 third-party stacks, API keys, version drift). Mitigate by pinning versions in a lockfile, making the matrix a nightly/non-CI report (like continuous-benchmark-gate), and snapshotting rival outputs so the matrix is reproducible offline. |
| `AAB-6` | **Ship AAB-mini CI gate + AAB-full report + signed leaderboard pack** — Wire make aab-mini-check (CI-safe, all six axes end-to-end on the fixture) into the beta/release lane alongside the three receipt gates. Keep aab-full-report and aab-baseline-matrix-report nightly (pattern of continuous-benchmark-gate). Build make aab-leaderboard-pack that assembles signed AAB run reports for public submission, and publish the standalone verifier binary + spec so submissions are independently checkable. | `make aab-mini-check is green and present in alpha-check/release-check; make aab-leaderboard-pack produces an Ed25519-signed bundle that the standalone verifier validates without the DB` | M | AAB-3, AAB-4, AAB-5 | Adding a heavy gate to the release lane could slow CI; keep aab-mini truly mini and push full/baseline to nightly. |
| `SCALE-1` | **Prove fail-closed binder invariant across the cluster (follower read, failover, partition)** — Add crates/cortex-engine/tests/cluster_fail_closed.rs asserting that the binder seed [PushAgentAllowed, PushLive, And] (binder.rs:137-145) is preserved and returns ZERO out-of-scope cells when: (a) a read is served by a follower, (b) a read happens mid leader-failover, (c) a read happens during a network partition (using the existing replication_partition_matrix / replication_failure_injection harnesses). This is the keystone that makes the accountability receipt honest in production — a node must never widen scope or serve an unfiltered stale-widened answer. | `make consensus-failover-binder-check runs the new test and asserts 0 out-of-scope cells across follower-read, failover, and partition scenarios, plus that the receipt emitted in each scenario still carries an 'allowed' access decision for every cell` | L | Pillar: Accountability receipt + real crypto | May surface real correctness gaps where a follower serves a stale AgentView/policy version (scope drift across nodes). Budget for fixing AgentView/policy-version replication, not just testing it. |
| `SCALE-2` | **Extend multi-agent consistency to cross-node read-your-writes + monotonic-read** — Today multi_agent_consistency.rs is single-node agent-transaction semantics. Add a cluster variant that, at a declared MemoryConsistencyLevel (crates/cortex-engine/src/multi_agent_consistency.rs), asserts read-your-writes and monotonic-read hold ACROSS NODES through one leader failover and one partition heal — i.e. an agent that wrote on the leader and reads from a follower never sees an older state than its own write at the chosen level. | `make multi-agent-cluster-consistency-check asserts read-your-writes and monotonic-read across nodes survive a leader failover and a partition heal at the declared consistency level` | L | SCALE-1 | Strong consistency across nodes may require routing reads to the leader or waiting for commit-index catch-up, adding latency; document the consistency-level/latency tradeoff rather than silently weakening guarantees. |
| `SCALE-3` | **Make the accountability receipt replica-invariant + commit audit-chain head into it** — Ensure the same inputs produce a byte-identical pack+receipt regardless of which replica served the query (exported determinism_hash equal across nodes) and that the keyed audit-chain head (from the audit-MAC prerequisite) is committed into the receipt so the answer's provenance and ordering survive node loss. Externalize signed receipt roots so a third party can verify even after the originating node dies (CT-style witness/mirror, even before full HA). | `make receipt-replica-invariance-check asserts pack+receipt and determinism_hash are byte-identical across two replicas for the same query, and the receipt embeds and re-verifies the audit-chain head` | M | SCALE-1, SCALE-2, Pillar: Accountability receipt + real crypto, Pillar: MUST-FIX correctness prerequisites | Any per-node nondeterminism (node_id, timestamps, elapsed_nanos, map ordering) breaks replica-invariance; requires the same canonicalization discipline as the determinism axis — coordinate with the receipt pillar's canonical serializer. |
| `SCALE-4` | **Promote consensus gates from research-track into the release lane** — Wire consensus-partition-soak-check, consensus-failover-slo-check, and consensus-rejoin-check (currently only under distributed-consensus-research-check, mk/core-security-ops.mk:138-148) into release-check (mk/release.mk), alongside the new SCALE-1/SCALE-2/SCALE-3 cluster gates. Retire or rename the 'research' framing for these gates, update docs/STATUS.md / COMMUNITY_ROADMAP.md / SECURITY_MODEL.md to reflect that HA + cluster fail-closed is a release gate, not a research track, and gate this honestly (don't promote until SCALE-1..3 pass). | `make release-check runs consensus-partition-soak-check, consensus-failover-slo-check, consensus-rejoin-check, consensus-failover-binder-check, multi-agent-cluster-consistency-check, and receipt-replica-invariance-check; public-claims-check passes with the updated HA claims` | S | SCALE-1, SCALE-2, SCALE-3 | Promoting flaky distributed tests into the release lane could destabilize CI; require the partition/failover suites to be soak-stable (N consecutive green runs) before promotion, and keep an escape hatch to demote if flakiness reappears. |
| `SCALE-5` | **Surface retrieval-incompleteness (ANN visit-budget exceeded) as a first-class hashed pack anomaly** — Add a ContextPackAnomalyCode::RetrievalIncomplete (context/mod.rs:164-171) and plumb HNSW's budget_exceeded (search_impl.rs:75,119) — which is computed but discarded for the pack — into the ContextPack and json_export so an answer can disclose it may be incomplete. This must be inside the signed receipt so a sparse-scope agent's possibly-truncated context is accountable, not hidden. Pairs with the guarded-ANN sparse-scope fix from the prerequisites pillar. | `make context-pack-retrieval-incomplete-check asserts that when the ANN visit budget is exceeded for a sparse-scope agent, the pack carries a RetrievalIncomplete anomaly AND that anomaly is bound into the receipt (verifier sees it)` | M | Pillar: MUST-FIX correctness prerequisites, Pillar: Accountability receipt + real crypto | Surfacing incompleteness may regress headline recall optics; frame as honesty/accountability win and report it on the AAB completeness sub-metric rather than hiding it. |

**Измеримые критерии готовности столпа:**
- make aab-mini-check is green in CI and runs all six AAB-1 axes end-to-end on a checked-in 50-100 query fixture in under the standard alpha-check time budget
- make aab-baseline-matrix-report produces a head-to-head table for CortexDB + Zep/Graphiti + Mem0 + Cognee + pgvector+policy, and the report asserts the structural finding: all four rivals score 0 / UNRANKED on receipt-verifiability and determinism axes while posting comparable accuracy/tokens
- make aab-scope-leak-check asserts exactly 0 forbidden-scope cells across all adversarial queries AND across all export formats (extends context-pack-private-scope-check), at budgets B in {2k,4k,8k}
- make aab-determinism-check asserts pack+receipt are byte-identical across two independent processes AND across two different cluster replicas serving the same query
- make consensus-failover-binder-check (new) asserts a follower read, a read during a leader failover, and a read during a partition all preserve the fail-closed seed [PushAgentAllowed, PushLive, And] and return zero out-of-scope cells
- make multi-agent-cluster-consistency-check (new) asserts read-your-writes and monotonic-read hold across nodes at the declared MemoryConsistencyLevel through one leader failover and one partition heal
- release-check (mk/release.mk) includes the promoted consensus gates (partition-soak, failover-SLO, rejoin) and the new cluster fail-closed + AAB-mini gates; distributed-consensus-research-check is renamed/retired as 'research' and its gates run in the release lane
- make aab-receipt-verifiability-check confirms a standalone verifier binary (no cortex-engine link) validates 100% of fixture receipts emitted by the cluster, and a tamper case (flip one access decision / one cited byte / one budget number on a replica) is REJECTED
- make aab-tokens-to-answer-check reports tokens using one fixed reference tokenizer (cl100k/o200k) for the headline number, with the deterministic in-engine profile reported as a secondary metric, comparable across all rivals

**Зависимости:** Pillar: Accountability receipt + real crypto (this pillar's axes 5-6 and the receipt-determinism gates DEPEND on a frozen accountability_receipt.v1 schema, BLAKE3/SHA-256 Merkle commitment, Ed25519 signing, and a standalone verifier binary existing first; AAB cannot honestly score receipt-verifiability until FNV is replaced); Pillar: MUST-FIX correctness prerequisites (cosine dot.abs() fix, guarded-ANN sparse-scope recall + budget_exceeded surfacing, conflict unit/currency normalization, keyed audit MAC) — CortexDB must not self-report AAB numbers until these land, or the benchmark notarizes bugs; Existing replication stack (crates/cortex-engine/src/replication/*) and its research gates (distributed-consensus-research-check) — this pillar promotes and extends them rather than building consensus from scratch; A frozen public dataset/license plan for AAB-full corpora (LongMemEval/LoCoMo + ALCE/TREC-RAG + ConflictBank/MAGIC governance overlays) so the benchmark is redistributable

## Приложение B. Реконсилированная фазовая последовательность и критический путь

Ниже — два независимых прохода планировщика. **Авторитетным является второй (состязательный)**: он выносит «канонические байты + детерминизм» в отдельную Phase 0, ставит исправление багов перед подписью, и переопределяет настоящий ров (привязка `access_root` к исполненной алгебре плана + transparency log против эквивокации). Первый проход оставлен как исходная версия для сверки.

### B.1 Авторитетная последовательность (состязательный ревизор)

**Вердикт.** DIRECTIONALLY CORRECT, EXECUTION-FLAWED. The north star (a third-party-checkable accountability receipt) is the right non-absorbable bet and the codebase genuinely supports it: I verified the binder seed [PushAgentAllowed, PushLive, And] (binder.rs:137-145), the real divergence where the persisted ANN path derives `allowed` from readable_scopes ONLY (search/access.rs:9-17) and never composes PushLive/WHERE, the cosine dot.abs() sign bug (metric.rs:44), the unkeyed FNV audit chain, the feedback.rs 28-bit silent-truncation collision, and zero crypto deps in any Cargo.toml. So the diagnosis is sound. BUT: (1) the central absorption argument has a hole the roadmap admits then under-funds — DB equivocation is the ONE thing a signing wrapper CAN'T be stopped from doing either, so "non-absorbable" rests on the transparency log that is repeatedly demoted to "stub/optional/follow-up"; without it a pgvector+policy+sign wrapper replicates ~80% of the receipt. (2) Sequencing is inverted in one place that will cause rework: the canonical-serialization module (AR-1) and the determinism/elapsed_nanos exclusion are dependencies of FOUR pillars but are buried as a sub-task of the receipt pillar — they must be a standalone pillar-zero. (3) Two entire pillars (Absorption-proof live-baseline matrix, and the 4-competitor AAB benchmark) are research/marketing theater that consume XL/heavy effort and do not change whether the product is absorbable — they PROVE a claim rather than BUILD the moat. (4) Several "L" tasks (AR-6 captured access, AR-7 verifier, SCALE-1 cluster fail-closed) are XL. Fix the sequencing, fund the transparency log, cut the competitor theater, and this is a strong 2-3 quarter program.

| Фаза | Тег | Тема | Milestone-exit |
|---|---|---|---|
| **Phase 0 — Determinism + canonical-bytes foundation (extracted from receipt pillar)** | v0.2.0-beta.3 | Before any hashing/signing, make the hashable surface exist and be provably deterministic. Pull AR-1 (JCS canonical module), the elapsed_nanos/Instant::now exclusion from verification/operator.rs (confirmed at lines 28,37,46,60,83,93,102,155,166,188), and REPRO-1 canonical_bytes() into ONE standalone foundation that the receipt, determinism-hash, AAB-determinism, and replica-invariance pillars all consume. This is currently scattered as a sub-bullet of three different pillars and WILL be rebuilt twice if not centralized. | canonical_bytes() exists for ContextPack + VerificationReport with a property test proving invariance under map key-permutation and proving zero wall-clock bytes in the hashed surface; no crypto required yet (non-crypto namespaced content hash is fine here). |
| **Phase 1 — Correctness prerequisites (the evidence must be true before it is notarized)** | v0.2.0-beta.4 | Land the four confirmed bugs that would otherwise be cryptographically notarized lies. All verified in-file. Keep this lean: it is a blocker, not a destination. | cosine(v,-v)==0 and metric.rs==dedup.rs on a shared fixture; feedback/session no longer silently truncate (reject over-width); conflict detection routes through verification/numeric so $1.2M==1,200,000; budget_exceeded surfaces as a RetrievalIncomplete anomaly. All four behind gates in the release lane. |
| **Phase 2 — Real crypto + the signed receipt over honest inputs** | v0.3.0-beta.1 | Introduce the first real crypto (blake3 + ed25519-dalek), build the Merkle-tree receipt over the now-trustworthy evidence, and ship the STANDALONE offline verifier. This is the load-bearing pillar. Fold the captured-access-decision work (FC-5/AR-6) in here because access_root is worthless as a re-derivation. | A signed accountability_receipt.v1 attaches to packs/verify; cortex-receipt-verify (no engine link) accepts golden and rejects all 7 tamper classes; determinism gate proves byte-identical receipts incl. deterministic Ed25519 signatures across two processes. |
| **Phase 3 — End-to-end fail-closed parity (make access_root honest at the physical layer)** | v0.3.0-beta.2 | Close the verified divergence: persisted ANN/lexical `allowed` must equal the bound bitmap program (PushLive + WHERE), not readable_scopes alone. Fix sparse-scope recall collapse. This is what makes access_root a true commitment rather than a notarized re-derivation. Prove it with a property model. | ann-scope-parity-check green (persisted allowed == eval_bitmap_program(plan) ∩ vector_cells); sparse-scope recall within epsilon of exact; fail-closed-invariant-model emits a stable model_hash bound into the receipt. |
| **Phase 4 — Crypto foundation hardening (audit + at-rest become real)** | v0.3.0-beta.3 | Now that the crypto module and key custody exist, replace FNV audit chain with keyed HMAC/Ed25519 and XOR-FNV backup with XChaCha20-Poly1305+Argon2id, and bind the receipt hash into the audit record. Make the docs honest. This discharges two of the four accountability-undermining defects. | Audit chain is forgery-resistant (no MAC key => cannot verify); backup is AEAD with legacy-refuse; audit record commits the receipt hash; crypto-claims-honesty-check green. |
| **Phase 5 — Verification conflict-recall at strength + frozen learned ranker** | v0.4.0-beta.1 | Broaden conflict coverage (unit/currency/temporal/citation, multi-value) and MEASURE it, and freeze the ranker weights into an auditable artifact so the determinism_hash binds the exact ranker. These deepen two receipt commitments (conflict_commitment, determinism_hash) but are not blockers for the receipt's existence — hence later. | verify-conflict-recall-check: recall>=0.90, false-conflict<=0.05 on a labeled corpus; ranking weights loaded from a frozen Q16 artifact with a drift gate; determinism_hash changes iff weights change. |
| **Phase 6 (OPTIONAL / DE-RISKED) — Cluster fail-closed + transparency anchor** | v0.4.0 / v1.0.0-rc | Promote the already-built Raft stack (verified: full replication/ modules + ~26 test files) into the release lane and — critically — ship the append-only transparency log that closes the equivocation gap. The transparency anchor is the ONLY thing here that affects absorbability; the rest is production-readiness, valuable but separable. | consensus-failover-binder-check proves the binder seed survives follower-read/failover/partition; receipt-replica-invariance-check proves byte-identical receipts across replicas; a transparency log of pack_roots is live so two contradictory signed receipts are detectable. |

**Phase 0 — Determinism + canonical-bytes foundation (extracted from receipt pillar) — состав:**
- AR-1 canonical/JCS module
- REPRO-1 canonical_bytes()
- verification/operator.rs elapsed_nanos exclusion (DV6 determinism half)
- field-classification allowlist test (hashed vs non-hashed)

**Phase 1 — Correctness prerequisites (the evidence must be true before it is notarized) — состав:**
- CP-1 cosine fix
- CP-2 unify cosine impls
- CP-3 cell-id slot-width unification + reject
- CP-4 numeric-aware conflict detection
- CP-5 budget_exceeded -> ContextPack anomaly
- CP-6 aggregate gate

**Phase 2 — Real crypto + the signed receipt over honest inputs — состав:**
- AR-2 schema freeze
- AR-3 DB-computed cell_content_hash
- AR-4 Merkle trees + roots
- AR-5 Ed25519 sign + key custody
- AR-6/FC-5 captured access-decision with policy_version
- AR-7 standalone verifier
- AR-8 tamper suite + umbrella gate
- CRY-2 shared crypto module
- CRY-1 deps + policy gate

**Phase 3 — End-to-end fail-closed parity (make access_root honest at the physical layer) — состав:**
- FC-2 derive allowed from bitmap program
- FC-3 sparse-scope recall fix
- FC-6 scope-leak benchmark across all surfaces
- FC-7 property/proptest invariant model + model_hash
- FC-8 aggregate gate

**Phase 4 — Crypto foundation hardening (audit + at-rest become real) — состав:**
- CRY-3 AEAD backup
- CRY-4 keyed audit chain
- CRY-5 audit binds receipt hash
- CRY-6 key management/rotation
- CRY-7 honest docs
- CRY-8 aggregate

**Phase 5 — Verification conflict-recall at strength + frozen learned ranker — состав:**
- DV1-DV5 conflict normalization/temporal/citation/multi-value
- DV7 labeled recall benchmark
- RANK-1 frozen weights artifact
- RANK-2 trainer compiles to artifact + drift gate
- REPRO-2/REPRO-3 determinism_hash + cross-process gate
- RANK-4 explain faithfulness

**Phase 6 (OPTIONAL / DE-RISKED) — Cluster fail-closed + transparency anchor — состав:**
- SCALE-1 cluster fail-closed
- SCALE-3 replica-invariant receipt + audit head
- Transparency log (promoted from stub to first-class)
- SCALE-4 promote consensus gates to release lane

**Критический путь:**
1. Canonical-bytes + determinism foundation (AR-1 + REPRO-1 + elapsed_nanos exclusion) — gates everything hashable; currently mis-filed as a receipt sub-task
2. Correctness fixes (cosine, conflict-normalization, cell-id, budget disclosure) — a receipt over corrupt evidence is a notarized lie, so these strictly precede signing
3. Real crypto module (blake3 + ed25519-dalek) + key custody — the first crypto in the workspace; receipt, audit-MAC, and AEAD-backup all block on this single keystore/key_id surface
4. Merkle receipt + Ed25519 signing (AR-4, AR-5) over honest inputs
5. Standalone cortex-receipt-verify binary with enforced no-engine dependency (AR-7) — the artifact that makes the claim externally checkable
6. FC-2 ann-scope-parity (persisted allowed == bound bitmap program) + AR-6 captured access decision — without these, access_root attests a re-derivation, not enforcement, and the single genuine moat is unproven end-to-end
7. Transparency log / equivocation anchor — without it the non-absorbability claim is materially weaker; should be ON the critical path, not demoted to a stub

**Что параллелится:**
- Correctness fixes CP-1 (cosine), CP-3 (cell-id), CP-4 (conflict) are mutually independent and can land in parallel once Phase 0 lands
- DV2 numeric normalization (verification quality) is independent of the crypto track and can proceed alongside Phase 2
- CRY-3 AEAD backup is independent of the receipt path once CRY-2 lands — it shares only the crypto module, not the receipt
- RANK-1 frozen-weights extraction is a pure refactor independent of crypto; can start any time after Phase 1
- FC-6 scope-leak benchmark and FC-7 invariant model are test-only and parallel to the crypto track
- Schema-freeze work (AR-2, SPEC-1, SPEC-2) can proceed in parallel with crypto implementation since the schema is the contract both sides build to

**Переоценено / упущено:**
- MISSING — equivocation defense is the actual absorption decider and is underfunded. The roadmap repeatedly lists the transparency log as 'optional stub / follow-up / design-doc level' (AR-5, CRY-6, SCALE-3). But a pgvector+policy+ed25519 wrapper CAN sign a JCS-canonical bundle with Merkle roots; what it cannot easily do is prevent a malicious operator from issuing two contradictory-but-individually-valid receipts. That is the ONLY forgery class the signing wrapper also can't stop — so it is precisely where the moat lives, and it is the part the roadmap defers. Promote the append-only pack_root transparency log to a first-class, gated deliverable.
- MISSING — a precise statement of WHAT a wrapper structurally cannot replicate. The strongest non-absorbable claim is NOT 'we emit a signed receipt' (replicable) but 'access_root is bound to the actually-executed plan-algebra enforcement, provable by re-running the bound bitmap program against the cited cells.' That binding (FC-2 + AR-6 + FC-7 model_hash) is the real closure. The roadmap has the pieces but never names this as THE argument — it leans on 'reproducing the receipt requires re-implementing the whole engine,' which is rhetoric, not a proof. A wrapper doesn't need to reproduce the engine; it needs to emit a receipt that passes the same verifier. Make the verifier's access-check require re-evaluating the bound program, not just trusting an 'allowed' leaf.
- OVERSCOPED — the entire 'Absorption proof + open GCE specification' pillar's live-baseline matrix (BASE-1, BASE-2) and the AAB pillar's 4-competitor adapters (AAB-5, marked XL). Running live pgvector+OPA+Zep+Mem0+Cognee stacks is benchmark marketing that PROVES the moat exists; it does not BUILD it. It is heavy, flaky (4 third-party stacks, API keys, version drift — the roadmap admits this), and a reviewer will call it rigged regardless. Cut to: ship the open spec + standalone verifier + ONE documented thin-wrapper reference attempt that provably fails the equivocation/access-binding axes. That is sufficient to discharge the Oso test at ~10% of the effort.
- OVERSCOPED — the whole 'Accountability benchmark + production scale' pillar bundles two unrelated things: a competitor benchmark (cut, per above) and Raft promotion (keep, but separable). Promoting the already-built consensus stack is real value, but coupling it to a 6-axis 5-competitor leaderboard makes a clean infra task hostage to a marketing artifact.
- MISSING — key compromise / revocation story for a single-node beta. The roadmap notes 'a leaked node key forges all receipts' and moves on. For an accountability product this is existential: there is no key revocation, no receipt expiry, no re-anchoring protocol beyond a one-line 'dual-trust window.' At minimum specify receipt validity windows and a revocation list the verifier consults.
- MISSING — performance/latency budget for the receipt path. Merkle-folding every cell + Argon2id KDF + Ed25519 per query adds real latency, and the roadmap never sets a budget or gate. AR-4 is marked L but has no perf exit-criterion. A receipt that doubles p99 latency will be feature-flagged off and the moat evaporates in practice.

**Топ-риски:**
| Риск | Митигейт |
|---|---|
| The non-absorbability thesis is weaker than asserted: a thin pgvector + policy-engine + ed25519 wrapper CAN emit a JCS-canonical, Merkle-rooted, signed bundle with access/provenance/budget leaves. ~80% of accountability_receipt.v1 is replicable by a competent wrapper. The roadmap's 'requires re-implementing the whole engine' is rhetoric. | Make the verifier's access check structurally engine-bound: it must re-evaluate the SIGNED bitmap_program against the cited cells' scope/status, so a wrapper that merely asserts 'allowed' leaves fails unless it also implements the fail-closed plan algebra and binds the executed program. Pair with the transparency log so equivocation (the one class a wrapper also can't beat) is detectable. THESE TWO, not the signature, are the moat — fund them as P0. |
| Canonical-serialization and elapsed_nanos-exclusion are dependencies of 4+ pillars but are filed as sub-tasks (AR-1 inside receipt, REPRO-1 inside ranker, DV6 inside verification). Built independently, they will diverge — and two canonicalizers silently fork DB vs verifier, the exact failure the roadmap warns about for JCS. | Extract a Phase-0 'foundation' pillar owning ONE canonical_bytes() + ONE field-classification allowlist, consumed everywhere. Ship cross-language golden vectors before anyone hashes anything. |
| Effort mis-sizing hides 1-2 quarters of slip. AR-6 (thread captured access decision through binder->scan->pack across crates, touching the hot path) is L-labeled but is XL; AR-7 (standalone verifier re-implementing canonicalization independently AND a dependency-graph gate) is XL; SCALE-1 (cluster fail-closed surfacing AgentView/policy-version replication gaps the roadmap admits it may uncover) is XL; AAB-5 is correctly XL but should be cut. | Re-grade AR-6/AR-7/SCALE-1 to XL and split each: AR-6 into (a) carry compact decision token, (b) consume in pack; AR-7 into (a) verifier lib, (b) no-engine-dep gate. Budget for the AgentView-replication fix SCALE-1 will surface, don't just test for it. |
| Schema freeze before the leaf set is final (AR-2 depends only on AR-1) risks a forced v2 bump when FC-5 adds policy_version, CP-5 adds RetrievalIncomplete, and DV4/DV5 add temporal/citation conflict kinds — all of which must be IN the hashed leaves. | Do NOT freeze accountability_receipt.v1 until Phase 1 (correctness) and FC-5 (captured access w/ policy_version) land. Freeze the schema AFTER the leaf set is settled by code, reviewing context/mod.rs + verification/types.rs field-by-field as AR-2's own risk note says. |
| Determinism is asserted as achievable but the codebase has live wall-clock embedding (verified: Instant::now() at 9 sites in verification/operator.rs) and the determinism gate is currently a STATIC lint (bans HashMap tokens) over an ARCHIVED doc — i.e. the existing 'determinism' guarantee is weaker than the roadmap's current_state even admits. | Replace the lint-only engine-determinism-check with a real cross-process byte-identity harness (REPRO-3) early, and treat any hidden non-determinism it surfaces (float formatting, candidate-truncation tie-breaks) as a Phase-0 blocker, not a Phase-5 surprise. |

### B.2 Исходная последовательность (планировщик)

**Вердикт.** Sequence into four version-tagged phases gated by a single dominant critical path: trustworthy evidence + real crypto + canonical serialization must land BEFORE the receipt is built, the receipt before its independent verifier, the verifier before the head-to-head category proof, and only then promote to a production/cluster-attested contract. The North Star artifact (the signed accountability receipt) is buildable in v0.4 but is a notarized lie unless v0.3 first fixes the evidence-corrupting bugs (cosine dot.abs(), conflict normalization, cell-id collisions, ANN scope drift) and replaces FNV/XOR with real BLAKE3+Ed25519+AEAD+Argon2id. The decisive insight: a single shared canonical-bytes module is the agreement point that gates roots, determinism hash, AND the no-engine-link verifier — it must be specified once, early, and owned by one team. Most measurement/benchmark/correctness work parallelizes; the crypto→receipt→verifier→proof spine is strictly serial.

| Фаза | Тег | Тема | Milestone-exit |
|---|---|---|---|
| **v0.3 hardening** | v0.3 | Make the evidence trustworthy and lay the crypto+canonicalization foundation. Discharge every defect that would turn a signed receipt into a notarized lie (cosine sign bug, conflict false/missed recall, cell-id collisions, ANN scope drift, hidden incompleteness, wall-clock non-determinism), introduce the workspace's first real cryptographic primitives, and pin the single canonical-bytes wire format that DB and verifier will both agree on. | correctness-prerequisites-check + crypto-foundation-check + canonical-serialization-check are green in the release lane: every evidence signal the receipt will bind (cosine ranking, conflict set, cell identity, access decision, completeness flag) is correct/captured/disclosed; zero FNV/XOR backs any integrity surface; and canonical_bytes() is byte-stable under key-permutation with all wall-clock fields provably excluded. |
| **v0.4 accountability-receipt** | v0.4 | Build, sign, and independently verify the category-defining artifact. Assemble the five Merkle trees + pack_root + determinism_hash over the now-trustworthy evidence, Ed25519-sign only the fixed-size header, and ship a standalone offline verifier that links NONE of the engine and rejects every tampering class. | accountability-receipt-check (schema-freeze + determinism + tamper + independent-verifier sub-gates) is green and wired into alpha-check: two runs on a fixed store yield byte-identical pack+receipt incl. signatures, the no-engine-link cortex-receipt-verify accepts every genuine fixture and rejects every mutation, and at-rest/audit integrity now rests on real AEAD/MAC bound into the receipt. |
| **v0.5 category-proof** | v0.5 | Convert the receipt from an internal property into a public, externally-checkable category contract that passes the Oso feature-or-product test. Publish the open GCE/receipt spec, prove the fail-closed invariant as a machine-checked attested property, and run a live head-to-head benchmark where a pgvector+policy library structurally scores UNRANKED on receipt-verifiability and determinism. | accountability-check (spec-freeze + standalone-verify + tamper + cross-process determinism + conformance) is green in the release lane, AND aab-baseline-matrix-report reproducibly shows CortexDB ranked on all six axes while a live pgvector+policy baseline is UNRANKED on receipt-verifiability and determinism — the absorption-resistance closure made empirical and published, not asserted. |
| **v1.0 production-accountability** | v1.0 | Make the accountability contract hold under production failure modes and at cluster scale, so the receipt is honestly emitted by any replica — not just a single beta node. Promote the existing Raft stack from research-track and prove the fail-closed binder invariant survives follower reads, leader failover, and partition. | release-check includes consensus-failover-binder-check + multi-agent-cluster-consistency-check + receipt-replica-invariance-check and they are green: an independent third party can verify any answer from any replica without trusting the DB, the fail-closed guarantee provably holds through failover/partition, and HA is an honest release gate rather than a research track. |

**v0.3 hardening — состав:**
- CP-1/FC-1 fix cosine dot.abs() + overflow (reuse proven dedup.rs signed-clamp; reject dot<=0; i128/u128 widening)
- CP-2 unify the two cosine impls onto one source of truth so they cannot re-diverge
- CP-3/cell-id: unify memory/session/feedback on one documented slot width (31-bit), reject-not-truncate over-width ids, behind schema-version/migration guard
- CP-4/DV1+DV2 wire EXISTING verification/numeric normalization into context/conflicts.rs (kills $1.2M vs 1,200,000 false 3-way; integer-only) + add unit/currency class conversion
- DV3 multi-value extraction; DV4 stop temporal facts skipping numeric conflicts + same-date contradiction; DV5 citation conflicts
- DV6 exclude elapsed_nanos/Instant::now from the hashable verification surface (precondition for any determinism claim)
- FC-2 derive persisted ANN/lexical allowed-set from the bound bitmap program (parity with binder), FC-3 sparse-scope exact-fallback recall, FC-4/CP-5 surface ANN budget_exceeded as a first-class RetrievalIncomplete ContextPack anomaly
- FC-5 capture real access-decision enforcement (policy_version + AgentView digest) instead of pack-time re-derivation
- CRY-1/AR-1 add audited RustCrypto deps (blake3/sha2, ed25519-dalek, chacha20poly1305, argon2, getrandom, zeroize, subtle) behind a feature flag + dependency-policy grep gate
- CRY-2 single shared cortex-crypto primitives module (hash/AEAD/KDF/MAC/sign) with pinned KAT vectors
- AR-1/REPRO-1 canonical-bytes (JCS/RFC-8785 or explicit integer-only byte form, recursive key sort, no floats/timestamps, domain tags) for ContextPack + VerificationReport — the normative wire format
- AR-3 DB-computed cell_content_hash = blake3(canonical cell bytes), not the self-asserted payload string
- RANK-1 extract all ranking magic-constants into a frozen Q16 weights artifact + generated module (pure refactor, behavior-identical)

**v0.4 accountability-receipt — состав:**
- AR-2 freeze accountability_receipt.v1 JSON Schema as ONE additive optional field on context_pack.v1 + golden fixture + spec doc (additive-until-v2 honored)
- REPRO-2 determinism_hash binding (query, AgentView projection, options, frozen_weights_version) using the real CRY-2 hash
- AR-4 receipt body: access/provenance/cell_set/verification roots + budget/conflict commitments via ordered Merkle folds; pack_root binds output, determinism_hash binds input; promote grounding report + budget_exceeded into the hashed surface
- AR-5/CRY-6 Ed25519-sign the header with RFC-8032 deterministic nonces; node keystore, key_id, rotation/dual-trust, public-key export
- REPRO-3 byte-identical determinism harness (two runs + checkpoint) wired into engine-determinism-check; replace stale archived-doc token check
- AR-7/VERIF-1 standalone cortex-receipt-verify binary (own crate, no cortex-engine/storage/aql link, dependency-graph-asserted) implementing the 7-step verifier from public inputs only
- AR-8/VERIF-2 table-driven tamper suite (flip estimated_tokens, flip access decision, shift byte_start, drop VisibleConflict, swap verdict, replay under different query/AgentView, flip signature byte) — 100% rejection, mutation-of-mutation guard
- CRY-3 replace backup XOR-FNV with XChaCha20-Poly1305 + Argon2id (v2 format) + refuse-legacy; CRY-4 keyed audit chain (SHA-256 + HMAC/Ed25519, shared writer/verifier); CRY-5 commit per-answer receipt hash into the audit record
- CRY-7 honest at-rest/crypto docs + doc-lint gate; RANK-2/RANK-3 compile offline LTR profiles into the frozen artifact + drift gate + engine-side lift proof; REPRO-... weights-version-binding (hash changes iff weights change)
- DV7 labeled contradiction-recall benchmark (>=150 cases) feeding verification_root coverage; RANK-4 explain-faithfulness (score == sum of components)

**v0.5 category-proof — состав:**
- SPEC-1 open GCE_CONTRACT.md (ContextPack result type + six invariants + conformance obligations); SPEC-3 verifier algorithm + threat model doc (each forgery class mapped to a defending field)
- FC-7 machine-checked fail-closed formal-invariant model (proptest over both bitmap-program and persisted ANN paths) with exported model_hash bound into the receipt's access attestation
- FC-6 scope-leak benchmark across EVERY output surface (>=200 agent x query x format x persistence x budget combos, pre/post checkpoint+compact, 0 sentinel bytes); FC-8 fail-closed-end-to-end aggregate gate
- AAB-1/AAB-2/AAB-3 freeze AAB-1 six-axis spec + governance-overlay fixtures + harness for the four CortexDB-side axes (scope-leak, citation P/R via NLI, contradiction recall + false-conflict, tokens-to-answer at fixed reference tokenizer)
- AAB-4 wire receipt-verifiability + determinism axes (gating: <100% verifiability or non-byte-identical => UNRANKED)
- BASE-1/AAB-5 live pgvector + OPA/Cedar + RAG-library baseline harness (+ Zep/Mem0/Cognee adapters, nightly); BASE-2/AAB-6 head-to-head matrix proving rivals score 0/UNRANKED on axes 5-6
- CONF-1 public conformance + adversarial suite (scope-widening, fabricated-citation, dropped-conflict, forged-audit, anti-correlation) with a thin-wrapper reference that provably fails >=3 axes
- DV8 publish measured VERIFY recall/false-conflict numbers; WIRE-1 aggregate accountability-check into release + nightly lanes

**v1.0 production-accountability — состав:**
- SCALE-1 cluster_fail_closed: prove [PushAgentAllowed, PushLive, And] preserved and 0 out-of-scope cells on follower reads, mid-failover, and during partition (may surface AgentView/policy-version replication gaps to fix)
- SCALE-2 cross-node read-your-writes + monotonic-read at the declared MemoryConsistencyLevel through failover and partition heal
- SCALE-3 replica-invariant receipt: byte-identical pack+receipt+determinism_hash regardless of serving replica; commit keyed audit-chain head into the receipt; externalize signed roots to a CT-style witness/mirror so history survives node loss
- SCALE-5 RetrievalIncomplete anomaly bound into the receipt at cluster scale (sparse-scope honesty)
- SCALE-4 promote consensus-partition-soak/failover-slo/rejoin gates from distributed-consensus-research-check into release-check (require N consecutive soak-green before promotion); update STATUS/SECURITY/ROADMAP docs honestly
- AAB-full + signed leaderboard pack (aab-leaderboard-pack) for public submission; optional transparency-log anchor for equivocation mitigation
- Receipt feature flag flipped on by default; key-management/rotation ops flow hardened

**Критический путь:**
1. v0.3: CP-1/FC-1 fix cosine dot.abs() (+CP-2 unify) — unblocked, gates trustworthy ranking the receipt attests
2. v0.3: CP-4/DV1+DV2 wire numeric normalization into conflict detection + DV6 exclude elapsed_nanos — gates a sound, deterministic conflict/verdict surface
3. v0.3: FC-2 ANN scope-parity + FC-5 capture real access-decision enforcement — gates an honest access_root (cannot be a pack-time re-derivation)
4. v0.3: CRY-1/AR-1 add real crypto deps + CRY-2 shared crypto module — the workspace's first hash/signature primitive; gates every signed/MAC'd surface
5. v0.3: AR-1/REPRO-1 canonical-bytes module (the single DB<->verifier agreement point) + AR-3 DB-computed cell_content_hash — gates all Merkle leaves/roots
6. v0.4: AR-4 build five Merkle trees + pack_root + REPRO-2 determinism_hash — the receipt body, consuming all v0.3 evidence
7. v0.4: AR-5/CRY-6 Ed25519-sign the header with deterministic nonces + key custody — makes the receipt third-party-checkable
8. v0.4: AR-7/VERIF-1 standalone no-engine-link cortex-receipt-verify + AR-8/VERIF-2 tamper suite — proves verifiability WITHOUT trusting the DB
9. v0.5: FC-7 attested fail-closed model_hash + BASE-1/BASE-2 live baseline head-to-head — proves the category is non-absorbable empirically
10. v1.0: SCALE-1 cluster fail-closed + SCALE-3 replica-invariant receipt — extends the proven contract to production failure modes

**Что параллелится:**
- CP-1 cosine fix and CP-3 cell-id unification are independent of each other and of the crypto track — both can start day one (CP-1 has zero deps; reference impl already in dedup.rs)
- The entire MUST-FIX correctness track (CP/FC/DV bug fixes, conflict normalization, ANN recall, scope-leak benchmark) runs IN PARALLEL with the crypto-foundation track (CRY-1..CRY-4) — they share no code; only the receipt assembly (AR-4) needs both
- Canonical-bytes (AR-1/REPRO-1) can be specified and golden-vectored in parallel with bug fixes, since it depends only on the field set, not on crypto — but it MUST land before AR-4
- RANK-1 ranking-constant extraction (pure refactor) and the whole learned-ranker pipeline (RANK-2/RANK-3) are independent of the receipt spine and can proceed any time after RANK-1
- DV7 contradiction-recall benchmark and FC-6 scope-leak benchmark are measurement work parallel to receipt build, gated only by their respective bug fixes landing
- AAB competitor adapters (BASE-1, AAB-5 Zep/Mem0/Cognee) are heavy external-stack work that can be built in nightly lanes throughout v0.4 while the receipt is being finished — they only block the v0.5 matrix report
- SCALE-1/SCALE-2 cluster fail-closed and consistency tests can be developed against the existing replication harness in parallel with v0.4/v0.5, but cannot pass honestly until the receipt exists (SCALE-3 depends on the receipt + canonical serializer)
- CRY-7/DV8/SPEC-1 doc-honesty work parallelizes with implementation as long as doc-lint gates cross-check measured numbers

**Переоценено / упущено:**
- OVERSCOPED for v1.0: full per-scope ANN subgraph partitioning (index-build change) is correctly deferred in FC-3 to a follow-up; do not let it creep into the recall fix — the exact-fallback-for-small-allowed-set is the cheap high-value half and is sufficient for the receipt's completeness honesty.
- OVERSCOPED: the AAB competitor matrix (AAB-5, four live third-party stacks with API keys/version drift) is XL and flaky; keep it strictly nightly with snapshotted rival outputs, NOT in the fast release lane. The category proof needs only ONE credible live baseline (pgvector+OPA/Cedar) to make the structural point; Zep/Mem0/Cognee are corroboration, not gating.
- UNDERSCOPED RISK: the canonical-bytes spec (AR-1) is treated as one task but is the single highest-leverage agreement point — a wrong/ambiguous canonicalization silently forks DB vs verifier. It deserves cross-language golden vectors and a frozen normative doc up front, weighted heavier than its 'M' effort tag implies.
- MISSING explicit task: a migration/refuse-to-read gate is mentioned per-item (backup v2, audit chain v2, cell-id encoding) but there is no single 'format-version-bump audit' ensuring all three on-disk breaks ship coherently in v0.3/v0.4 without silently breaking the same-inputs=>byte-identical invariant across versions. Add one consolidating migration gate.
- MISSING: equivocation (DB signs two contradictory-but-individually-valid receipts) is correctly documented as out-of-scope for the receipt itself, but the transparency-log/witness anchor that mitigates it is only a 'stub/design-doc' until v1.0 SCALE-3. For a single-node beta this is acceptable, but the v0.5 'category proof' should explicitly state this caveat so the absorption-resistance claim is not overstated.
- SLIGHT REDUNDANCY: canonical_bytes() and determinism_hash appear in BOTH the receipt pillar (AR-1/AR-4) and the reproducibility pillar (REPRO-1/REPRO-2). Assign single ownership (reproducibility pillar OWNS canonical_bytes; receipt pillar CONSUMES it) to avoid two divergent hashing schemes — the roadmap notes this but it must be enforced as a hard coordination point, not a suggestion.

**Топ-риски:**
| Риск | Митигейт |
|---|---|
| A signed receipt is built on still-corrupt evidence (cosine, conflict recall, access drift, hidden incompleteness), converting a bug into a cryptographically-notarized lie — the single worst failure mode of the whole program. | Hard-gate: correctness-prerequisites-check (CP-1..CP-5, DV1..DV6, FC-2/FC-5) MUST be green and in the release lane BEFORE AR-4 receipt assembly is allowed to merge. Treat every MUST-FIX as blocking for receipt emission, not parallel-to-ship. |
| Canonicalization ambiguity (unicode/number/key-order) silently forks the DB's bytes from the independent verifier's, so genuine receipts fail or forged ones pass — defeating the entire third-party-verifiability thesis. | Pin RFC-8785 JCS or an explicit integer-only byte form in a frozen normative doc with cross-language golden vectors; the standalone verifier re-derives canonicalization from the published spec, never imports the engine module (dependency-graph-asserted in AR-7). |
| Hidden non-determinism (HashMap iteration, float formatting, locale, residual wall-clock) leaks into a hashed root, making same-inputs=>byte-identical fail flakily and breaking signature reproducibility. | Route every hashed value through the one canonical module sourced only from BTreeMap/BTreeSet; field-exclusion test greps the hashed surface for elapsed_nanos/SystemTime; run the determinism harness 3x cross-process in CI; keep the HashMap/HashSet lint as a guard. |
| Adding the workspace's first crypto deps (getrandom OS entropy, RustCrypto crates) stalls on dependency-allowlist/vendoring policy or CI-sandbox entropy, blocking the entire crypto→receipt→verifier spine. | Front-load CRY-1 as the very first crypto task; confirm allowlist + vendoring + CI entropy availability before any downstream crypto work; gate the feature behind a cargo flag for the first release so non-crypto builds are unaffected. |
| On-disk format breaks (backup v2, keyed audit chain v2, cell-id encoding) ship incoherently and silently violate cross-version byte-identity, or leave legacy XOR-FNV/FNV artifacts silently trusted. | Every format change ships behind a schema-version bump with refuse-to-read or explicit migration; add one consolidating migration-compatibility gate; legacy XOR-FNV backup and FNV audit chain are REFUSED on read with a typed error, never silently decoded. |
| The AAB head-to-head is dismissed as a rigged comparison, undermining the public category-proof. | Make axes 1-4 reuse established methodology (ALCE/TREC citation, ConflictBank conflicts, LongMemEval/LoCoMo substrate) so the baseline genuinely wins or ties those; leave axes 5-6 (receipt-verifiability, determinism) as the sole STRUCTURAL differentiator; publish the harness + standalone verifier so anyone can reproduce. |
| Promoting flaky distributed/consensus tests into the release lane destabilizes CI and blocks shipping. | Require N consecutive soak-green runs before promoting partition-soak/failover-SLO gates; keep a demote escape hatch; run the heavy cluster + AAB-full suites nightly, with CI-safe mini subsets in the fast lane (mirrors the existing balanced_50 / continuous-benchmark split). |
| Single-node key custody is a footgun: a leaked node key forges all receipts; a lost MAC/sign key breaks audit verification — and HA is deferred to v1.0. | Document key rotation (key_id bump + dual-trust window + re-anchor) and recovery in v0.4; specify the optional append-only transparency anchor; explicitly state the equivocation caveat in the v0.5 category-proof so the claim is 'internally consistent and third-party-checkable', not overstated as Byzantine-proof. |

## Приложение C. Реестр гейтов (`make <name>`)

Каждая фаза измерима новым или существующим гейтом. Сводный список упомянутых в плане гейтов (в стиле текущей gate-культуры проекта — отчёты пишутся в `target/<gate>/report.json`):

- `make aab-baseline-stack-check`
- `make aab-citation-attribution-check`
- `make aab-conformance-check`
- `make aab-contradiction-recall-check`
- `make aab-determinism-check`
- `make aab-mini-check`
- `make aab-mini-fixture-check`
- `make aab-receipt-verifiability-check`
- `make aab-scope-leak-check`
- `make aab-spec-check`
- `make aab-tokens-to-answer-check`
- `make access-check`
- `make accountability-access-capture-check`
- `make accountability-canonical-check`
- `make accountability-cell-hash-check`
- `make accountability-check`
- `make accountability-receipt-check`
- `make accountability-receipt-determinism-check`
- `make accountability-receipt-schema-check`
- `make accountability-receipt-sign-check`
- `make accountability-receipt-spec-check`
- `make accountability-receipt-tamper-check`
- `make accountability-receipt-verify-check`
- `make achine-check`
- `make alpha-check`
- `make ann-budget-disclosure-check`
- `make ann-metric-matrix-check`
- `make ann-scope-parity-check`
- `make ann-sparse-scope-recall-check`
- `make audit-chain-check`
- `make audit-receipt-binding-check`
- `make auth-rotation-check`
- `make baseline-comparison-check`
- `make canonical-serialization-check`
- `make cell-id-collision-check`
- `make changed-after-check`
- `make conflict-normalization-check`
- `make consensus-failover-binder-check`
- `make consensus-failover-slo-check`
- `make consensus-partition-soak-check`
- `make consensus-rejoin-check`
- `make context-access-decision-capture-check`
- `make context-pack-conflict-visibility-check`
- `make context-pack-private-scope-check`
- `make context-pack-retrieval-incomplete-check`
- `make context-pack-schema-contract-check`
- `make context-verify-quality-check`
- `make correctness-prerequisites-check`
- `make cosine-metric-correctness-check`
- `make cross-check`
- `make crypto-claims-honesty-check`
- `make crypto-deps-policy-check`
- `make crypto-foundation-check`
- `make crypto-primitives-check`
- `make determinism-check`
- `make distributed-consensus-check`
- `make distributed-consensus-research-check`
- `make docs-claims-check`
- `make encrypted-backup-check`
- `make encrypted-backup-legacy-refuse-check`
- `make encrypted-backup-rotation-check`
- `make engine-determinism-check`
- `make externally-check`
- `make fail-closed-end-to-end-check`
- `make fail-closed-invariant-model-check`
- `make gce-spec-doc-check`
- `make hnsw-cosine-correctness-check`
- `make key-management-check`
- `make learned-ranking-calibration-check`
- `make machine-check`
- `make migration-compatibility-check`
- `make multi-agent-cluster-consistency-check`
- `make pack-completeness-signal-check`
- `make pack-determinism-hash-check`
- `make per-check`
- `make post-check`
- `make public-claims-check`
- `make ranking-explain-faithfulness-check`
- `make ranking-frozen-weights-check`
- `make ranking-learned-lift-check`
- `make ranking-weights-drift-check`
- `make re-check`
- `make receipt-replica-invariance-check`
- `make receipt-threat-model-check`
- `make release-check`
- `make replication-lifecycle-check`
- `make replication-partition-check`
- `make schema-check`
- `make scope-leak-bench-check`
- `make secrets-check`
- `make security-gate-v2-check`
- `make security-release-report-check`
- `make tamper-check`
- `make third-party-check`
- `make verification-quality-check`
- `make verify-check`
- `make verify-citation-conflict-check`
- `make verify-conflict-recall-check`
- `make verify-determinism-check`
- `make verify-multivalue-extraction-check`
- `make verify-numeric-normalization-check`
- `make verify-temporal-conflict-check`
- `make weights-version-binding-check`
