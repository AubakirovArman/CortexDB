# Agent Memory v2

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

TTL is storage policy, not a payload-scan helper. Memory descriptors are indexed
by `cell_id` and `expires_at_unix_seconds`; AQL retrieve applies the lifecycle
filter before payload materialization, and maintenance can tombstone expired
memory through:

```rust
let expired = db.expired_memory_cells(now_unix_seconds);
let tombstoned = db.expire_memory_cells(now_unix_seconds)?;
```

`expire_memory_cells` reads the maintained lifecycle index and writes tombstones
through the normal WAL path. Expired memory is excluded from AQL retrieve even
before the background maintenance pass tombstones it, and tombstones survive
restart through WAL replay.

Decay scoring is also deterministic and fixed-point:

```rust
let scores = db.memory_decay_scores(now_unix_seconds);
```

`freshness_q16` is `Q16_ONE` for permanent memory or memory without enough
timing metadata, decreases linearly over the TTL window, and becomes
`Q16_ZERO` after expiry:

```text
expires_at = created_unix_seconds + ttl_seconds
expired    = now_unix_seconds >= expires_at
freshness  = remaining_ttl / ttl_seconds, encoded as Q16
```

`RankOp` applies the same freshness as a decay multiplier for memory cells, so
two equally relevant temporary memories are ordered by remaining TTL while
permanent memory and non-memory cells keep their normal ranking score.

Feedback is stored as durable `type=feedback` cells and is used as a
deterministic pre-pack ordering signal for ContextPack selection. The current
model supports useful/not-useful votes, per-source-cell scores, fixed-window
decay, stats, and explainable `feedback_bonus` ContextPack score components.

## Epic 141 Agent Memory v2 Contract

| Plan task | Current implementation | Boundary |
| --- | --- | --- |
| Add long-term memory | Permanent memory is represented by `type=memory` cells without a TTL, written through `REMEMBER` and persisted through WAL/checkpoint/replay. | Long-term memory is durable local storage, not a learned profile or autonomous memory manager. |
| Add working memory | Working memory is represented by scoped short-TTL memory cells that can be retrieved into ContextPacks during active tasks. | Working memory is explicit and policy checked; CortexDB does not infer hidden session memory. |
| Add private/shared memory | Private and shared memory are modeled through scopes such as `agent:<id>`, `project:<name>`, and `tenant:<name>`, then enforced by `AgentView` read/write scope policy. | The current boundary is local AgentView enforcement, not enterprise RBAC. |
| Add TTL/decay | TTL expiry uses a descriptor-backed lifecycle index, `expired_memory_cells` / `expire_memory_cells`, query-time lifecycle filtering, rank decay, and fixed-point `memory_decay_scores`. | Expiry is deterministic; semantic decay and learned importance are future ranking work. |
| Add feedback | Feedback is stored as durable `type=feedback` cells, influences ContextPack pre-pack ordering, decays deterministically, and appears as `feedback_bonus` in ContextPack explain output. | Feedback is a deterministic signal, not a reinforcement-learning loop. |

## Memory Classes

### Agent Sessions

Agent sessions are explicit short-lived memory cells with a shared
`session_id`. They are useful for task-local context that should be queryable
during an active workflow without becoming hidden global memory.

```rust
let session = db.start_agent_session(
    &view,
    "agent:finance",
    b"review capex and schedule evidence first",
    3600,
    now,
)?;
db.remember_session_memory(&session, &view, b"temporary note", None, now + 60)?;
let cells = db.retrieve_session_cells(&session.session_id, &view, now + 120);
```

Session cells are stored through the same WAL/MemTable path as other memory
cells. Their payload header includes:

```text
type=memory
memory_type=workflow_result | observation
session_id=agent-7-session-42
session_kind=context | temporary_memory
ttl_seconds=3600
created_unix_seconds=1760000000
```

Retrieval is scope-checked against `AgentView`, filtered by `session_id`, and
excludes cells whose TTL has expired at the requested `now_unix_seconds`.

### Long-Term Memory

Long-term memory is a durable memory cell with no TTL. It survives restart,
checkpoint, and replay like other knowledge cells:

```text
REMEMBER "Prefer cited budget evidence" IN SCOPE project:investments AS TYPE decision;
```

Use it for stable preferences, durable decisions, and reusable workflow facts.

### Working Memory

Working memory is an explicit short-lived memory cell. It is useful for an
active task or session and is removed by the TTL expiry path:

```text
REMEMBER "For this review, compare capex and completion date first"
IN SCOPE agent:finance
AS TYPE workflow_result
TTL 3600 SECONDS;
```

This keeps temporary context queryable while avoiding silent hidden memory.

### Private And Shared Memory

Private/shared behavior comes from scopes:

```text
agent:finance        private agent memory
project:investments  shared project memory
tenant:alpha         tenant-level memory
```

`AgentView` controls which scopes can be read and written, so AQL cannot use
memory outside the caller's allowed boundary.

### TTL, Decay, And Feedback

TTL and decay keep stale memory visible as an explicit policy outcome instead
of a hidden ranking heuristic. The lifecycle index is maintained from typed
descriptors during open/replay, put/patch, and tombstone; lazy payload residency
does not need to load payloads to decide expiry or decay:

```rust
let expired = db.expired_memory_cells(now_unix_seconds);
let tombstoned = db.expire_memory_cells(now_unix_seconds)?;
let scores = db.memory_decay_scores(now_unix_seconds);
```

Feedback is stored durably and can be audited because it is just another
typed cell class linked to the source cell.

Run the end-to-end local memory demo:

```bash
make agent-memory-demo-check
```

or manually:

```bash
examples/demo/agent_memory/run.sh
```

## Implemented

- **Long-term memory** — permanent `type=memory` cells are supported by omitting
  TTL from `REMEMBER`.
- **Working memory** — short-TTL memory cells provide explicit task/session
  memory without hidden state.
- **Private/shared memory** — scope names and `AgentView` policy enforce
  private agent, shared project, and tenant memory boundaries.
- **Automatic background TTL scheduling** — A background task runs every 60s on each
  active tenant database, asks the lifecycle index for expired memory cells, and
  tombstones them via WAL. See `cortex-server/src/lifecycle.rs`
  (background interval loop) and `cortex-engine/src/memory.rs`
  (`expire_memory_cells`).
- **Deterministic decay scoring** — `memory_decay_scores` reports q16 freshness
  without floating-point scoring or payload scans, and `RankOp` applies that
  freshness as a deterministic multiplier for temporary memory cells.
- **Feedback ordering** — ContextPack candidate ordering uses durable feedback
  scores before packing.
- **Agent sessions** — explicit `session_id` memory cells provide bounded
  task-local context, temporary memory, TTL filtering, and session-scoped
  retrieval through the normal WAL/replay path.
- **End-to-end memory demo** — `examples/demo/agent_memory` exercises CLI
  remember, context, and verify flows, while the gate also runs TTL/decay and
  feedback regression tests.

## Not Yet

- Natural-language contradiction extraction.
- Production memory ranking beyond fixed-point decay and feedback ordering.
- Enterprise RBAC-backed memory policy store.
- Autonomous memory synthesis without explicit writes.
