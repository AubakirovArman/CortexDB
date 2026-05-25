# Storage Roadmap

## Implemented

- ACLOG WAL v0 binary codec.
- ACLOG reader scan with safe truncate offset.
- ACLOG writer actor with strict and balanced durability modes.
- In-memory MVCC MemTable skeleton.
- Usable single-node loop through `cortex-engine`.
- Durable operation `CommitSeq` in WAL `CellCore`.
- Initial `.acs`, `.acb`, and `.aci` file foundations.
- Atomic manifest persistence and recovery into MemTable.
- Incremental engine checkpoint into `.acs`, `.acb`, `.aci`, and `manifest.acm`.
- Full snapshot compaction that retires older live segment handles.
- AQL retrieve over engine-built bitmap indexes.
- Persisted-index-first AQL retrieve with in-memory delta overlay.
- Compact bitmap candidate ids with persisted full `CellId` mapping.
- Atomic segment, bitmap-index, lexical-index, and manifest writes.
- CRC corruption detection for segment, bitmap-index, lexical-index, and manifest files.
- Fixed-point BM25-style ranking helper.
- Integer vector search and graph-backed HNSW-style search helper.
- Minimal `cortexdb` CLI for local put/get/tombstone/flush checks.
- JSON HTTP server API for put/get/tombstone/flush/compact/health checks with optional bearer auth.
- Single-node/distributed placement and deterministic quorum log skeleton.
- Crash/restart/corruption matrix tests for Core Alpha.
- Best-effort repair for orphan temp cleanup and safe WAL tail truncation.
- Atomic write audit and storage format documentation.
- SegmentBundle API and explicit retired segment garbage collection.
- ACLOG format freeze notes and read-only WAL diagnostics CLI.
- MemTable historical reads, deterministic iterators, range scan, and stats v2.
- AQL retrieve execution exposed through CLI and HTTP.

## Next

1. Formalize core cell metadata instead of payload-line metadata parsing.
2. Add WAL writer backpressure and real group commit batching.
3. Persist consensus log entries through ACLOG or a dedicated replication log.
4. Add real network replication and leader election.
5. Add background GC policy around active readers.

## Not Yet

- Large-scale BM25 ranking pipeline with analyzers and field weighting.
- Persistent vector index pages.
- Multi-layer HNSW with deletion and rebuild policy.
- Consensus, replication transport, and distributed recovery.
