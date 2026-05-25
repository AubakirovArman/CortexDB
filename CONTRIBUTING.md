# Contributing

## Checks

Run these before opening a pull request:

```bash
cargo test --workspace
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
```

## Core Rules

- Do not use floats in core scoring, policy thresholds, or persistent formats.
- Do not silently clamp semantic constraints. Clamp operational limits only when policy says clamp.
- Do not add `unsafe` without a specific design note and review.
- Do not add PostgreSQL, SQLite, Qdrant, ChromaDB, Neo4j, Elasticsearch, or another database as the internal storage backend.
- Keep source files focused. Prefer module splits before a file grows large.
