# CortexDB

[![Rust](https://github.com/AubakirovArman/CortexDB/actions/workflows/rust.yml/badge.svg)](https://github.com/AubakirovArman/CortexDB/actions/workflows/rust.yml)

**CortexDB is an experimental Core Alpha of an agent-native context database.**

> ⚠️ **Warning:** CortexDB is currently in **Core Alpha status** and is suitable for local experiments, research, architecture validation, and early contributors. It is **not recommended for production workloads yet.**

CortexDB is specifically engineered for autonomous AI agents. Unlike traditional databases that return raw rows or tables, or vector databases that return fragmented, unverified text chunks, CortexDB compiles permission-safe, evidence-aware **Context Packs** with strict token-budget limits and deterministic fact verification.

---

## 3-Minute Demo

```bash
# 1. Load a fixture
cargo run -p cortex-cli -- load-fixture examples/datasets/investment_projects ./demo-db

# 2. Search
cargo run -p cortex-cli -- search ./demo-db project:investments "Solar Plant budget"

# 3. Retrieve a ContextPack with anomaly reports
cargo run -p cortex-cli -- context ./demo-db project:investments \
  'RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN default LIMIT 10 CANDIDATES;' --json

# 4. Verify a fact for numeric conflicts
cargo run -p cortex-cli -- verify ./demo-db project:investments \
  'VERIFY FACT "Solar Plant budget is 1.2B KZT" IN BRAIN default;' --json
```

Or run the full demo: `make demo`

---

## Current Core Alpha Features (v0.1.0-core-alpha)

- **Single-Node Durable Storage:** Strict Write-Ahead Log (WAL) with group commit, MVCC MemTable, and incremental check-pointing/compaction.
- **Durable Local Agent Memory:** Scope-isolated agent-facing memory retrieval with dynamic decay/TTL scoring.
- **Deterministic Fact Verification (`VERIFY FACT`):** Heuristic and deterministic numerical and citation checking that detects contradictions before they reach the agent.
- **HTTP Server:** Async HTTP surface over actor-isolated local single-node core built on **Tokio**, **Axum**, and **Tower-HTTP** with strict 2MB body limit boundaries.
- **Crate Ecosystem:** Fully modular workspace crates: `cortex-core`, `cortex-aql`, `cortex-storage`, `cortex-engine`, `cortex-server`, and `cortex-cli`.

## Long-Term Vision (Experimental/Under Active Development)

- **Consensus-Driven Replication (Raft):** Multi-node replication log syncing and leader election (current status: primitive foundations/experimental model).
- **Consistent Hashing Sharding:** Distributed namespace layout and dynamic query routing (current status: experimental layout primitives).
- **Guarded HNSW Approximate Search:** Fixed-point distance metrics (DotProduct, Cosine, L2) with deterministic multi-layer graphs, exact fallback, recall gates, visit-budget limits, SLO reporting, repeatable recall/latency reports, release-mode synthetic/drift/external/metric-matrix fixture gates, an external-corpus harness, and CI report artifacts. Long-running benchmark history remains future work.

---

## Crates

- `crates/cortex-aql`: AQL parser, AST, policy validation, binder, and bitmap VM.
- `crates/cortex-storage`: ACLOG WAL, manifest, segment, bitmap, lexical, vector, and experimental HNSW graph files.
- `crates/cortex-core`: In-memory MVCC MemTable, read transactions, cell versions, and manifest primitives.
- `crates/cortex-engine`: Single-node database loop, compaction, AQL-backed retrieve, memory TTL/decay, source trust, `VERIFY FACT` reports, ContextPack, exact vector search, and experimental HNSW foundations.
- `crates/cortex-sdk`: Blocking Rust HTTP client for the versioned server API, with `cargo package` preflight coverage.
- `crates/cortex-cli`: Command `cortexdb` for local operations and loading fixtures.
- `crates/cortex-server`: Async JSON HTTP API built on Axum and Tokio with per-tenant `DatabaseActor` workers over the local blocking database core.

The current AQL query contract is frozen in [`docs/AQL_V0_4.md`](docs/AQL_V0_4.md).

---

## Dataset Fixtures Pack

CortexDB includes five built-in standard dataset fixtures under `examples/datasets/` for demo scenarios:
- `legal_policies` — compliance auditing scenarios.
- `sec_financial_facts` — financial facts checking.
- `support_tickets` — agent customer support memory.
- `investment_projects` — conflicting budgets verification.
- `world_indicators` — global development statistics.

To populate your database with a dataset fixture:
```bash
cargo run -p cortex-cli -- load-fixture examples/datasets/legal_policies ./data
```

---

## Minimal Engine Example

```rust
use cortex_core::CellId;
use cortex_engine::Database;

let mut db = Database::open("./data")?;
let seq = db.put_cell(CellId(1), b"hello".to_vec())?;
let value = db.get_latest_cell(CellId(1));
assert_eq!(value, Some(b"hello".to_vec()));
# Ok::<(), Box<dyn std::error::Error>>(())
```

---

## Minimal CLI Check

```bash
cargo run -p cortex-cli -- put ./data 1 hello
cargo run -p cortex-cli -- get ./data 1
cargo run -p cortex-cli -- flush ./data
cargo run -p cortex-cli -- stats ./data
cargo run -p cortex-cli -- validate ./data
cargo run -p cortex-cli -- backup ./data ./data.backup
cargo run -p cortex-cli -- restore ./data.backup ./data.restored
cargo run -p cortex-cli -- load-fixture examples/datasets/legal_policies ./data
```

Backup and restore behavior is documented in
[`docs/BACKUP_RESTORE.md`](docs/BACKUP_RESTORE.md).

---

## Minimal HTTP Check

```bash
cargo run -p cortex-server -- ./data 127.0.0.1:8181
curl 'http://127.0.0.1:8181/v1/health'
curl 'http://127.0.0.1:8181/v1/stats'
curl 'http://127.0.0.1:8181/v1/validate'
```

Set `CORTEXDB_AUTH_TOKEN` to require one admin Bearer token, or
`CORTEXDB_AUTH_TOKENS="admin:root-token,data:app-token"` for static
admin/data route roles. Set
`CORTEXDB_ACTOR_QUEUE_CAPACITY` to tune the per-tenant bounded database actor
queue; a full queue returns `503 database_busy` as explicit backpressure.

The HTTP response schema is documented in [`docs/API_JSON_SCHEMAS.md`](docs/API_JSON_SCHEMAS.md)
and the OpenAPI contract is available at [`docs/openapi.yaml`](docs/openapi.yaml).

---

## SDK Quickstart

### Python

```python
from cortexdb_client import CortexDBClient

client = CortexDBClient("http://127.0.0.1:8181")

# Put a cell
client.put_cell(1, "scope=finance\n\nBudget is 450M KZT")

# Search
results = client.search("finance", "budget")
print(results["results"][0]["payload"])

# ContextPack
pack = client.context("finance", 'RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN default LIMIT 5 CANDIDATES;')
print(pack["cells"][0]["payload_text"])
```

### TypeScript

```typescript
import { CortexDBClient } from "@cortexdb/client";

const client = new CortexDBClient("http://127.0.0.1:8181");

await client.putCell(1, "scope=finance\n\nBudget is 450M KZT");
const results = await client.search("finance", "budget");
console.log(results.results[0].payload);
```

### Rust

```rust
use cortex_sdk::CortexDbClient;

let client = CortexDbClient::new("http://127.0.0.1:8181");
client.put_cell_response(1, "scope=finance\n\nBudget is 450M KZT")?;
let results = client.search_keyword_response("finance", "budget", 10)?;
println!("{}", results.results[0].payload);
```

### ContextPack JSON Example

```json
{
  "token_budget_tokens": 4000,
  "estimated_tokens": 2500,
  "truncated": false,
  "citations_required": true,
  "cells": [
    {
      "cell_id": 1,
      "estimated_tokens": 120,
      "citation": "report_q1.pdf#page=3",
      "payload_text": "Solar Plant budget is 1.2B KZT",
      "explain": {
        "score": 95,
        "matched_terms": ["budget", "solar"],
        "why_selected": "high lexical match",
        "base_bm25": 80,
        "source_trust_bonus": 10,
        "redundancy_penalty": 0
      }
    }
  ],
  "anomalies": [
    {
      "cell_id": null,
      "code": "token_overload",
      "message": "Cell exceeds token budget"
    }
  ]
}
```

### VERIFY FACT JSON Example

```json
{
  "verdict": "mixed_evidence",
  "supporting": [
    {
      "cell_id": 1,
      "matched_terms": 3,
      "source_trust_q16": 65535,
      "citation": "report_q1.pdf#page=3",
      "payload_text": "Solar Plant budget is 1.2B KZT"
    }
  ],
  "contradicting": [
    {
      "cell_id": 2,
      "matched_terms": 3,
      "source_trust_q16": 65535,
      "citation": "report_q2.pdf#page=5",
      "payload_text": "Solar Plant budget is 1.4B KZT"
    }
  ],
  "numeric_conflicts": [
    {
      "metric": "budget",
      "left": "1.2B KZT",
      "right": "1.4B KZT"
    }
  ]
}
```

---

## What Is Not Production-Ready Yet

- **BM25 ranking** is heuristic, not production-tuned.
- **HNSW** has guarded production controls, but exact vector scan is still the most predictable default for critical workloads.
- **Replication** is a local consensus model, not a real distributed transport.
- **Ingestion pipelines** are alpha smoke paths, not production document/OCR/API adapters.
- **No built-in LLM integration** — ContextPack is designed to be consumed by external agents.
- **Single-node only** — sharding and multi-node replication are on the long-term roadmap.

The built-in developer console is available at:

```text
http://127.0.0.1:8181/dashboard
```

SDK publication preflight covers Python wheel building, npm package dry-runs,
and Rust `cargo package`:

```bash
make sdk-check
```

Manual tag-gated package publishing is documented in
[`docs/SDK_RELEASE.md`](docs/SDK_RELEASE.md).

---

## Quality & Release Verification Gates

The entire workspace compiles, checks, and formats cleanly under our automated release gate:
```bash
make alpha-check
```

This enforces:
- `cargo check --workspace`
- `cargo test --workspace --all-features` (230+ green tests)
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- Robust Investment Projects demo script completion
