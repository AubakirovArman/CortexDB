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
- Search API v1 with Unicode tokenization, field weights, and hybrid fusion.
- ContextPack sparse redundancy reduction with anomaly diagnostics.
- Structured `KnowledgeCell` ingestion API over the current payload encoding.
- Policy-checked AQL `REMEMBER` execution into memory cells.
- Policy-checked AQL `VERIFY FACT` evidence report v0.
- CLI and HTTP smoke surfaces for AQL `REMEMBER` and `VERIFY FACT`.
- Memory TTL expiry scan and WAL-backed tombstone path.
- Fixed-point memory decay scoring.
- Durable context feedback cells.
- ContextPack feedback ordering signal.
- Q16 source trust metadata and VERIFY FACT evidence ordering.
- Aggregated context feedback statistics.
- Explicit contradiction markers for VERIFY FACT reports.
- Scoped database keyword search exposed through CLI and HTTP.
- Queryable structured conflict index from contradiction markers.
- Centralized payload-header metadata parser with body/title lexical scoring fields.
- `.aci` lexical index v2 with per-candidate document lengths and `ACI0` read compatibility.
- `.aci` lexical index format magic `ACI2` with weighted term-frequency
  statistics and `ACI0`/`ACI1` read compatibility.
- WAL writer bounded queue option for caller-side backpressure.
- Balanced WAL writer group commit batches queued appends before acknowledgment.
- WAL writer metrics for records, bytes, fsyncs, and committed batches.
- WAL replay metrics for seen, applied, skipped, payload bytes, and safe offset.
- Public WAL writer metrics exposed through engine stats, CLI stats, and HTTP stats.
- Knowledge-cell writes include structured `CellMetadata` WAL sections.
- WAL truncate and manifest dump/validate CLI tools.
- Core benchmark baseline for put/get/checkpoint/reopen/context pack.
- Durable AgentView persistence under `agent_views/`.
- Public storage format inventory for current magic/version compatibility.
- Scoped keyword search reads persisted `.aci` postings directly when there is
  no uncheckpointed WAL tail, with MemTable snapshot fallback for fresh writes.
- `.acv` vector pages persist integer vectors and support exact scoped vector
  scan without snapshot rebuild when there is no uncheckpointed WAL tail.
- Public scoped vector search input is exposed through CLI and HTTP over the
  current exact `.acv`/snapshot vector path.
- ACLOG-backed replication log entries persist consensus-model `Term`,
  `LogIndex`, and payload records for local recovery.
- `VERIFY FACT` now applies conservative natural-language contradiction hints
  after structured `contradicts=` markers.

## Next

1. Add replication transport and leader-election semantics.
2. Add richer verification guards for citations and numeric claims.
3. Add production BM25 analyzers and quality fixtures.

## Not Yet

- Large-scale BM25 ranking pipeline with analyzers and field weighting.
- Persistent ANN vector index pages.
- Multi-layer HNSW with deletion and rebuild policy.
- Consensus transport, leader election, and distributed recovery.
