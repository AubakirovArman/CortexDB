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

Checkpoint writes only visible versions and tombstone markers changed after the
previous manifest `checkpoint_seq`. The manifest is updated through an atomic
temp-file plus rename protocol. Recovery loads live segments first, then skips
durable WAL records already covered by the manifest and replays only newer tail
records.

Compaction is a separate full visible snapshot operation. It retires previous
live segment handles and produces one current segment/index set.
