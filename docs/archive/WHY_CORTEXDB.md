# Why CortexDB

CortexDB is for AI-agent workflows where **correctness, bounded context, and evidence** are first-class concerns.

A traditional stack often gives you one of these, not all:

- **Postgres/SQL:** strong correctness and transactions, but no native semantic retrieval for agents and no built-in context-budgeted outputs.
- **Vector DB:** fast semantic recall, but weak provenance, weak citation guarantees, and weak policy context.
- **Agent framework memory buffers:** easy to build, but often ephemeral and hard to make durable with auditability.

CortexDB’s design choice is to combine:

1. **Durable single-node storage** (WAL + MVCC + checkpoint/compact + recovery) so data is not lost after restart.
2. **AQL** for declarative retrieval constraints (`scope`, `status`, `type`, numeric thresholds, `LIMIT`, `REQUIRE`).
3. **ContextPack** as the retrieval output shape, not raw nearest rows/chunks.
4. **Permission-safe execution** (`AgentView`) and policy clamping.
5. **Deterministic verification** (`VERIFY FACT`) for evidence-aware confidence, with explicit mixed/insufficient verdicts.
6. **Typed APIs** (CLI, HTTP, SDK) with snapshot contract checks.

## What CortexDB is trying to be

`agent-native context compiler`, not a direct replacement for a full RDBMS or a managed cloud vector service.

- It returns a **bounded context package** ready for LLM consumption.
- It keeps provenance/citation boundaries visible in the returned payload.
- It tracks candidate selection and policy effects for explainability.

## What it is not (yet)

- not production-distributed Raft cluster DB,
- not enterprise secret-management platform,
- not fully production-grade HNSW benchmark-tuned for arbitrary data scales,
- not a drop-in replacement for all analytics/query workloads.

For the current boundary, see:

- `ARCHITECTURE.md`
- `WHY_AGENT_NATIVE_DB.md`
- `README.md` (status + limits)
- `CORE_ALPHA.md`
