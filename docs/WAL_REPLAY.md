# WAL Replay

`cortex-engine::replay_wal(path)` scans ACLOG records and rebuilds a MemTable.

Replay order defines temporary commit sequence:

```text
first record -> CommitSeq(1)
second record -> CommitSeq(2)
```

This is an MVP rule. A future format must persist commit sequence explicitly.

Partial WAL tails are handled through `safe_truncate_offset`. `Database::open`
truncates bytes after the safe offset before starting a writer, so later appends
remain replayable.

Corrupt payload CRC is treated as a recovery error.
