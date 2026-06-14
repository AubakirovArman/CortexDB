# AQL v0.5 REMEMBER Write Contract

AQL v0.5 keeps the v0.4 grammar and freezes the runtime contract for
`REMEMBER`. Parser, binder, and diagnostics from [`AQL_V0_4.md`](AQL_V0_4.md)
remain valid unless this document states a stricter write-path rule.

## Brain Target

The current data model is single-brain. `default = BrainId(1)` is the only real
brain namespace. Non-default AQL brain identifiers are deprecated aliases for
`BrainId(1)` and do not provide isolation. Migration guidance is in
[`BRAIN_SEMANTICS.md`](BRAIN_SEMANTICS.md).

## REMEMBER

```sql
REMEMBER "Use conservative budget assumptions" IN SCOPE project:investments AS TYPE decision TTL 3600 SECONDS;
```

`TTL <integer> SECONDS` is optional. `AS TYPE <memory_type>` is required.

Supported memory types:

- `decision`
- `preference`
- `workflow_result`
- `error_log`
- `observation`

## Policy

Binding is fail-closed:

- `AgentView.allow_remember` must be `true`;
- target scope must be in `AgentView.writable_scopes`;
- memory type must be in `AgentView.allowed_memory_types`;
- TTL must be less than or equal to `AgentView.max_ttl_seconds` when a maximum
  is configured.

Policy failures deny the write. v0.5 does not silently rewrite scope,
memory type, TTL, or payload text.

## Created Cell

One successful `REMEMBER` creates exactly one durable `KnowledgeCell`.

The encoded payload body is the quoted string content from the AQL statement.
The metadata headers are:

- `scope=<requested scope name>`;
- `status=ready`;
- `type=memory`;
- `memory_type=<requested memory type>`;
- `ttl_seconds=<requested TTL>` only when TTL is present;
- `created_unix_seconds=<current unix seconds>`;
- `source=agent:<AgentView.agent_id>`.

The returned `RememberedCell` exposes the created `cell_id`, write
`commit_seq`, and effective `ttl_seconds`.

## Cell ID Allocation

REMEMBER cell IDs live in the memory namespace:

```text
0x8000_0000_0000_0000 | ((agent_id & 0x7fff_ffff) << 32) | sequence
```

`sequence` is allocated from the manifest-backed `memory_cell_cursors` counter
for the agent slot. The counter is advanced and persisted before the WAL write,
so gaps are allowed after failed or interrupted writes, but IDs are not reused.
When opening an older database without a cursor, the engine initializes from
the highest observed live or tombstoned memory sequence for that agent slot and
then persists the cursor.

Generic ingest allocation uses the manifest-backed `next_cell_id` cursor in the
non-memory ID space. It does not allocate from the high-bit REMEMBER namespace.

## Read/Verify Cycle

REMEMBER cells are ordinary readable memory cells after commit:

- `RETRIEVE CONTEXT ... WHERE type = "memory"` can return them when the scope is
  readable by the agent;
- `VERIFY FACT` may use their text as evidence when the target brain and scope
  are readable and verify policy allows the operation.

## Compatibility Rules

- Do not change REMEMBER metadata fields without a new AQL version document.
- Do not replace manifest-backed allocation with descriptor scans or max-id
  heuristics.
- Add regression tests for policy denials, ID allocation, and
  remember-retrieve-verify behavior when changing this path.
