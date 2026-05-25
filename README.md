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
```

## Crates

- `crates/cortex-aql`: AQL parser, AST, policy validation, binder, bitmap bytecode, and mock bitmap VM.
- `crates/cortex-storage`: ACLOG WAL v0 codec, reader recovery scan, and writer actor.
- `crates/cortex-core`: in-memory MVCC MemTable, read transactions, cell versions, and manifest primitives.
- `crates/cortex-engine`: first usable single-node database loop over WAL and MemTable.

Still intentionally out of scope: storage segments, BM25, vector search, HNSW, distributed mode, and server APIs.

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

## Roadmap

| Milestone | Scope |
| --- | --- |
| 0.5 | ACLOG WAL v0 |
| 0.6 | Usable single-node DB loop |
| 0.7 | Patch/tombstone replay hardening |
| 0.8 | Snapshot reads and recovery polish |

## Checks

```bash
cargo check --workspace
cargo test --workspace --all-features
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
```
