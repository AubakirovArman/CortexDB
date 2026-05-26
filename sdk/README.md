# CortexDB SDK Surface

This directory contains Core Alpha HTTP clients for early integrations and
package publication dry-runs. They target the stable `/v1/*` `cortex-server`
surface and intentionally keep runtime dependencies small.

- `python/cortexdb_client.py`: stdlib Python client with PyPI metadata.
- `typescript/cortexdb-client.js`: fetch-based JavaScript runtime entrypoint.
- `typescript/cortexdb-client.d.ts`: TypeScript declarations.
- `typescript/cortexdb-client.ts`: TypeScript source reference.
- `crates/cortex-sdk`: blocking Rust HTTP client with crates.io metadata.

The SDK APIs are Core Alpha contracts and may still receive additive changes.
Search clients expose a typed response shape with `search_mode`, `ann_report`,
and ranked `results` so ANN fallback behavior is visible to callers.
`publish/check.sh` validates Python bytecode/tests/wheel packaging, Rust tests
and `cargo package`, and npm package dry-runs when npm is installed.
