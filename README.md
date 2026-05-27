# CortexDB

[![Rust](https://github.com/AubakirovArman/CortexDB/actions/workflows/rust.yml/badge.svg)](https://github.com/AubakirovArman/CortexDB/actions/workflows/rust.yml)

**CortexDB is an experimental Core Alpha of an agent-native context database.** 

> ⚠️ **Warning:** CortexDB is currently in **Core Alpha status** and is suitable for local experiments, research, architecture validation, and early contributors. It is **not recommended for production workloads yet.**

CortexDB is specifically engineered for autonomous AI agents. Unlike traditional databases that return raw rows or tables, or vector databases that return fragmented, unverified text chunks, CortexDB compiles permission-safe, evidence-aware **Context Packs** with strict token-budget limits and deterministic fact verification.

---

## Current Core Alpha Features (v0.1.0-core-alpha candidate)

- **Single-Node Durable Storage:** Strict Write-Ahead Log (WAL) with group commit, MVCC MemTable, and incremental check-pointing/compaction.
- **Durable Local Agent Memory:** Scope-isolated agent-facing memory retrieval with dynamic decay/TTL scoring.
- **Deterministic Fact Verification (`VERIFY FACT`):** Heuristic and deterministic numerical and citation checking that detects contradictions before they reach the agent.
- **HTTP Server:** An async HTTP surface over actor-isolated local core built on **Tokio**, **Axum**, and **Tower-HTTP** with strict 2MB body limit boundaries.
- **Crate Ecosystem:** Fully modular workspace crates: `cortex-core`, `cortex-aql`, `cortex-storage`, `cortex-engine`, `cortex-server`, and `cortex-cli`.

## Long-Term Vision (Experimental/Under Active Development)

- **Consensus-Driven Replication (Raft):** Multi-node replication log syncing and leader election (current status: primitive foundations/experimental model).
- **Consistent Hashing Sharding:** Distributed namespace layout and dynamic query routing (current status: experimental layout primitives).
- **Real-Time Vector Indexing (HNSW):** Dynamic vector inserts and graph maintenance directly inside MemTable (current status: early experimental indexes).

---

## Crates

- `crates/cortex-aql`: AQL parser, AST, policy validation, binder, and bitmap VM.
- `crates/cortex-storage`: ACLOG WAL, manifest, segment, bitmap, lexical, vector, and experimental HNSW graph files.
- `crates/cortex-core`: In-memory MVCC MemTable, read transactions, cell versions, and manifest primitives.
- `crates/cortex-engine`: Single-node database loop, compaction, AQL-backed retrieve, memory TTL/decay, source trust, `VERIFY FACT` reports, ContextPack, exact vector search, and experimental HNSW foundations.
- `crates/cortex-sdk`: Blocking Rust HTTP client for the versioned server API, with `cargo package` preflight coverage.
- `crates/cortex-cli`: Command `cortexdb` for local operations and loading fixtures.
- `crates/cortex-server`: Async JSON HTTP API built on Axum and Tokio with per-tenant `DatabaseActor` workers over the local blocking database core.

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
cargo run -p cortex-cli -- load-fixture examples/datasets/legal_policies ./data
```

---

## Minimal HTTP Check

```bash
cargo run -p cortex-server -- ./data 127.0.0.1:8181
curl 'http://127.0.0.1:8181/v1/health'
curl 'http://127.0.0.1:8181/v1/stats'
curl 'http://127.0.0.1:8181/v1/validate'
```

The HTTP response schema is documented in [`docs/API_JSON_SCHEMAS.md`](docs/API_JSON_SCHEMAS.md)
and the OpenAPI contract is available at [`docs/openapi.yaml`](docs/openapi.yaml).

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
