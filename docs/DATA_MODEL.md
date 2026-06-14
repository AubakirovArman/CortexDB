# CortexDB Data Model

Status: Core Alpha contract with database-grade target fields.

This document is the source contract for CortexDB knowledge cells, visibility,
versioning, lifecycle, provenance, and future typed metadata migration. It is
intended to be readable by client and engine implementers without reading the
Rust source.

## Model Boundary

CortexDB stores knowledge for AI agents. A query result is not only a set of raw
rows; the main query result is a ContextPack: a permission-filtered,
token-budgeted, cited, explainable subset of knowledge cells.

The durable write unit is a knowledge cell version:

```text
CellId
CommitSeq
CellDescriptor-compatible metadata
payload bytes
optional tombstone state
```

Core Alpha currently serializes metadata as leading payload header lines. New
knowledge-cell writes also carry a structured WAL metadata section, but payload
headers remain the compatibility source of truth until the typed descriptor
migration is complete.

## Knowledge Cell

A knowledge cell is a typed, versioned knowledge object. It is not just a text
chunk and not just a vector. It has:

- a stable `CellId`;
- a `CommitSeq` visibility boundary;
- metadata used by access control, retrieval, ranking, verification, and
  provenance;
- payload bytes used as the human/agent-readable body;
- optional lifecycle state such as TTL, tombstone, or memory type.

Supported cell types in Core Alpha:

```text
document_block
table
fact
entity
relation
memory
feedback
tool
source_ref
raw
```

Unknown cell types are invalid for strict metadata decoding. Looser ingestion
paths may sanitize or reject invalid values before write.

## Current Metadata Compatibility Contract

The current payload format is:

```text
scope=<scope_id>
status=<ready|stale|...>
type=<cell_type>
[memory_type=<type>]
[ttl_seconds=<seconds>]
[created_unix_seconds=<unix_seconds>]
[source_trust_q16=<0..65535>]
[source_trust_class=<class>]
[source=<source_label>]
[citation=<citation_text>]
[source_id=<source_ref_id>]
[source_url=<url>]
[document_id=<document_id>]
[chunk_id=<chunk_or_range>]
[cell_range=<chunk_or_range>]
[json_path=<path>]
[page=<page_number>]
[row=<row_number>]
[confidence_q16=<0..65535>]
[content_hash=<hash>]

payload body bytes
```

`docs/CELL_METADATA_MODEL.md` defines the complete Core Alpha payload header
serialization rules. This document defines the higher-level data model and the
target typed descriptor that will replace hot-path payload parsing.

## Target CellDescriptor

The database-grade target is a typed descriptor that is read without parsing
user payload text. It should be stored in WAL records and segment metadata, and
materialized in `CellVersion` on replay/open.

| Field | Type | Required | Purpose |
| --- | --- | --- | --- |
| `cell_id` | `u64` / `CellId` | Yes | Stable identity for a cell. |
| `created_seq` | `CommitSeq` | Yes | First sequence where the version is visible. |
| `deleted_seq` | `Option<CommitSeq>` | No | Tombstone visibility boundary. |
| `scope_id` | interned string/id | Yes | Mandatory permission and tenant visibility field. |
| `brain_id` | string/id | Yes, default allowed | Logical knowledge namespace. |
| `cell_type` | enum | Yes | Retrieval and validation type. |
| `status` | enum/string | Yes | Visibility state such as ready/stale. |
| `memory_type` | enum/string | No | Agent memory subtype. |
| `ttl_seconds` | `Option<u64>` | No | Expiration policy for memory/lifecycle. |
| `created_unix_seconds` | `Option<u64>` | No | Recency and freshness ranking. |
| `valid_from` | `Option<String>` | No | Temporal validity start (`YYYY-MM-DD`, inclusive). |
| `valid_to` | `Option<String>` | No | Temporal validity end (`YYYY-MM-DD`, inclusive). |
| `source_trust_q16` | `Option<u16>` | No | Fixed-point trust score. |
| `source_trust_class` | enum/string | No | Calibrated trust class. |
| `source_id` | string | No | Structured provenance id. |
| `citation` | string | No | Human-readable citation. |
| `source_url` | string | No | Optional URL. |
| `document_id` | string | No | Source document identity. |
| `cell_range` | string | No | Stable chunk/range identity. |
| `json_path` | string | No | JSON source path for structured ingestion. |
| `page` | `Option<u32>` | No | Page number for document/PDF sources. |
| `row` | `Option<u32>` | No | Row number for table/CSV sources. |
| `confidence_q16` | `Option<u16>` | No | Fixed-point provenance confidence. |
| `content_hash` | string | No | Deduplication and audit identity. |
| `parent_id` | string | No | Parent chunk/document relation. |
| `embedding_ref` | string/id | No | Reference to vector storage. |

The descriptor, not payload text, is the target source of truth for permission
checks, lifecycle, provenance, and index updates.

## Visibility And MVCC

CortexDB uses sequence-based visibility:

```text
visible(version, read_seq) =
  version.created_seq <= read_seq &&
  (version.deleted_seq is None || read_seq < version.deleted_seq)
```

Reads should use a `ReadTxn` snapshot sequence. A long-lived reader must keep
the versions it can see valid until the reader is released. The target
database-grade contract is snapshot isolation for readers plus atomic
single-writer batches.

### Snapshot pinning contract

