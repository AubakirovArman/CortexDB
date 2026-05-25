# CortexDB Architecture

Current pipeline:

```text
AQL -> Parser -> Raw AST -> Binder -> Bitmap VM -> Candidates_0
                                   |
                                   v
                              ACLOG WAL -> MemTable -> Segments
```

## Crates

- `cortex-aql`: query language, policy validation, binding, bitmap bytecode, and mock VM.
- `cortex-storage`: ACLOG WAL v0 binary codec, reader recovery scan, and writer actor.
- `cortex-core`: in-memory MVCC MemTable, cell versions, read transactions, and manifest primitives.
- `cortex-engine`: database facade connecting WAL append, MemTable update, and replay.

## Boundaries

AQL is not allowed to expand access. The binder starts from the agent-allowed
mask and live-cell mask, then intersects compiled `WHERE` filters.

`BitmapOp::Not` is evaluated inside the segment-local universe. It is a local
set complement, not a permission bypass.

Persistent storage is being built in layers: WAL first, then MemTable recovery,
then segment files and indexes.
