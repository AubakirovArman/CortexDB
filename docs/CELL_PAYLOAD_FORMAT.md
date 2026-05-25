# CortexDB Cell Payload Format

CortexDB uses an **unstructured body text + structured metadata headers** design for cell payloads. This enables deterministic parsing of metadata (like scopes, trust levels, and metric key-value pairs) directly within the database engine and the AQL/ContextPack processors, without relying on heavy or slow external LLM parsing.

---

## 1. Structure of a Cell Payload

A cell payload consists of two distinct zones, separated by an empty line:

1. **Header Lines (Metadata):** Single-line key-value pairs at the beginning of the payload.
2. **Body Text (Content):** The raw text content of the cell, starting after the first empty line following the header lines.

### Example:
```text
scope=project:investments
status=ready
type=fact
source=report_q1.pdf#page=3
project=Solar Plant
metric=budget
value=1200000000
currency=KZT

Solar Plant Q1 report highlights. The total approved budget for the Solar Plant project in first quarter is 1.2B KZT.
```

---

## 2. Core Metadata Fields (Reserved Keys)

The database engine recognizes the following standard keys in the header zone:

| Key | Type | Description |
| --- | --- | --- |
| `scope` | `String` | Namespace/scope identifier for security and visibility checks. |
| `status` | `String` | Cell visibility state (e.g. `ready`, `draft`, `retired`). |
| `type` | `String` | Classification type (e.g. `fact`, `document_block`, `memory`). |
| `source` / `citation` | `String` | Citation/provenance reference of the cell content. |
| `memory_type` | `String` | Used for memory decay/TTL rules (e.g. `decision`, `observation`). |
| `project` | `String` | Associated project identifier (used in deterministic fact verification). |
| `metric` | `String` | Associated metric name (used for numeric conflict detection). |
| `value` | `Decimal/Integer` | Metric value representation. |
| `currency` | `String` | Currency denomination for values (e.g., `KZT`, `USD`). |

---

## 3. Parsing and Rules

- **Header boundary:** The header parsing stops at the first empty line, or at the first line that does not contain a `=` character.
- **Spaces:** Leading and trailing spaces around keys and values are automatically trimmed during parsing.
- **Deterministic Verification:** When running `VERIFY FACT` queries, CortexDB scans cells in the matched scopes, parses `project`, `metric`, `value`, and `currency` tags, and compares them with the target fact's parsed metrics to detect contradictions automatically.
