# Deterministic Chunking v1

CortexDB ingestion must produce stable provenance for the same input and
policy. This document freezes the Core Alpha chunking contract used by text,
JSON, and table-style ingestion.

## Text

Text ingestion uses `TextChunkPolicy`:

- `max_chars`: maximum Unicode scalar count per emitted chunk.
- `overlap_chars`: fixed character overlap for chunks created from a single
  long paragraph.
- `min_chars`: minimum trimmed size required to emit a chunk.

Chunk IDs are stable and independent of `CellId`:

```text
<sanitized-document-id>#chunk-0001
<sanitized-document-id>#chunk-0002
```

The overlap policy is `TextOverlapPolicy::FixedChars`. Paragraphs are packed in
order when they fit within `max_chars`; long paragraphs are split with the fixed
overlap.

## JSON

JSON ingestion uses `JsonChunkPolicy`:

- leaf values are emitted as fact cells;
- nested object paths use `.` as the separator;
- array positions use numeric path components;
- paths are sorted before cells are written.

For example:

```json
{"z":1,"a":{"b":2},"arr":[{"x":3}]}
```

emits deterministic `json_path` values:

```text
a.b
arr.0.x
z
```

## Tables

CSV/table ingestion uses `TableChunkPolicy`:

- the first source row is treated as the header row;
- data rows use 1-based source row numbers;
- the first data row is row `2`;
- `cell_range` is `row-<source-row>`.

For example, the first two data rows produce:

```text
row=2
cell_range=row-2

row=3
cell_range=row-3
```

## Compatibility

Changing these policies changes public provenance handles. Any future change
must either preserve old IDs/paths/ranges or be shipped as an explicit migration
with compatibility tests.
