# Core Engine

`cortex-engine` is the first facade over the lower layers.

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
- ContextPack v0 construction over retrieved cells
- storage stats, validation, and best-effort repair surfaces

It still does not provide production BM25, HNSW storage, distributed
consensus, document ingestion, or LLM integration.
