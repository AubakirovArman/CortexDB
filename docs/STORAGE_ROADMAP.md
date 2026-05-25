# Storage Roadmap

## Implemented

- ACLOG WAL v0 binary codec.
- ACLOG reader scan with safe truncate offset.
- ACLOG writer actor with strict and balanced durability modes.
- In-memory MVCC MemTable skeleton.
- Usable single-node loop through `cortex-engine`.
- Durable operation `CommitSeq` in WAL `CellCore`.
- Initial `.acs`, `.acb`, and `.aci` file foundations.
- Manifest persistence and recovery into MemTable.
- Engine checkpoint into `.acs`, `.acb`, `.aci`, and `manifest.acm`.
- AQL retrieve over engine-built bitmap indexes.
- Minimal fixed-point BM25-style ranking helper.
- Minimal integer vector search and HNSW-compatible exact fallback helper.
- Minimal `cortexdb` CLI for local put/get/tombstone/flush checks.
- Minimal HTTP server API for put/get/tombstone/flush checks.
- Single-node/distributed placement configuration skeleton.

## Next

1. Replace full-snapshot checkpoint with incremental segment flush.
2. Add manifest atomic-write protocol and crash tests around checkpoint phases.
3. Use persisted `.acb` / `.aci` directly without rebuilding query indexes from payloads.
4. Replace HNSW exact fallback with graph ANN insertion/search.
5. Add production server protocol, auth, and structured responses.

## Not Yet

- Production BM25 ranking pipeline.
- Production vector index.
- Real HNSW graph ANN.
- Consensus, replication transport, and distributed recovery.
