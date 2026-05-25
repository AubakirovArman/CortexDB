# Agent Memory v0

Agent memory now has a minimal durable path:

```text
AQL REMEMBER
-> parser
-> binder and AgentView policy
-> KnowledgeCellType::Memory
-> WAL append
-> MemTable update
-> restart replay
-> AQL RETRIEVE by scope/type/memory_type
```

`Database::remember_aql` accepts only `REMEMBER` statements. It reuses the AQL
binder, so the write is denied unless the `AgentView` allows remember, owns the
target write scope, allows the memory type, and accepts the requested TTL.
The same path is exposed for smoke usage through:

```text
cortexdb remember <path> <scope> '<REMEMBER ...;>'
POST /v1/remember?scope=<scope>
```

The current storage bridge still writes metadata as payload header lines:

```text
scope=project:investments
status=ready
type=memory
memory_type=decision
ttl_seconds=60
created_unix_seconds=1760000000
source=agent:1

body bytes...
```

TTL enforcement is available through:

```rust
let expired = db.expired_memory_cells(now_unix_seconds);
let tombstoned = db.expire_memory_cells(now_unix_seconds)?;
```

`expire_memory_cells` writes tombstones through the normal WAL path.

## Not Yet

- Memory decay scoring.
- AgentView persistence.
- VERIFY FACT contradiction detection.
- Feedback loop storage.
