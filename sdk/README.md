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
ANN evaluation clients expose the `/v1/search/ann-evaluate` contract as a typed
response with exact top-k, ANN top-k, overlap, and fixed-point `recall_q16`.
`ann_report` also exposes `recall_q16`, `min_recall_q16`, `production_safe`,
`fallback_performed`, and `slo_violations` so clients can enforce ANN/HNSW
recall, fallback, and visit-budget guardrails.
All clients support additive tenant/realm scoping so package users can target
the same per-tenant database layout exposed by `cortex-server` and `/dashboard`.
`publish/check.sh` validates Python bytecode/tests/wheel packaging, Rust tests
and `cargo package`, SDK version consistency, tenant routing, ANN evaluation surface presence,
and npm package dry-runs when npm is installed.

Use `make sdk-check` for the local gate. The GitHub `SDK Release` workflow runs
the same preflight on SDK changes and can publish all three packages manually
from a `v*` tag after registry credentials are configured. See
[`docs/SDK_RELEASE.md`](../docs/SDK_RELEASE.md).
