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
`CellId`. Persisted job files are written atomically with file sync plus rename,
so a restart should see either the old complete job state or the new complete
job state, not a partially written JSON file.

## Ingestion Job Lifecycle

The HTTP API exposes persisted ingestion job records so clients and the
dashboard can inspect the result of ingestion work after the immediate request:

- list jobs with `GET /v1/ingest/jobs`;
- read one job with `GET /v1/ingest/jobs/<job_id>`;
- cancel or delete one job with `DELETE /v1/ingest/jobs/<job_id>`;
- retry a failed job with `POST /v1/ingest/jobs/<job_id>/retry`.

The current retry and cancel behavior is deliberately small: it is a Core Alpha
operator surface for deterministic local jobs, not a distributed background job
system. Empty text, JSON, and CSV ingestion requests return zero cells and a
`null` first cell id instead of panicking or fabricating a cell.

On `Database::open`, stale jobs that were persisted as `running` are requeued as
`queued` with a recovery message. This is CortexDB's current restart-resume
boundary: the engine does not silently replay an HTTP upload body, but it also
does not leave an interrupted job looking active forever. Operators can inspect
the durable progress, then retry, cancel, or delete the job record explicitly.

The CLI exposes the same local job lifecycle for operator review:

```bash
cortexdb ingest-jobs ./db
cortexdb ingest-job ./db 1
cortexdb ingest-job-retry ./db 1
cortexdb ingest-job-cancel ./db 1
cortexdb ingest-job-delete ./db 1
```

`retry` only accepts failed jobs and clears the failure message while increasing
`retry_count`. `cancel` only accepts queued/running jobs. Completed jobs are
immutable from the cancel/retry path and can only be deleted as persisted
history.

## Implemented

- **TTL expiry/decay scanning** — `Database::expired_memory_cells` and
  `Database::expire_memory_cells` scan snapshots for TTL-elapsed memory cells and
  tombstone them through WAL. `Database::memory_decay_scores` returns fixed-point
  freshness scores per cell. See `cortex-engine/src/memory.rs`.

## Not Yet

- PDF layout reconstruction and object graph repair.
- OCR, page images, tables, forms, and scanned-document pipelines.
- Enrichment jobs.
