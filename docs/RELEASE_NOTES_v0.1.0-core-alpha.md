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
- ContextPack v1 with stable JSON schema, token budget, source refs, explain details, and citation anomaly reporting.
- CLI commands for put/get/tombstone/flush/compact/stats/validate/repair/context/AQL/search/verify/remember/ingest/unlock.
- `clap`-backed CLI parser with structured help/version output.
- HTTP API for health, put/get/tombstone/flush/compact/stats/validate/AQL/search/context/verify/remember/ingest.
- Server requests route through per-tenant `DatabaseActor` workers.
- Built-in `/dashboard` developer console for operations, cell writes, search,
  ANN evaluation, ingestion, AQL, ContextPack, and VERIFY smoke paths.
- Dashboard requests can target per-tenant realms, keep a short request history,
  and inspect persisted ingestion job progress.
- Dashboard markup, style, and behavior are split into separate server modules
  so the built-in console can grow without a large monolithic raw string.
- Typed serde JSON response structs for server API payloads.
- OpenAPI 3.1 contract in `docs/openapi.yaml`.
- Python, TypeScript, and Rust SDK clients cover the health/cell/maintenance/AQL/search/context/verify/remember/ingestion API surface.
- Python, TypeScript, and Rust SDK clients support tenant/realm routing through
  additive builder helpers.
- SDK publication preflight builds a Python wheel, checks npm package contents, and verifies Rust `cargo package`.
- SDK release workflow runs package preflight on SDK changes and supports
  manual tag-gated publishing for PyPI, npm, and crates.io.
- Release workflow builds the ANN smoke baseline package and attaches its
  checksum-backed `.tar.gz` to GitHub Releases for `v*` tags.
- Experimental replication now enforces AppendEntries previous-log matching,
  conflict suffix truncation, and follower commit-index clamping in local
  transport tests.
- Guarded HNSW approximate search (alpha): fixed-point distance metrics
  (DotProduct, Cosine, L2) with exact-vector fallback for empty, invalid,
  under-returning, or low-recall graph traversals. Not production-grade yet.
- Cosine similarity uses integer-only fixed-point `u128::isqrt()` — no `f64`
  arithmetic in the scoring path.
- Vector dimension mismatch on the write path returns
  `EngineError::VectorDimensionMismatch` instead of being silently skipped.
- HTTP vector ANN search responses expose `ann_report` so clients can tell
  whether HNSW was used or an exact fallback protected correctness.
- ANN evaluation can compare persisted HNSW results against exact vector scan
  and report recall as fixed-point `recall_q16`.
- ANN evaluation is exposed through `cortexdb search-vector-eval` and
  `POST /v1/search/ann-evaluate`.
- Rust, Python, and TypeScript SDK surfaces include typed ANN evaluation
  responses for package-readiness checks.
- ANN search now has a `low_recall` exact fallback guard (75% production default)
  while evaluation still measures raw HNSW top-k recall.
- ANN reports expose `recall_q16` and `min_recall_q16` when the top-k recall
  guard runs, so clients can inspect HNSW quality decisions.
- HNSW integrity reports now cross-check persisted graph links against vector
  candidates during storage validation.
- Persisted `.acv` validation checks vector dimensions so ANN/exact scoring
  cannot silently compare only shared vector prefixes.
- Persisted `.ach` files store `dimension` and `metric` metadata with
  backward-compatible decode.
- Replication consensus state now enforces current-term commit rules and
  majority match-index advancement.
- Replication followers reject non-contiguous AppendEntries batches and
  conflicting same-term leaders in the local consensus model.
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
make release-check
```

which includes:
- `cargo check --workspace` with `-D warnings`
- `cargo test --workspace --all-features` with `-D warnings`
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- ANN baseline release package generation
- SDK preflight checks
- OpenAPI coverage and contract validation
- SDK contract validation (Python + TypeScript smoke tests against live server)
- Core benchmark matrix
- CLI smoke tests
- Server smoke tests
- Investment projects demo (search + AQL + ContextPack + Verify)

and after the GitHub Actions `Rust` workflow is green on stable and beta.

Latest local evidence: `make release-check` passed on 2026-05-29, including
workspace check, all-features tests, formatting, clippy with `-D warnings`, SDK
smoke checks, OpenAPI contract validation, SDK contract validation, the core
benchmark matrix, CLI smoke tests, server smoke tests, API snapshot tests, and
the upgraded investment projects demo (search + AQL + ContextPack + Verify with
numeric conflict detection).
