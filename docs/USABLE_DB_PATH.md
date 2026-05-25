# Usable Database Path

Milestone 0.6 connects the existing storage pieces into the first usable
single-node loop:

```text
PutCell -> WAL append -> MemTable update -> restart -> WAL replay -> Retrieve
```

The critical write invariant is:

```text
append durable WAL record first, update MemTable second
```

If WAL append fails, the in-memory state must not change.

The current MVP stores operations as ACLOG records:

- `PutCell` uses `WalRecordType::PutCellBatch`
- `PatchCell` uses `WalRecordType::PatchCellBatch`
- `TombstoneCell` uses `WalRecordType::TombstoneBatch`

`CellCore` stores little-endian `CellId`. `PayloadInline` stores payload bytes.

Replay currently derives `CommitSeq` from record order. This is sufficient for
the first local loop, but `commit_seq` must become durable in a future WAL
header or operation section.
