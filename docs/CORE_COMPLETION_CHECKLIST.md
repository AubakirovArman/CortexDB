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
- Tombstone-only checkpoint records are preserved and never resurrect cells.
- `Database::validate_storage()` checks manifest segment counts, core storage
  files, indexes, and recoverable WAL state.
- `Database::storage_stats()` reports current sequence, checkpoint sequence,
  segment counts, MemTable stats, WAL size, and live WAL writer metrics.
- Persisted bitmap and lexical indexes merge same-key postings by set union
  across checkpoint segments.
- Candidate allocation is fallible; overflow and candidate id `0` fail closed.
- A single database directory has a process lock and shuts down its WAL writer
  from `Drop`.
- Runtime bitmap evaluation uses an AgentView-derived allowed mask, not only the
  full segment universe.
- `Database::open()` removes known orphan temp files in the database root and
  segment directory after acquiring the process lock.
- `Database::close()` performs explicit writer shutdown and releases the lock
  through normal drop.
- Validation rejects duplicate live segment ids, live/retired overlap,
  candidate id `0`, candidate ids mapped to multiple cells, and manifest
  checkpoint sequence regressions.
- `Database::validate_storage_report()` collects all detectable validation
  errors instead of stopping at the first one.
- Atomic writes use unique temp filenames and open-time cleanup removes both
  legacy and unique temp files.
- Stale lock recovery is explicit through `StaleLockPolicy::Break` or
  `cortexdb unlock <path> --force`.

## Validation Commands

```bash
cargo fmt --check
RUSTFLAGS="-D warnings" cargo check --workspace
RUSTFLAGS="-D warnings" cargo test --workspace --all-features
RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets -- -D warnings
```

## Deliberately Out Of Core

- Production BM25 ranking.
- Production HNSW.
- Persistent approximate vector indexes beyond exact `.acv` scan.
- Network replication and leader election.
- Document ingestion and LLM integration.