`Database::pin_read_txn()` returns a `PinnedReadTxn` handle that registers the
current `CommitSeq` in the active pin registry. The handle keeps the snapshot
valid until it is dropped. `Database::gc_horizon()` returns the oldest pinned
sequence, or `current_seq` when no pins exist. Checkpoint and compaction call
`gc_versions_before(gc_horizon())`, so versions visible to any pinned snapshot
are never removed. Short point reads may use `read_txn()` without pinning, but
any iterator or long-lived scan must hold a `PinnedReadTxn` for the duration of
the read.

## Tombstone Semantics

A tombstone hides a cell from reads at or after the tombstone sequence. A
tombstone does not erase historical visibility for older snapshots. Tombstone
state must survive WAL replay, checkpoint, compaction, backup, and restore.

Target behavior:

- tombstone is a first-class version state;
- compaction may drop obsolete versions only after the GC barrier proves no
  reader can observe them;
- validation must detect inconsistent tombstone/index/manifest state.

## Scope, Brain, And AgentView

`scope_id` is a security field. A query must not read, rank, pack, explain, or
leak payload bytes outside the requesting `AgentView` readable scopes.

Target invariant:

```text
No payload bytes outside AgentView.readable_scopes can appear in a result,
explain trace, anomaly, metric, or error body.
```

Core Alpha enforces this mainly through AQL binder policy and metadata-derived
scope filters. The target model enforces it by intersecting permission bitmaps
inside physical scans before payload reads.

`brain_id` is not an isolation namespace in the current storage model.
`default = BrainId(1)` is the only real brain. Non-default AQL brain names are
deprecated aliases for `BrainId(1)` and must not be presented as separate
storage, permission, statistics, or index namespaces. Use scopes and tenants for
isolation; see [`BRAIN_SEMANTICS.md`](BRAIN_SEMANTICS.md).

## Lifecycle

Lifecycle fields describe whether and when a cell should participate in reads:

- `status`: the basic visibility status used by filters.
- `ttl_seconds`: memory expiration policy from creation time.
- `created_unix_seconds`: recency and freshness ranking.
- `valid_from` / `valid_to`: temporal truth validity, not storage lifetime.

Expiration may write a tombstone. Temporal invalidity should normally filter a
cell from evidence for a time-scoped query but does not delete it.

Temporal validity is inclusive and open-ended:

- missing `valid_from` means the cell is valid from the beginning of the
  timeline;
- missing `valid_to` means the cell remains valid until superseded or deleted;
- both missing means the cell has no explicit temporal validity window and is
  eligible for all `REQUIRE VALID AT` dates;
- date strings use `YYYY-MM-DD`; shorter legacy year/month forms may still be
  parsed by verification helpers, but AQL `REQUIRE VALID AT` requires a full
  ISO date.

AQL retrieval can request a temporal evidence slice:

```aql
RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
WHERE space = project:investments
REQUIRE valid at "2025-06-01";
```

The engine applies this filter from descriptor-backed metadata before payload
materialization, so expired or not-yet-valid cells do not spend lazy payload
reads.

## Provenance

Provenance fields allow ContextPack and VERIFY outputs to be audited:

- `source_id`
- `citation`
- `source_url`
- `document_id`
- `chunk_id`
- `json_path`
- `page`
- `row`
- `content_hash`

ContextPack citations should be built from typed provenance fields when
available. New WAL descriptor sections carry `source_id`, `source_url`,
`document_id`, `cell_range`, `json_path`, `page`, `row`, `confidence_q16`,
`citation`, and `content_hash` when those values are present at write time.
Payload-header provenance remains valid during Core Alpha compatibility.

## Compatibility And Migration

The migration path is dual-read, not big-bang:

1. Existing payload headers remain readable.
2. New writes include payload headers for compatibility and a structured WAL
   `CellDescriptor` carrying descriptor-backed provenance fields.
3. Replay/open materializes a descriptor from structured metadata when present,
   or from payload headers when reading legacy data.
4. Segment format v2 stores descriptors and payload offsets separately.
5. A future migration command can rewrite old segments into the descriptor
   format.

Breaking format changes require version gates, fixtures, and explicit migration
tests.

## Stable And Experimental

Stable for Core Alpha:

- `CellId` and `CommitSeq` visibility;
- payload-header metadata compatibility;
- WAL before MemTable mutation;
- basic cell types listed above;
- AQL/ContextPack scope filtering through current metadata paths.
- descriptor-backed provenance for WAL replay/open and ContextPack exports.

Experimental:

- removing payload-header compatibility;
- typed temporal validity columns;
- fact/numeric indexes;
- lazy payload residency;
- real multi-brain semantics beyond deprecated aliases.

## Related Documents

- [`CELL_METADATA_MODEL.md`](CELL_METADATA_MODEL.md) — current payload metadata
  serialization.
- [`BRAIN_SEMANTICS.md`](BRAIN_SEMANTICS.md) — current single-brain contract and
  migration plan for deprecated aliases.
- [`ARCHITECTURE.md`](ARCHITECTURE.md) — storage and query architecture.
- [`CONTEXT_PACK.md`](CONTEXT_PACK.md) — ContextPack behavior.
- [`AQL_V0_5.md`](AQL_V0_5.md) — current REMEMBER write contract.
- [`AQL_V0_4.md`](AQL_V0_4.md) — frozen query language grammar.
- [`AUTH.md`](AUTH.md) — HTTP auth and AgentView token binding.
