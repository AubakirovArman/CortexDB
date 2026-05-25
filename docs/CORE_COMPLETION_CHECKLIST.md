# CortexDB Core Completion Checklist

Core completion means the single-node durable path is internally consistent and
covered by tests.

## Required Invariants

- `PutCell`, `PatchCell`, and `TombstoneCell` append WAL before mutating `MemTable`.
- WAL operation records written by `Database` include durable `CommitSeq`.
- Restart restores `current_seq` from durable `CommitSeq`.
- Checkpoint and compact stop the WAL writer cleanly before truncating WAL.
- Checkpoint writes incremental segment/index bundles.
- Compact writes a full visible snapshot and retires older segment handles.
- Recovery loads live segments first and then replays WAL records newer than
  `manifest.checkpoint_seq`.
- Bitmap candidates are compact `u32` ids and map back to full `CellId`.
- Storage files are written through temp-file, fsync, rename, and parent fsync.
- Segment, bitmap index, lexical index, and manifest files reject CRC corruption.

## Validation Commands

```bash
cargo fmt --check
RUSTFLAGS="-D warnings" cargo check --workspace
RUSTFLAGS="-D warnings" cargo test --workspace --all-features
RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets -- -D warnings
```

## Deliberately Out Of Core

- Production BM25 ranking.
- Persistent vector pages.
- Production HNSW.
- Network replication and leader election.
- Document ingestion and LLM integration.
