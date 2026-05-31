# Core Engine

`cortex-engine` is the first facade over the lower layers.

The stable embedded entrypoints are documented in `ENGINE_API.md`; the common
single-node loop starts with `Database::open`, applies `PutCell` operations
through the WAL, then serves reads from the MVCC snapshot.

It owns:

- database directory creation
- WAL path selection (`db.aclog`)
- WAL replay during open
- WAL writer startup
- MemTable updates after successful WAL append
- snapshot reads from the current commit sequence
- checkpoint and compact publication into segment/index/manifest files
- AQL retrieve execution over engine bitmap indexes
- persisted `.aci` keyword search and `.acv` exact vector scan
- ContextPack v1 construction over retrieved cells
- storage stats, validation, and best-effort repair surfaces
- ACLOG-backed replication log entry persistence for local consensus-model recovery
- deterministic in-memory replication transport and leader-election semantics

It still does not provide production BM25, HNSW storage, production consensus
networking, document ingestion, or LLM integration.
