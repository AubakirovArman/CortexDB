# CortexDB Cell Payload Metadata Model (v0.1.0-core-alpha)

CortexDB uses the cell payload's text lines as the single, authoritative **Source of Truth (SoT)** for query metadata. This document formalizes the payload metadata serialization standard.

For the broader knowledge cell, MVCC, lifecycle, scope, provenance, and target
typed descriptor contract, see [`DATA_MODEL.md`](DATA_MODEL.md). This document
only defines the current Core Alpha payload-header compatibility format.

---

## 1. Metadata Schema Format
Each knowledge cell payload consists of a structured headers section followed by double newlines (`\n\n`) and the raw body text content.

```text
scope=<scope_id>
status=<ready|stale>
type=<fact|document_block|memory>
source=<provenance_source_id>
source_id=<structured_source_ref_id>
source_url=<source_url>
doc_id=<document_id>
page=<page_number>
row=<row_number>
chunk_id=<chunk_or_cell_range>
json_path=<json_path>
confidence_q16=<0..65535>
[project=<entity_project_id>]
[metric=<numeric_metric_type>]
[value=<numeric_value>]
[currency=<currency_unit>]

<Raw body text content starts here...>
```

---

## 2. Key-Value Headers

| Header | Required | Purpose |
| --- | --- | --- |
| **`scope`** | Yes | Scope isolation identifier (e.g. `project:legal`). |
| **`status`** | Yes | Cell visibility status (`ready` or `stale`). |
| **`type`** | Yes | Cell classification: `fact`, `document_block`, `memory`. |
| **`source`** | No | Raw source file or citation provenance. |
| **`citation`** | No | Explicit citation string. |
| **`source_id`** | No | Structured SourceRef id. If present, it also satisfies citation requirements. |
| **`source_url` / `url`** | No | Optional URL for the structured SourceRef. |
| **`document_id` / `doc_id`** | No | Optional document id for the structured SourceRef. |
| **`cell_range` / `chunk_id`** | No | Optional range or chunk id for the structured SourceRef. |
| **`page`** | No | Optional page number for the structured SourceRef. |
| **`row` / `row_number`** | No | Optional table/CSV row number for the structured SourceRef. |
| **`json_path`** | No | Optional JSON path for structured JSON/API provenance. |
| **`confidence_q16`** | No | Fixed-point SourceRef confidence. Used by AQL `REQUIRE confidence >= ...`. |
| **`project`** | No | Associated entity name (used in numeric conflict extraction). |
| **`metric`** | No | Associated numeric metric (e.g., `budget`, `revenue`). |
| **`value`** | No | Standardized raw numeric value. |
| **`currency`** | No | Currency or unit of measurement (e.g. `KZT`, `USD`, `%`). |

Plain-text ingestion writes stable chunk provenance into the header:

```text
document_id=<source>
chunk_id=<sanitized-source>#chunk-0001
```

JSON ingestion writes sorted leaf `json_path=<flattened.path>` values for every
emitted fact. CSV ingestion writes `row=<1-based source row>` and
`cell_range=row-<n>` for every data row. PDF text ingestion writes `page=<n>`
when the caller provides a page.

The chunk id is deterministic for the same document id, text, and
`TextChunkPolicy`. It is independent of `CellId`, so ContextPack citations stay
stable across restarts and reimports that preserve the same chunk policy.
The full deterministic ingestion contract is in
[`DETERMINISTIC_CHUNKING.md`](archive/DETERMINISTIC_CHUNKING.md).

---

## 3. Serialization Rules

1. **Header Order:** Key-value lines are parsed line-by-line sequentially from the top of the payload.
2. **Body Boundary:** The metadata section ends immediately on the first double newlines (`\n\n`), after which any text is treated strictly as the cell body text.
3. **White Space:** Values are trimmed of outer whitespaces upon parsing.

## 4. Numeric Value and Unit Rules

`value` stores the normalized numeric value for deterministic verification. It
does not carry implicit magnitude.

Examples:

```text
metric=budget
value=1200000000
currency=KZT
```

This is interpreted as `1.2B KZT` for display because the stored value is the
full integer amount.

```text
metric=risk
value=1.2
currency=%
```

This is interpreted as `1.2 %`. CortexDB must not infer `B`, `M`, or any other
magnitude from a decimal metadata value. If a fact says `1.2B KZT`, the `B`
scale must be explicit in the fact text or represented by a full normalized
integer value such as `1200000000`.
