# CortexDB Comparisons

Status: neutral positioning guide, not a replacement claim.

CortexDB is a local agent-native context database. It is designed to compile
bounded, cited Context Packs and deterministic verification reports for AI-agent
workflows. It should be compared by fit, not by assuming it replaces every
database, vector index, or agent framework.

Run the comparison-doc gate:

```bash
make comparison-docs-check
```

## Epic 138 Comparison Docs v2 Contract

This page closes the comparison-docs epic by covering five adjacent stacks
without replacement claims:

| Epic task | Fit question | CortexDB boundary |
| --- | --- | --- |
| Compare with vector DB | Do you need raw ANN/vector retrieval or governed context output? | CortexDB includes vector foundations, but beta value is policy-filtered ContextPack and VERIFY output. |
| Compare with RAG storage | Do you need a chunk store or a durable context database with explicit retrieval contracts? | CortexDB stores knowledge cells and emits typed, cited context packages; it is not a generic object store. |
| Compare with Postgres | Do you need relational transactions/reporting or agent context assembly over curated cells? | CortexDB should sit beside SQL for agent retrieval, not replace operational SQL. |
| Compare with memory frameworks | Do you need orchestration/session memory or a durable queryable memory backend? | CortexDB is the storage/retrieval layer under agent runtimes, not the agent runtime itself. |
| Compare with document search | Do you need broad portal search/faceting or cited context packs for agent workflows? | CortexDB has search foundations, but dedicated search engines remain the better fit for general search portals. |

## Short Version

| Stack | Strong fit | CortexDB difference | Practical guidance |
| --- | --- | --- | --- |
| PostgreSQL / SQL databases | General relational data, transactions, joins, reporting, operational SQL workloads. | CortexDB focuses on AQL retrieval, evidence-aware ContextPack output, and agent scope policy. | Use CortexDB beside SQL when agents need cited context packs over curated knowledge cells. |
| Vector databases | High-throughput nearest-neighbor retrieval and embedding-centric search. | CortexDB treats vectors as one retrieval signal behind policy, citations, verification, and token-budget packing. | Use vector DBs when raw ANN retrieval is the product; use CortexDB when the output must be a governed context package. |
| Classic RAG stacks | Fast prototyping with chunking, embeddings, and prompt assembly. | CortexDB makes retrieval constraints, citations, anomalies, and verification explicit in typed outputs. | Use CortexDB when repeated agent workflows need durable memory, scoped retrieval, and deterministic evidence fields. |
| Agent memory frameworks | Session memory, tool orchestration, and application-level memory flows. | CortexDB provides a durable database core and query/verification surface rather than agent orchestration. | Use CortexDB as the storage/retrieval layer under agent frameworks. |
| Document search engines | Lexical search, ranking, faceting, observability over large indexed corpora. | CortexDB has search foundations, but the beta product centers on ContextPack generation and verification. | Use dedicated search engines for broad search portals; use CortexDB for context DB workflows. |

## What CortexDB Adds

- durable single-node write path: WAL, MemTable, checkpoint, compact, replay;
- AQL with policy-aware filters and requirement clauses;
- ContextPack output with token budget, citations, anomalies, and explain data;
- deterministic `VERIFY FACT` with source trust and numeric conflict support;
- typed CLI, HTTP, and SDK surfaces with contract checks;
- local validation, backup, audit, and release evidence gates.

## What CortexDB Does Not Claim

CortexDB beta does not claim:

- a full PostgreSQL/SQLite replacement;
- a managed vector database replacement;
- production distributed consensus;
- hosted managed cloud readiness;
- enterprise compliance certification;
- legal-grade verification or legal advice;
- fallback-free production HNSW.

These boundaries are enforced by [`PUBLIC_CLAIMS_POLICY.md`](PUBLIC_CLAIMS_POLICY.md).

## When To Use Together

Common combined architecture:

```text
PostgreSQL / object store / document system
-> ingestion
-> CortexDB knowledge cells
-> AQL retrieval
-> ContextPack / VERIFY FACT
-> agent or application
```

For vector-heavy systems:

```text
embedding provider
-> vector index or CortexDB vector foundations
-> policy-filtered candidates
-> ContextPack packing and verification
```

For agent frameworks:

```text
agent runtime / tools
-> CortexDB retrieve_context / verify_fact / remember
-> prompt or tool decision
```

## Related Docs

- [`WHY_CORTEXDB.md`](WHY_CORTEXDB.md)
- [`RAG_VS_CORTEXDB.md`](RAG_VS_CORTEXDB.md)
- [`CONTEXT_PACK_TECHNOLOGY.md`](CONTEXT_PACK_TECHNOLOGY.md)
- [`BETA_RELEASE.md`](BETA_RELEASE.md)
- [`PUBLIC_CLAIMS_POLICY.md`](PUBLIC_CLAIMS_POLICY.md)
