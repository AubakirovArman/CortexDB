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
- `ingest_json`: flat JSON object fields into fact cells with
  `json_path=<flattened.path>` SourceRef provenance.
- `ingest_csv`: header row plus one document block per data row with
  `row=<1-based source row>` and `cell_range=row-<n>` provenance.
- `ingest_pdf_text`: external PDF extraction hook that stores extracted text
  with `source_format=pdf` and optional `page=<n>` SourceRef metadata.
- `ingest_pdf_bytes`: native no-dependency extractor for simple uncompressed
  PDF text objects and `/FlateDecode` zlib streams before storing one aggregate
  `source_format=pdf` cell with extraction metadata.
- `ingest_pdf_bytes_pages`: native digital-PDF extraction with one emitted
  document block per extracted text-bearing page. Each cell writes
  `page=<n>`, `cell_range=page-<n>`, `source_format=pdf`,
  `extraction_boundary=native_digital_pdf`, and `pdf_page_count=<n>`.

The native PDF extractor handles literal strings and hex strings inside
`BT ... ET` text objects, including simple compressed Flate streams. It
intentionally rejects unsupported/empty PDFs instead of silently storing an
empty document.

PDF extraction has an explicit adapter boundary. `NativeDigitalPdfTextExtractor`
is the local digital-PDF path for files that already contain text. Production
layout-aware digital parsing belongs behind `ExternalPdfParserAdapter`; scanned
PDFs and page images must use an external `ExternalOcrAdapter`. The disabled
adapters validate request shape and then fail closed. Scanned PDFs must cross
the explicit `ScannedPdfOcrRequest` boundary after their pages have been
rendered to images. OCR output can carry page-level confidence plus block-level
confidence and normalized bounding boxes, and invalid OCR output fails
validation before it enters normal text chunking. See
[`PDF_TEXT_EXTRACTION.md`](archive/PDF_TEXT_EXTRACTION.md).

Focused gate:

```bash
make pdf-digital-adapter-check
make ocr-adapter-trait-check
```

## Text Chunking Policy

Plain-text chunking is now an engine-level policy instead of ad-hoc adapter
logic. `TextChunkPolicy` controls maximum chunk size, overlap, and minimum
chunk size. `split_text_chunks(document_id, text, policy)` returns deterministic
`TextChunk` records with stable ids:

```text
<sanitized-document-id>#chunk-0001
<sanitized-document-id>#chunk-0002
```

`Database::ingest_text_chunks` uses the default policy. For tests or controlled
imports, `Database::ingest_text_chunks_with_policy` accepts an explicit policy.
This matters because ContextPack citations need a stable provenance handle: each
stored text chunk writes `source_id`, `document_id`, and `chunk_id` in the
payload header before the body, so `CellMetadata::from_payload` can expose it as
a structured `SourceRef`.

The deterministic ingestion policy is frozen in
[`DETERMINISTIC_CHUNKING.md`](archive/DETERMINISTIC_CHUNKING.md). In short:

- text uses stable `<document>#chunk-000N` ids and fixed character overlap for
  long paragraphs;
- JSON emits sorted leaf `json_path` values with `.` separators and numeric
  array components;
- CSV/table ingestion treats row 1 as the header and emits data row provenance
  as `row=<n>` and `cell_range=row-<n>`.

Chunk size defaults are also tracked by a retrieval-quality benchmark. See
[`CHUNKING_QUALITY_BENCHMARK.md`](archive/CHUNKING_QUALITY_BENCHMARK.md) and run:

```bash
make chunking-quality-benchmark-check
```

`IngestionProgressTracker` provides a small synchronous progress surface for
adapter jobs. The first tracked helper is `Database::ingest_csv_with_progress`,
which records total rows, completed cells, status, and the last written
`CellId`. Persisted job files are written atomically with file sync plus rename,
so a restart should see either the old complete job state or the new complete
job state, not a partially written JSON file.

## Optional Embedding at Ingest (opt-in)

Text ingestion can attach a vector to each stored chunk so the cell becomes
retrievable by vector search without a separate backfill pass. This is
**opt-in and off by default**: the engine itself stays network-free.

- Engine API: `Database::ingest_text_chunks_with_embedder(first_cell_id, text,
  options, policy, update_policy, embedder)` takes an injected
  `&dyn Embedder`. The engine calls `embedder.embed(chunk_text)` for each chunk
  and writes the returned lanes as a `vector=` payload header. The embedding
  backend (HTTP client, model, quantization) is owned by the caller, not the
  engine. Offline/deterministic tests use `DeterministicTestEmbedder`.
- HTTP API: `POST /v1/ingest/text?embed=true`. When `embed=true`, the server
  builds an embedder from the `CORTEXDB_EMBEDDING_*` environment variables and
  embeds each chunk during ingest. The `embed` value is parsed fail-closed: a
  non-boolean value is rejected with `400 bad_request` before any write, and
  `embed=true` with no embedding endpoint configured returns
  `bad_request: semantic requires vector or embedding config` rather than
  silently storing un-embedded cells.
- Default path unchanged: without `embed` (or `embed=false`), ingest writes no
  `vector=` header and performs no network I/O, exactly as before.

