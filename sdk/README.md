# CortexDB SDK Surface

This directory contains minimal HTTP client sketches for early integration tests.
They target the existing `cortex-server` endpoints and intentionally avoid
extra runtime dependencies.

- `python/cortexdb_client.py`: stdlib Python client.
- `typescript/cortexdb-client.ts`: fetch-based TypeScript client.
- `python/pyproject.toml` and `typescript/package.json`: early package metadata.

The stable public SDK APIs are not frozen yet.
