# Architecture

Current CortexDB scope is the AQL compiler and mock bitmap execution layer.

```text
AQL string
-> Parser
-> Raw AST
-> Binder
-> BoundRetrievePlan
-> Bitmap VM
-> Candidates_0 mask
```

## Crates

- `crates/cortex-aql`: parser, AST, policy validation, binder, bitmap bytecode, and mock bitmap VM.
- `crates/cortex-storage`: storage skeleton for the next WAL milestone. WAL is not implemented yet.

## Safety Boundary

AQL filters may only narrow the `AgentView` allowed mask. Binder starts retrieval bytecode with
`PushAgentAllowed`, `PushLive`, and `And`, then intersects any compiled `WHERE` expression.

`BitmapOp::Not` is evaluated as a complement inside the segment-local universe. It is not a
permission expansion mechanism because the final plan remains intersected with the agent mask.
