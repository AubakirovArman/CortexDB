# CortexDB Core Alpha

Core Alpha is the single-node durable database path. It is intentionally smaller
than the full CortexDB roadmap.

## Included

- `PutCell`, `PatchCell`, and `TombstoneCell`.
- WAL append before MemTable mutation.
- Durable `CommitSeq` in new WAL operation records.
- Restart recovery from checkpoint plus WAL tail.
- Incremental checkpoint into `.acs`, `.acb`, `.aci`, and manifest files.
- Compact into a full visible snapshot segment.
- AQL retrieve over current and persisted candidates.
- Full `CellId` preservation through internal candidate mappings.
- CRC checks for WAL, segment, bitmap index, lexical index, and manifest files.
- CLI and HTTP smoke paths for put/get/flush/compact/stats/validate.
- Explicit stale lock recovery and collect-all validation reports.

## Excluded

- Production BM25 scoring.
- Persistent vector pages.
- Production HNSW.
- Distributed consensus.
- Document ingestion.
- LLM integration.
