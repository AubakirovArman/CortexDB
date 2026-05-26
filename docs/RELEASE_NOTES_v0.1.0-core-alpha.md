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
- CLI commands for put/get/tombstone/flush/compact/stats/validate/repair/context/AQL/search/verify/remember/ingest/unlock.
- `clap`-backed CLI parser with structured help/version output.
- HTTP API for health, put/get/tombstone/flush/compact/stats/validate/AQL/search/context/verify/remember/ingest.
- Typed serde JSON response structs for server API payloads.
- OpenAPI 3.1 contract in `docs/openapi.yaml`.
- Empty ingestion safety for text, JSON, and CSV inputs.
- Crash, restart, corruption, lifecycle, repair, AQL retrieve, and storage validation tests.

## Explicit Non-Goals

- Production BM25 ranking.
- Persistent approximate vector indexes beyond exact `.acv` scan.
- Production HNSW.
- Real distributed consensus or replication transport.
- Production document/OCR/API ingestion pipelines; current ingestion adapters are alpha smoke paths.
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

Latest local evidence: `make alpha-check` passed on 2026-05-26, including
workspace check, all-features tests, formatting, clippy with `-D warnings`, the
core benchmark matrix, and the investment projects demo.
