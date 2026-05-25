# CortexDB v0.1.0-core-alpha Release Notes

Core Alpha is the first durable single-node CortexDB prototype.

## Included

- AQL parser, binder, bitmap bytecode, and mock bitmap VM.
- AgentView policy checks and runtime AgentAllowed masks.
- ACLOG WAL v0 with CRC32C, strict recovery, and best-effort safe offsets.
- MemTable MVCC with put, patch, tombstone, snapshot reads, and stats.
- `Database::open`, write path, replay, checkpoint, compact, repair, stats, and validation.
- `.acs`, `.acb`, `.aci`, and `.acm` files with atomic writes and CRC32C footers.
- Candidate mapping that preserves full `CellId(u64)`.
- ContextPack v0 with token budget and citation anomaly reporting.
- CLI commands for put/get/tombstone/flush/compact/stats/validate/repair/context/unlock.
- Minimal HTTP API for health, put/get/tombstone/flush/compact/stats/validate/context.
- Crash, restart, corruption, lifecycle, repair, AQL retrieve, and storage validation tests.

## Explicit Non-Goals

- Production BM25 ranking.
- Persistent vector index pages.
- Production HNSW.
- Real distributed consensus or replication transport.
- Document/PDF/API ingestion.
- LLM integration.

## Release Gates

The tag may be pushed only after:

```bash
cargo fmt --check
RUSTFLAGS="-D warnings" cargo check --workspace
RUSTFLAGS="-D warnings" cargo test --workspace --all-features
RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets -- -D warnings
```

and after the GitHub Actions `Rust` workflow is green on stable and beta.
