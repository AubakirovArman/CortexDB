# CortexDB

[![Rust](https://github.com/AubakirovArman/CortexDB/actions/workflows/rust.yml/badge.svg)](https://github.com/AubakirovArman/CortexDB/actions/workflows/rust.yml)

**CortexDB is an Enterprise-Grade, Production-Ready Agent-Native Context Database.** 

CortexDB is specifically engineered for autonomous AI agents. Unlike traditional databases that return raw rows or tables, or vector databases that return fragmented, unverified text chunks, CortexDB returns permission-safe, evidence-aware **Context Packs** with strict token-budget limits and deterministic fact verification.

---

## Key Features

- **High-Performance Async Network Layer:** Built on **Tokio**, **Axum**, and **Tower-HTTP** with strict 2MB body limit enforcement and zero-downtime shared database state routing.
- **Multilingual Search Engine (BM25 v2 & HNSW):** Full Kazakh, Russian, and English Unicode tokenization, BM25 v2 relevance ranking, real-time HNSW vector indexing, and Reciprocal Rank Fusion (RRF) hybrid search.
- **Anti-Hallucination Fact Verification (`VERIFY FACT`):** Header-aware structured numeric and citation guards that identify numerical contradictions on the fly, return verification status (supported, contradicted, mixed, insufficient), and compile structured `VerificationReport v2` responses.
- **Consensus-Driven Replication (Raft):** Out-of-the-box multi-node replication log syncing, Leader/Follower elections, heartbeats, and database snapshot transfers.
- **Consistent Hashing Sharding:** Distributed layout configuration that dynamically routes read and write requests to respective replicas.

---

## Crates

- `crates/cortex-aql`: AQL parser, AST, policy validation, binder, and bitmap VM.
- `crates/cortex-storage`: ACLOG WAL, manifest, segment, bitmap, lexical, vector, and HNSW graph files.
- `crates/cortex-core`: In-memory MVCC MemTable, read transactions, cell versions, and manifest primitives.
- `crates/cortex-engine`: Single-node database loop, compaction, AQL-backed retrieve, memory TTL/decay, source trust, `VERIFY FACT` reports, ContextPack, and HNSW-backed vector search.
- `crates/cortex-cli`: Command `cortexdb` for local operations and loading fixtures.
- `crates/cortex-server`: High-performance asynchronous JSON HTTP API built on Axum and Tokio.

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
