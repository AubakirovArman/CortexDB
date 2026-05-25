# Ingestion v1

Core Alpha now has a minimal structured cell API:

- `KnowledgeCellType`
- `KnowledgeCellMetadata`
- `KnowledgeCell`
- `Database::put_knowledge_cell`
- `Database::remember_aql`

The durable format is intentionally unchanged. `KnowledgeCell::encode_payload`
serializes metadata into the existing payload-line convention:

```text
scope=project:investments
status=verified
type=fact
source=annual-report
source_trust_q16=60000

body bytes...
```

This lets AQL filters and ContextPack citations work through the current WAL,
MemTable, segment, bitmap, and lexical paths while reserving a future migration
to structured metadata sections.

`REMEMBER` execution is intentionally a thin bridge. The AQL parser and binder
enforce `AgentView.allow_remember`, write-scope access, allowed `MemoryType`, and
TTL policy. The engine then stores the memory as a `KnowledgeCellType::Memory`
payload with `memory_type`, optional `ttl_seconds`, and `source=agent:<id>`.

## Not Yet

- Metadata WAL sections.
- Document loaders.
- JSON/CSV/PDF ingestion adapters.
- Enrichment jobs.
- Progress API.
- TTL expiry/decay scanning.
