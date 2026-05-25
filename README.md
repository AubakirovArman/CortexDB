# CortexDB

[![Rust](https://github.com/AubakirovArman/CortexDB/actions/workflows/rust.yml/badge.svg)](https://github.com/AubakirovArman/CortexDB/actions/workflows/rust.yml)

Agent-native context database prototype for AI agents.

This repository currently implements the compiler and first storage milestones:

```text
AQL string
-> Parser
-> Raw AST
-> Binder
-> BoundRetrievePlan
-> Mock Bitmap VM
-> Candidates_0 mask
```

The storage path has started with ACLOG WAL v0 and an in-memory MVCC core:

```text
AQL -> Binder -> Bitmap VM -> WAL -> MemTable -> Segments
PutCell -> WAL append -> MemTable update -> restart -> WAL replay -> Retrieve
Checkpoint -> .acs segment + .acb bitmap index + .aci lexical index + manifest
```

## Crates

- `crates/cortex-aql`: AQL parser, AST, policy validation, binder, bitmap bytecode, and mock bitmap VM.
- `crates/cortex-storage`: ACLOG WAL v0, manifest, segment, bitmap-index, and lexical-index files.
- `crates/cortex-core`: in-memory MVCC MemTable, read transactions, cell versions, and manifest primitives.
- `crates/cortex-engine`: single-node database loop, checkpoint, AQL-backed retrieve, and MVP search helpers.
- `crates/cortex-cli`: minimal `cortexdb` command for local put/get/tombstone/flush checks.
- `crates/cortex-server`: minimal HTTP API for put/get/tombstone/flush checks.

BM25, vector search, HNSW, distributed placement, and server APIs exist as MVP foundations.
They are not production-grade ranking, ANN, consensus, or service layers yet.

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

## Minimal CLI Check

```bash
cargo run -p cortex-cli -- put ./data 1 hello
cargo run -p cortex-cli -- get ./data 1
cargo run -p cortex-cli -- flush ./data
```

## Minimal HTTP Check

```bash
cargo run -p cortex-server -- ./data 127.0.0.1:8080
curl -X POST 'http://127.0.0.1:8080/put?cell_id=1' --data-binary 'hello'
curl 'http://127.0.0.1:8080/get?cell_id=1'
```

## Roadmap

| Milestone | Scope |
| --- | --- |
| 0.5 | ACLOG WAL v0 |
| 0.6 | Usable single-node DB loop |
| 0.7 | Segment/index integration |
| 0.8 | Snapshot reads and recovery polish |
| 0.9 | Query/search/server MVP foundations |

## Checks

```bash
cargo check --workspace
cargo test --workspace --all-features
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
```
