# Architecture

Current CortexDB scope covers the AQL compiler, mock bitmap execution layer,
ACLOG WAL v0, and an in-memory MVCC core.

```text
AQL string
-> Parser
-> Raw AST
-> Binder
-> BoundRetrievePlan
-> Bitmap VM
-> Candidates_0 mask
-> ACLOG WAL
-> MemTable
```

## Crates

- `crates/cortex-aql`: parser, AST, policy validation, binder, bitmap bytecode, and mock bitmap VM.
- `crates/cortex-core`: in-memory MVCC core skeleton, read transactions, cell versions, and manifest primitives.
- `crates/cortex-storage`: ACLOG WAL v0 codec, reader recovery scan, and writer actor.

## Safety Boundary

AQL filters may only narrow the `AgentView` allowed mask. Binder starts retrieval bytecode with
`PushAgentAllowed`, `PushLive`, and `And`, then intersects any compiled `WHERE` expression.

`BitmapOp::Not` is evaluated as a complement inside the segment-local universe. It is not a
permission expansion mechanism because the final plan remains intersected with the agent mask.
