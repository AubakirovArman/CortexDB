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

## Not Yet

- Document loaders.
- JSON/CSV/PDF ingestion adapters.
- Enrichment jobs.
- Progress API.
- TTL expiry/decay scanning.
