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
