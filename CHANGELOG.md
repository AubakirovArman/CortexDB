# Changelog

## Unreleased

- Added AQL parser, binder, policy validation, and mock bitmap VM.
- Added ACLOG WAL v0 codec, reader recovery scan, and writer actor.
- Added in-memory MVCC MemTable and manifest skeleton.
- Added statement-level binding, bound plan variants, catalog facade traits, parser diagnostics,
  and bitmap bytecode explain output.
- Added `cortex-engine` usable single-node database loop with WAL replay and MemTable reads.
- Added durable operation commit sequence, AQL retrieve over engine candidates, initial segment/index
  file foundations, and a minimal CLI.
- Added manifest-backed checkpoint/recovery, engine flush to `.acs/.acb/.aci`, engine-backed AQL
  retrieve, MVP BM25/vector/HNSW helpers, minimal distributed placement, and `cortex-server`.
- Added incremental checkpoint tombstone markers, atomic manifest writes, persisted-index-first
  AQL retrieval with delta overlay, compaction, graph-backed HNSW-style search, JSON/auth server
  responses, CLI compact, and deterministic quorum log primitives.
- Added core-completion invariants: compact bitmap candidates with full `CellId` mapping, atomic
  segment/index writes, CRC corruption checks for core storage files, WAL restart-after-checkpoint
  tests, and the core completion checklist.
