# Multi-Agent Memory Consistency

`EPIC-F08` defines the single-node consistency contract for multiple agents
sharing CortexDB memory. The scope is deliberately local and auditable: it uses
commit sequences, `AgentView`, and existing WAL/MVCC behavior. It does not call an LLM,
does not create distributed consensus, and does not bypass AgentView.

## Levels

`MemoryConsistencyLevel` has three explicit levels:

- `PrivateReadYourWrites`: an agent's own private scope is visible to that
  agent after its commit sequence and remains hidden from other AgentViews
  unless policy explicitly grants that scope.
- `SharedImmediate`: a shared scope write is visible to any AgentView with that
  readable scope after the write commits on the single node.
- `SharedSequenced`: a handoff packet carries a ContextPack-style `pack_hash`
  plus `pack_seq`, and the receiver treats the packet as visible only after the
  advertised sequence has committed.

## Visibility

Visibility is policy-first. `classify_memory_visibility` reports the viewer,
owner, scope, `scope_id`, readable/writable flags, level, and
`visible_after_seq`. It is a reporting contract; the actual data path still
filters through AQL, AgentView scope checks, ContextPack access decisions, and
descriptor-backed metadata.

Private memory is not a naming convention. It is enforced by readable scopes:
if another AgentView lacks the private scope, broad AQL retrieval and
ContextPack packing cannot surface that memory.

Shared memory is immediate within one database instance. Once a WriteBatch or
agent transaction commits, later reads using an AgentView that can read the
scope may observe it.

## Handoff Packets

`AgentHandoffRequest` models a message from one agent to another:

- `source_agent_id` and `target_agent_id` must match their AgentViews.
- `scope` must be readable by both source and target.
- `pack_hash` identifies the exact context pack or message payload being handed
  off.
- `pack_seq` must not be ahead of `Database::current_seq()`.
- `required_after_seq` must be less than or equal to `pack_seq`.

`Database::plan_agent_handoff` returns an `AgentHandoffReport` with
`SharedSequenced` consistency. It does not write a cell and does not grant
permissions; it only validates that a receiver is allowed to consume a pack for
that shared scope at the advertised sequence.

## Conflict Semantics

Write conflicts use the existing agent transaction semantics:

- same-cell concurrent writes from the same `base_seq` report a structured
  stale-cell conflict;
- tombstones after `base_seq` report tombstone conflicts;
- disjoint shared-scope cells may commit independently;
- all transaction payloads must match the transaction scope.

This keeps F08 aligned with `EPIC-F04` instead of creating a parallel write
protocol.

## Operational Tradeoffs

- Single-node only: the contract does not provide cross-node linearizability.
- No implicit sharing: a handoff packet cannot make a private scope readable.
- Handoff packets are message metadata, not durable memory by themselves.
- Stale readers must compare `pack_seq` and `required_after_seq` before using a
  handed-off pack.
- If a product needs durable handoff history, store the handoff as a normal
  memory cell in a shared scope and keep the same `pack_hash`/`pack_seq`
  metadata.

## Acceptance Check

Run:

```bash
make multi-agent-consistency-check
```

The command writes:

```text
target/multi-agent-consistency/report.json
```

The report records doc marker checks and the focused regression test covering
private visibility, shared immediate visibility, stale same-cell conflicts, and
sequenced handoff validation.
