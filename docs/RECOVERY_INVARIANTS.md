# Recovery Invariants

- WAL replay must be deterministic.
- WAL append happens before MemTable update.
- A failed WAL append must not mutate MemTable.
- Partial tails may be truncated only to `safe_truncate_offset`.
- Corrupt payload CRC must not be silently accepted. Strict recovery errors;
  best-effort recovery stops at the last valid record.
- Unknown WAL sections may be ignored by operation decoding when required
  sections are present.
- `CommitSeq` is durable in `CellCore`; replay order is only a compatibility fallback.
- Manifest `checkpoint_seq` defines the highest sequence already represented in
  live `.acs` segments.
- Recovery loads live segments first, then skips WAL records with durable
  `CommitSeq <= checkpoint_seq`.
- Checkpoint writes only versions and tombstone markers changed after the last
  manifest checkpoint.
- Tombstone markers in incremental segments must prevent old checkpointed cells
  from resurrecting after WAL truncation.
- Compaction writes a full visible snapshot and retires previous segment handles.
- Bitmap candidates are compact ids. Segment cells persist the mapping back to
  full `CellId`, so large ids are not truncated.
- Segment, bitmap index, lexical index, and manifest files include CRC footers
  and must fail closed on corruption.
