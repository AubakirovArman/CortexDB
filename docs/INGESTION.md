# Ingestion v1

Core Alpha now has a minimal structured cell API:

- `KnowledgeCellType`
- `KnowledgeCellMetadata`
- `KnowledgeCell`
- `Database::put_knowledge_cell`
- `Database::remember_aql`

`KnowledgeCell::encode_payload` serializes metadata into the existing
payload-line convention:

```text
scope=project:investments
status=verified
type=fact
source=annual-report
source_trust_q16=60000

body bytes...
```

New `Database::put_knowledge_cell` writes also include a structured
`CellMetadata` WAL section. The section is deterministic UTF-8 metadata with a
`cortexdb.cell_metadata.v1` prefix. The payload header remains present so the
current MemTable, segment, bitmap, lexical, AQL, and search paths stay
compatible while storage gradually moves away from payload-line parsing.

`CellMetadata::from_payload` is the single engine parser for this convention.
It reads metadata only from the leading header and keeps the post-header body
separate for lexical terms, search, and redundancy checks.

`REMEMBER` execution is intentionally a thin bridge. The AQL parser and binder
enforce `AgentView.allow_remember`, write-scope access, allowed `MemoryType`, and
TTL policy. The engine then stores the memory as a `KnowledgeCellType::Memory`
payload with `memory_type`, optional `ttl_seconds`, and `source=agent:<id>`.

Minimal adapters now exist on `Database`:

- `ingest_text`: one document block from plain text.
- `ingest_json`: flat JSON object fields into fact cells.
- `ingest_csv`: header row plus one document block per data row.
- `ingest_pdf_text`: external PDF extraction hook that stores extracted text
  with `source_format=pdf` and optional page metadata.
- `ingest_pdf_bytes`: native no-dependency extractor for simple uncompressed
  PDF text objects and `/FlateDecode` zlib streams before storing the same
  `source_format=pdf` cell.

The native PDF extractor handles literal strings and hex strings inside
`BT ... ET` text objects, including simple compressed Flate streams. It
intentionally rejects unsupported/empty PDFs instead of silently storing an
empty document.

`IngestionProgressTracker` provides a small synchronous progress surface for
adapter jobs. The first tracked helper is `Database::ingest_csv_with_progress`,
which records total rows, completed cells, status, and the last written
`CellId`.

## Not Yet

- PDF layout reconstruction and object graph repair.
- Enrichment jobs.
- TTL expiry/decay scanning.