Environment configuration (server side) reuses the query-embedding client:

```bash
CORTEXDB_EMBEDDING_URL=https://<provider>/v1/embeddings
CORTEXDB_EMBEDDING_MODEL=<model-id>            # optional
CORTEXDB_EMBEDDING_API_KEY=<key>               # optional
CORTEXDB_EMBEDDING_TIMEOUT_MS=2000             # optional
```

An embedding error during ingest fails the whole `POST /v1/ingest/text` call
(fail-closed) instead of persisting a partially embedded batch.

### Corpus backfill

Cells ingested without embeddings (or before an endpoint was configured) can be
embedded later with the corpus-wide maintenance endpoint:

```text
POST /v1/embedding/backfill?batch_size=64&max_items=1000
```

- Drives the engine's `Database::backfill_embedding_debt_batched`, which scans
  for cells that still lack a vector and embeds them in batches through the
  configured `CORTEXDB_EMBEDDING_*` endpoint.
- **Idempotent by content hash**: a cell whose current text is already embedded
  is not re-embedded, so re-running the endpoint converges to zero remaining
  debt (`embedded_items` drops to `0`).
- Fail-closed: a malformed `batch_size`/`max_items` is a `400` before any work;
  with no embedding endpoint configured the call returns
  `bad_request: semantic requires vector or embedding config`.
- This is an operator/maintenance surface (analogous to `/v1/compact`): it
  operates corpus-wide rather than per agent scope. The response is an
  `EmbeddingBackfillReport` (`scanned_items`, `debt_items`, `embedded_items`,
  `failed_items`, `final_debt_items`, …).

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

## Ingestion Backpressure

Ingestion now has an engine-level `IngestionBackpressurePolicy`. The policy is
configured through `DatabaseOptions` and is checked before a new HTTP ingestion
job is created or any cells are written. This protects the local WAL/MemTable
path from unbounded uploads and job buildup.

The policy currently covers:

- queued job limit: too many durable queued jobs returns `database_busy`;
- running job limit: too many durable running jobs returns `database_busy`;
- input byte limit: oversized request bodies return `payload_too_large`;
- item limit: oversized row/fact batches are rejected before writes;
- rate limit: too many accepted ingestion starts in one window return
  `rate_limited`;
- cancellation guard: cancelled persisted jobs cannot be continued by future
  worker-style ingestion paths.

Core Alpha ingestion is still synchronous. Backpressure therefore acts as a
pre-write admission gate, not as a distributed work queue scheduler. The
focused regression gate is:

```bash
make ingestion-backpressure-check
```

## Ingestion Deduplication

Ingestion payloads now include deterministic hash metadata:

- `source_hash`: stable FNV-1a hash of the source identifier.
- `content_hash`: stable FNV-1a hash of the emitted body/chunk text.

These hashes are written into text, JSON, CSV, and other SourceRef-style
ingestion payloads before the body separator. `CellMetadata::from_payload` and
`CellMetadata::decode_payload` expose both fields so validation/reporting tools
can reason about duplicates without reparsing raw headers.

The update policy is explicit:

- `IngestionUpdatePolicy::AlwaysInsert` keeps the historical Core Alpha
  behavior and writes every emitted chunk.
- `IngestionUpdatePolicy::SkipExisting` skips a text chunk when a visible cell
  already has the same `source_hash` and `content_hash`.

Current deduplication is a deterministic local pre-write check over the visible
snapshot. It is not yet a persisted global dedup index, so high-volume imports
should treat it as a correctness guard, not as a large-scale indexing strategy.

Focused gate:

```bash
make ingestion-deduplication-check
```

## Ingestion Validation Report

Every HTTP ingestion response includes `validation_report`. The report is built
from the cells that were actually written:

- `cells_seen`: number of emitted cells checked after write.
- `processed_records`: number of emitted cells inspected from durable storage.
- `skipped_records`: number of non-error inputs skipped by the adapter.
- `invalid_metadata_records`: number of emitted cells whose headers fail strict
  metadata validation.
- `warnings`: structured issues such as missing payloads, missing SourceRef
  metadata, chunk id mismatch, or invalid metadata.
- `skipped_items`: non-error skips such as `no_cells_emitted` for empty text,
  `{}`, `[]`, or header-only CSV.
- `source_refs`: per-cell summary of SourceRef availability, source id, source
  URL, document id, page, row, cell range, JSON path, chunk id, and confidence.

The engine API exposes the same check as
`Database::ingestion_validation_report(&cells)`. This keeps ingestion reports
derived from durable cells rather than request-local assumptions.

## Implemented

- **TTL expiry/decay indexing** — `Database::expired_memory_cells` and
  `Database::expire_memory_cells` use the maintained memory lifecycle index to
  find TTL-elapsed memory cells and tombstone them through WAL.
  `Database::memory_decay_scores` returns fixed-point freshness scores per cell
  without payload scans. See `cortex-engine/src/memory.rs`.

## Not Yet

- PDF layout reconstruction and object graph repair.
- OCR, page images, tables, forms, and scanned-document pipelines.
- Enrichment jobs.
