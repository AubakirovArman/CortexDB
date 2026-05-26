# CortexDB SDK Surface

This directory contains minimal HTTP client sketches for early integration tests.
They target the existing `cortex-server` endpoints and intentionally avoid
extra runtime dependencies.

- `python/cortexdb_client.py`: stdlib Python client.
- `typescript/cortexdb-client.ts`: fetch-based TypeScript client.
- `crates/cortex-sdk`: blocking Rust HTTP client.
- `python/pyproject.toml` and `typescript/package.json`: early package metadata.

The SDK APIs are Core Alpha contracts and may still receive additive changes.
