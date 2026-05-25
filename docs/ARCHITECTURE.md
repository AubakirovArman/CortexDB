# CortexDB Architecture

Current pipeline:

```text
AQL -> Parser -> Raw AST -> Binder -> Bitmap VM -> Candidates_0
                                   |
                                   v
                              ACLOG WAL -> MemTable -> Segments
                                   |             |
                                   v             v
                              atomic manifest.acm   .acs/.acb/.aci
```

## Crates

- `cortex-aql`: query language, policy validation, binding, bitmap bytecode, and mock VM.
- `cortex-storage`: ACLOG WAL v0 binary codec, manifest, segment, and index files.
- `cortex-core`: in-memory MVCC MemTable, cell versions, read transactions, and manifest primitives.
- `cortex-engine`: database facade connecting WAL, MemTable, incremental checkpoint, compaction, query indexes, search helpers, and replay.
- `cortex-cli`: minimal local CLI for checking the engine loop, checkpoint, and compaction.
- `cortex-server`: minimal JSON HTTP API over the engine.

## Boundaries

AQL is not allowed to expand access. The binder starts from the agent-allowed
mask and live-cell mask, then intersects compiled `WHERE` filters.

`BitmapOp::Not` is evaluated inside the segment-local universe. It is a local
set complement, not a permission bypass.

Persistent storage is layered: WAL first, incremental checkpointed segment/index
files next, full snapshot compaction for segment retirement, then future garbage
collection and crash-matrix hardening.
