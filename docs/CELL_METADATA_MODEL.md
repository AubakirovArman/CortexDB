# CortexDB Cell Payload Metadata Model (v1.0.0-stable)

CortexDB uses the cell payload's text lines as the single, authoritative **Source of Truth (SoT)** for query metadata. This document formalizes the payload metadata serialization standard.

---

## 1. Metadata Schema Format
Each knowledge cell payload consists of a structured headers section followed by double newlines (`\n\n`) and the raw body text content.

```text
scope=<scope_id>
status=<ready|stale>
type=<fact|document_block|memory>
source=<provenance_source_id>
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
| **`project`** | No | Associated entity name (used in numeric conflict extraction). |
| **`metric`** | No | Associated numeric metric (e.g., `budget`, `revenue`). |
| **`value`** | No | Standardized raw numeric value. |
| **`currency`** | No | Currency or unit of measurement (e.g. `KZT`, `USD`, `%`). |

---

## 3. Serialization Rules

1. **Header Order:** Key-value lines are parsed line-by-line sequentially from the top of the payload.
2. **Body Boundary:** The metadata section ends immediately on the first double newlines (`\n\n`), after which any text is treated strictly as the cell body text.
3. **White Space:** Values are trimmed of outer whitespaces upon parsing.
