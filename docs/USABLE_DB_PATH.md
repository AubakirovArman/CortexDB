# Usable Database Path

Milestone 0.6 connects the existing storage pieces into the first usable
single-node loop:

```text
PutCell -> WAL append -> MemTable update -> restart -> WAL replay -> Retrieve
Checkpoint -> .acs/.acb/.aci -> manifest.acm -> restart -> segment load -> WAL tail replay
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

`CellCore` stores little-endian `CellId` and durable `CommitSeq`.
`PayloadInline` stores payload bytes.

Replay uses durable `CommitSeq` from `CellCore`. Old records without that field
fall back to replay order for compatibility.

Checkpoint writes a full visible snapshot for the current MVP. The manifest stores
the checkpoint sequence, so recovery can skip durable WAL records already included
in the segment snapshot and replay only newer tail records.
