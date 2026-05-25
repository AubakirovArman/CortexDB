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
```

## Crates

- `crates/cortex-aql`: AQL parser, AST, policy validation, binder, bitmap bytecode, and mock bitmap VM.
- `crates/cortex-storage`: ACLOG WAL v0 codec, reader recovery scan, and writer actor.
- `crates/cortex-core`: in-memory MVCC MemTable, read transactions, cell versions, and manifest primitives.

Still intentionally out of scope: storage segments, BM25, vector search, HNSW, distributed mode, and server APIs.

## Checks

```bash
cargo check --workspace
cargo test --workspace --all-features
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
```
