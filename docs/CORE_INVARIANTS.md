# CortexDB Core Invariants

These invariants define the current single-node durable core.

1. WAL append happens before MemTable update.
2. Every new WAL operation contains `CommitSeq`.
3. `CandidateId` is internal and never truncates `CellId`.
4. `CandidateId(0)` is invalid.
5. Segment, index, and manifest writes are atomic and checksummed.
6. Manifest publication happens after segment and index files are durable.
7. Checkpoint truncates WAL only after manifest is durable.
8. Tombstones cannot resurrect cells after restart.
9. Strict recovery fails on WAL corruption.
10. Best-effort recovery stops at a safe WAL offset.
11. Runtime `AgentAllowed` masks are agent-specific.
12. One process may own a database directory at a time.
13. Atomic temp paths are unique per process and write counter.
14. Validation report generation collects all detectable storage errors.
