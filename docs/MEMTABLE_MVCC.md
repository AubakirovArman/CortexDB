# MemTable MVCC

`cortex-core::MemTable` stores versions by `CellId`.

Visibility rule:

```text
created_seq <= read_seq AND (deleted_seq IS NULL OR deleted_seq > read_seq)
```

`Database::read_txn()` returns a snapshot at the current commit sequence.
`Database::get_cell(txn, cell_id)` reads through that snapshot.

`put_cell` appends a new version. `patch_cell` marks the latest version deleted
and appends a replacement. `tombstone_cell` marks the latest version deleted.

## Core Alpha API

- `ReadTxn::at(seq)` creates a historical read snapshot.
- `MemTable::read_at(seq, cell_id)` reads a single historical version.
- `MemTable::live_cell_ids(txn)` returns visible cells in `CellId` order.
- `MemTable::deleted_cell_ids()` returns cells whose latest version is deleted.
- `MemTable::range_scan(txn, start, end)` scans visible versions in `CellId`
  order.
- `MemTable::stats()` reports live/deleted cell counts, version count,
  tombstones, max delta depth, index debt, and payload bytes.

`patch_cell` is a full payload replacement in Core Alpha. Section-level patch
merge remains future work.
