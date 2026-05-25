# CortexDB

Agent-native context database prototype for AI agents.

This repository currently implements the first compiler milestone:

```text
AQL string
-> Parser
-> Raw AST
-> Binder
-> BoundRetrievePlan
-> Mock Bitmap VM
-> Candidates_0 mask
```

The current scope is intentionally limited to the `cortex-aql` crate. It does
not implement WAL, storage segments, BM25, vector search, HNSW, or a server.

## Checks

```bash
cargo test
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```
