# CortexDB Core Alpha

Core Alpha is the single-node durable database path. It is intentionally smaller
than the full CortexDB roadmap.

## Frozen Scope

The Core Alpha scope is frozen around durable single-node behavior:

```text
Put/Patch/Tombstone
-> WAL append
-> MemTable MVCC
-> checkpoint/compact
-> restart recovery
-> AQL retrieve
-> ContextPack v1
```

The release checklist for tagging this scope is
[`CORE_ALPHA_RELEASE_CHECKLIST.md`](archive/CORE_ALPHA_RELEASE_CHECKLIST.md).

## Included

- `PutCell`, `PatchCell`, and `TombstoneCell`.
- WAL append before MemTable mutation.
- Durable `CommitSeq` in new WAL operation records.
- Restart recovery from checkpoint plus WAL tail.
- Incremental checkpoint into `.acs`, `.acb`, `.aci`, and manifest files.
- Compact into a full visible snapshot segment.
- AQL retrieve over current and persisted candidates.
- Full `CellId` preservation through internal candidate mappings.
- CRC checks for WAL, segment, bitmap index, lexical index, and manifest files.
- CLI and HTTP smoke paths for put/get/flush/compact/stats/validate.
- Explicit stale lock recovery and collect-all validation reports.
- `db.lock` owner metadata with process id and creation timestamp.
- Durable AgentView persistence for local policy objects.
- Best-effort repair for orphan temp cleanup and safe WAL tail truncation.
- ContextPack v1 with stable JSON schema, token budget, source refs, explain
  details, and citation anomaly reporting.
- Persisted `.acv` vector pages for exact vector scan.

## Excluded

- Production BM25 scoring.
- Production HNSW.
- Distributed consensus.
- Document ingestion.
- LLM integration.
