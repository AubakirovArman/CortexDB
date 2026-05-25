# Core Consistency Audit

This audit pauses new feature work and checks the existing Core Alpha links.

## Verified Dataflow

```text
Database::put_cell / patch_cell / tombstone_cell
-> wal_record_from_operation_with_seq
-> WalWriterHandle::append
-> MemTable mutation
-> checkpoint / compact
-> .acs + .acb + .aci + manifest.acm
-> Database::open
-> load checkpoint
-> replay WAL tail
-> AQL bind and bitmap VM
-> AgentView runtime mask
-> ContextPack
```

## Consistency Checks

| Area | Status | Evidence |
| --- | --- | --- |
| WAL before MemTable | OK | `Database::append_then_apply` appends before `apply_operation`; database loop tests cover restart. |
| Durable sequence | OK | New operations use `wal_record_from_operation_with_seq`; replay rejects operation records missing seq. |
| Candidate mapping | OK | `EngineAqlIndex` stores both `candidate_to_cell` and `cell_to_candidate`; old truncation patterns are absent. |
| Persisted index merge | OK | Checkpoint index merge unions postings across live segments. |
| Agent permissions | OK | `EngineAqlProvider` builds `agent_allowed` from readable scopes, not the full universe. |
| Tombstone behavior | OK | `record_tombstone` supports tombstone-only checkpoint and replay paths. |
| Atomic storage writes | OK | Segment, bitmap, lexical, and manifest writers use `write_atomic` and CRC32C footers. |
| Repair scope | OK | `repair_best_effort` cleans known temp files and truncates WAL only to safe offset under lock. |
| ContextPack | OK | ContextPack uses AQL retrieve output and reports citation anomalies instead of bypassing policy. |

## Contradictions Found

No code/doc contradiction requiring a format or API change was found in this
pass. The remaining gaps are roadmap items, not hidden contradictions:

- BM25, vector, HNSW, ingestion, SDKs, and distributed consensus remain MVP or
  non-goal areas for Core Alpha.
- ContextPack v0 uses approximate byte-based token estimation.
- Storage formats are versioned by magic/version policy but do not yet include
  forward-compatible field negotiation.

## Full-Stack Regression

`crates/cortex-engine/tests/full_stack_consistency.rs` now verifies the linked
path across write, restart, checkpoint, WAL tail, AQL retrieve, ContextPack,
compact, repair, stats, and validation.
