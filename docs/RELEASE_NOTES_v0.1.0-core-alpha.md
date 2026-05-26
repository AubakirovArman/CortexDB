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
- Server requests route through per-tenant `DatabaseActor` workers.
- Built-in `/dashboard` developer console for operations, cell writes, search,
  ANN evaluation, ingestion, AQL, ContextPack, and VERIFY smoke paths.
- Typed serde JSON response structs for server API payloads.
- OpenAPI 3.1 contract in `docs/openapi.yaml`.
- Python, TypeScript, and Rust SDK clients cover the health/cell/maintenance/AQL/search/context/verify/remember/ingestion API surface.
- SDK publication preflight builds a Python wheel, checks npm package contents, and verifies Rust `cargo package`.
- SDK release workflow runs package preflight on SDK changes and supports
  manual tag-gated publishing for PyPI, npm, and crates.io.
- Experimental replication now enforces AppendEntries previous-log matching,
  conflict suffix truncation, and follower commit-index clamping in local
  transport tests.
- Experimental ANN search now guards HNSW graph usage with exact-vector
  fallback for empty, invalid, or under-returning graph traversals.
- HTTP vector ANN search responses expose `ann_report` so clients can tell
  whether HNSW was used or an exact fallback protected correctness.
- ANN evaluation can compare persisted HNSW results against exact vector scan
  and report recall as fixed-point `recall_q16`.
- ANN evaluation is exposed through `cortexdb search-vector-eval` and
  `POST /v1/search/ann-evaluate`.
- Rust, Python, and TypeScript SDK surfaces include typed ANN evaluation
  responses for package-readiness checks.
- ANN search now has a `low_recall` exact fallback guard while evaluation still
  measures raw HNSW top-k recall.
- Replication consensus state now enforces current-term commit rules and
  majority match-index advancement.
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
workspace check, all-features tests, formatting, clippy with `-D warnings`, SDK
smoke checks, the core benchmark matrix, and the investment projects demo.
