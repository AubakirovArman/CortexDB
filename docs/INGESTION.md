# Ingestion v1

Core Alpha now has a minimal structured cell API:

- `KnowledgeCellType`
- `KnowledgeCellMetadata`
- `KnowledgeCell`
- `Database::put_knowledge_cell`

The durable format is intentionally unchanged. `KnowledgeCell::encode_payload`
serializes metadata into the existing payload-line convention:

```text
scope=project:investments
status=verified
type=fact
source=annual-report

body bytes...
```

This lets AQL filters and ContextPack citations work through the current WAL,
MemTable, segment, bitmap, and lexical paths while reserving a future migration
to structured metadata sections.

## Not Yet

- Metadata WAL sections.
- Document loaders.
- JSON/CSV/PDF ingestion adapters.
- Enrichment jobs.
- Progress API.
