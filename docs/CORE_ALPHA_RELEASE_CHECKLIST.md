# Core Alpha Release Checklist

This checklist gates the `v0.1.0-core-alpha` tag. A release is allowed only when
every required row is green on `main`.

## Required Gates

| Gate | Required evidence |
| --- | --- |
| Workspace CI | GitHub Actions `Rust` workflow passes on stable and beta. |
| Local check | `RUSTFLAGS="-D warnings" cargo check --workspace` passes. |
| Local tests | `RUSTFLAGS="-D warnings" cargo test --workspace --all-features` passes. |
| Formatting | `cargo fmt --check` passes. |
| Lints | `RUSTFLAGS="-D warnings" cargo clippy --workspace --all-targets -- -D warnings` passes. |
| File size audit | All checked-in Rust source and integration test files stay under 300 lines. |
| SDK smoke | `./sdk/publish/check.sh` passes Python bytecode, Python SDK unit tests, and TypeScript package dry-run. |
| Storage safety | WAL, segment, bitmap, lexical, and manifest corruption tests pass. |
| Lifecycle safety | open, close, Drop, lock, and stale unlock tests pass. |
| Repair safety | `Database::repair_best_effort` removes orphan temps and truncates only safe WAL tails. |
| Restart safety | put, patch, tombstone, checkpoint, compact, and WAL tail tests pass. |
| Crash matrix | Orphan bundles, temp manifests, restart tails, and corruption matrix tests pass. |
| Consistency audit | `CORE_CONSISTENCY_AUDIT.md` is current and full-stack consistency tests pass. |
| Atomic audit | `ATOMIC_WRITE_AUDIT.md` and `STORAGE_FORMATS.md` match the current writers/readers. |
| Benchmark baseline | `cargo bench -p cortex-engine --bench core_baseline` runs without external services. |
| ANN fixture gate | `make ann-fixture-check`, `make ann-drift-check`, and `make ann-external-check` pass; CI uploads all `target/ann/*report.json` files. |
| Query safety | AQL retrieve respects AgentView masks and candidate mappings. |
| ContextPack v0 | AQL-to-ContextPack tests pass for budget and citation anomalies. |
| Docs | README, Core Alpha docs, invariants, failure scenarios, and task pools are current. |

## Release Command

```bash
git tag -a v0.1.0-core-alpha -m "CortexDB Core Alpha"
git push origin v0.1.0-core-alpha
```

## Latest Local Gate Evidence

On 2026-05-26, `make alpha-check` passed locally. That covered workspace check,
all-features tests, formatting, clippy with `-D warnings`, SDK smoke checks, the
core benchmark matrix, and the investment projects demo script.

## Explicit Non-Goals For This Tag

- Production BM25 ranking.
- Persistent approximate vector indexes beyond exact `.acv` scan.
- Production HNSW.
- Distributed consensus.
- Document ingestion.
- LLM integration.
