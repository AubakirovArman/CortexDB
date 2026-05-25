# WAL Replay

`cortex-engine::replay_wal(path)` scans ACLOG records and rebuilds a MemTable.

Replay reads durable commit sequence from `CellCore`:

```text
CellCore = little-endian CellId + little-endian CommitSeq
```

If an old WAL record only contains `CellId`, replay falls back to record order.

Partial WAL tails are handled through `safe_truncate_offset`. `Database::open`
truncates bytes after the safe offset before starting a writer, so later appends
remain replayable.

`RecoveryMode::Strict` treats corrupt payload CRC as a recovery error.
`RecoveryMode::BestEffort` stops at the last valid record and truncates the
tail before new appends.

WAL diagnostics are read-only and do not acquire a database lock:

```bash
cortexdb wal-validate ./data
cortexdb wal-dump ./data
```

Use them for inspection before choosing strict recovery, best-effort repair, or
manual archival.
