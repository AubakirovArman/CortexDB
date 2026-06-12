# CortexDB Public Beta Landing

Status: concise external landing path for `v0.2.0-beta.1`.

## One-liner

CortexDB is a local single-node, agent-native context database for building
evidence-aware Context Packs.

## Value Proposition

For local developer and API evaluation, CortexDB combines durable local storage,
AQL retrieval, search, deterministic verification, and typed HTTP/CLI/SDK
surfaces. The goal is to return compact, cited, permission-aware context for an
agent instead of raw rows or unverified text fragments.

## Quickstart

Clone the repo and run the beta evidence and demo gates:

```bash
git clone https://github.com/AubakirovArman/CortexDB.git
cd CortexDB
make beta-release-check
make demo
```

## Demo

Minimal local loop:

```bash
cargo run -p cortex-cli -- load-fixture ./demo-db examples/datasets/investment_projects
cargo run -p cortex-cli -- search ./demo-db project:investments "solar budget"
cargo run -p cortex-cli -- context ./demo-db project:investments \
  'RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN default LIMIT 10 CANDIDATES;' --json
cargo run -p cortex-cli -- verify ./demo-db project:investments \
  'VERIFY FACT "Solar Plant budget is 1.2B KZT" IN BRAIN default;' --json
```

HTTP surface:

```bash
cargo run -p cortex-server -- ./demo-db 127.0.0.1:8181
curl http://127.0.0.1:8181/v1/health
curl http://127.0.0.1:8181/v1/compatibility
```

## Architecture Diagram

```text
CLI / HTTP / SDK
      |
      v
Database actor
      |
      +--> AQL parser/binder --> bitmap filters
      +--> retrieval/search --> ContextPack builder
      +--> VERIFY FACT --> citation and numeric checks
      |
      v
WAL -> MemTable MVCC -> checkpoint/compact -> segments + indexes
```

## Beta Scope

The beta is for local developer/API evaluation:

- ContextPack generation with token budget, citations, anomalies, and explain
  metadata;
- deterministic `VERIFY FACT` with citation and numeric conflict checks;
- single-node durable storage with WAL, MemTable, checkpoint, compact, and
  validation;
- typed HTTP API, CLI, Rust SDK, Python SDK, and TypeScript SDK examples;
- guarded exact/vector/HNSW foundations with recall gates and fallback policy.

## Limitations

CortexDB beta does not claim:

- production multi-node consensus or failover;
- managed cloud operations;
- enterprise RBAC/compliance certification;
- fallback-free production HNSW;
- built-in LLM inference;
- legal-grade verification or legal advice.

Those topics remain future milestones with explicit design gates.

## Evidence To Review

| Area | Local gate or doc |
| --- | --- |
| Beta release | `make beta-release-check`, [`BETA_RELEASE.md`](BETA_RELEASE.md) |
| API contract | `make openapi-contract-check`, [`openapi.yaml`](../openapi.yaml) |
| SDK contract | `make sdk-e2e-release-check`, [`SDK_RELEASE.md`](SDK_RELEASE.md) |
| Context quality | `make context-pack-quality-check` |
| Verification quality | `make verification-quality-check` |
| Retrieval quality | `make retrieval-quality-check` |
| Operations | `make operations-runbook-check`, [`OPERATIONS.md`](../OPERATIONS.md) |
| Dashboard | `make dashboard-product-check`, [`DASHBOARD_UI.md`](DASHBOARD_UI.md) |

## First Contribution Path

1. Run `make check` and `make beta-release-check`.
2. Pick one bounded surface: AQL, ContextPack, verification, SDKs, docs, or
   dashboard.
3. Add or update the matching evidence gate.
4. Keep public claims aligned with [`PUBLIC_CLAIMS_POLICY.md`](../PUBLIC_CLAIMS_POLICY.md).

## Links

- Architecture: [`ARCHITECTURE.md`](../ARCHITECTURE.md)
- Context Packs: [`CONTEXT_PACK_TECHNOLOGY.md`](CONTEXT_PACK_TECHNOLOGY.md)
- API schemas: [`API_JSON_SCHEMAS.md`](../API_JSON_SCHEMAS.md)
- Release notes: [`RELEASE_NOTES_v0.2.0-beta.1.md`](../RELEASE_NOTES_v0.2.0-beta.1.md)
- Documentation map: [`DOCUMENTATION_INDEX.md`](../DOCUMENTATION_INDEX.md)
