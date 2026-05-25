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
