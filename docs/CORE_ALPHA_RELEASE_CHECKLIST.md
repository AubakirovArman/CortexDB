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
| File size | Rust source files stay under 300 lines unless explicitly split next. |
| Storage safety | WAL, segment, bitmap, lexical, and manifest corruption tests pass. |
| Lifecycle safety | open, close, Drop, lock, and stale unlock tests pass. |
| Repair safety | `Database::repair_best_effort` removes orphan temps and truncates only safe WAL tails. |
| Restart safety | put, patch, tombstone, checkpoint, compact, and WAL tail tests pass. |
| Query safety | AQL retrieve respects AgentView masks and candidate mappings. |
| ContextPack v0 | AQL-to-ContextPack tests pass for budget and citation anomalies. |
| Docs | README, Core Alpha docs, invariants, failure scenarios, and task pools are current. |

## Release Command

```bash
git tag -a v0.1.0-core-alpha -m "CortexDB Core Alpha"
git push origin v0.1.0-core-alpha
```

## Explicit Non-Goals For This Tag

- Production BM25 ranking.
- Persistent vector index pages.
- Production HNSW.
- Distributed consensus.
- Document ingestion.
- LLM integration.
