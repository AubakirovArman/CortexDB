# RAG vs CortexDB

This document is a practical comparison for teams moving from classic RAG setups to CortexDB.

## Classic RAG pipeline

Typical stack:

1. chunk documents,
2. embed + vector search,
3. attach raw snippets to prompt.

Typical pain points:

- duplicate/overlapping snippets,
- missing provenance in the returned context,
- token budget overruns,
- weak policy enforcement for agent scopes,
- no built-in fact contradiction signal.

## CortexDB pipeline

1. ingest to durable `CellId` payloads with metadata (`scope`, `status`, `type`, etc.),
2. query via AQL + bitmap/metadata filters,
3. retrieve candidate cells,
4. compile ContextPack under token budget,
5. optionally run `VERIFY FACT` for conflict checks,
6. return structured response (`cells`, `token_budget`, `anomalies`, `citations`).

## Why this is different

- **Budget-aware outputs**: ContextPack is bounded by token budget up front.
- **Policy-aware retrieval**: AgentView and scope filters are applied before candidate emission.
- **Evidence-aware output**: citations and anomaly fields are explicit in response.
- **Operational safety**: same durable core and replay model supports restart + checkpoint/compact + validation.

## When CortexDB is a better fit

Use CortexDB when you need:

- strict tenant/agent scoping,
- deterministic reproducibility and policy enforcement,
- fact verification before forwarding context to LLM,
- explicit source traceability and anomaly reporting.

Use classic RAG services when:

- raw nearest-neighbor retrieval latency is the primary goal,
- provenance/citation loops are handled elsewhere,
- no durable local agent-memory layer is required.

## Practical migration

1. Start with dataset fixture (`examples/datasets/*`) and load with CLI.
2. Replace pure vector retrieval with `RETRIEVE CONTEXT` queries.
3. Add `VERIFY FACT` for riskier claim-heavy calls.
4. Tune `ContextPack` budget and required fields by route.

```bash
cargo run -p cortex-cli -- load-fixture examples/datasets/investment_projects ./demo-db
cargo run -p cortex-cli -- context ./demo-db investment_projects 'RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN default LIMIT 10 CANDIDATES;'
cargo run -p cortex-cli -- verify ./demo-db investment_projects 'VERIFY FACT "Solar Plant budget is 1.2B KZT" IN BRAIN default;'
```

For a full demo flow, use `make demo` and `examples/rag_demo/run.sh`.
