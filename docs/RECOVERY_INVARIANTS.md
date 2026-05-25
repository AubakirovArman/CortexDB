# Recovery Invariants

- WAL replay must be deterministic.
- WAL append happens before MemTable update.
- A failed WAL append must not mutate MemTable.
- Partial tails may be truncated only to `safe_truncate_offset`.
- Corrupt payload CRC must not be silently accepted.
- Unknown WAL sections may be ignored by operation decoding when required
  sections are present.
- Current `CommitSeq = replay order` is temporary and must become durable.
