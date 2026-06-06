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
`ann_report` also exposes `recall_q16`, `min_recall_q16`,
`hnsw_ef_construction`, `production_safe`, `fallback_performed`, and
`slo_violations` so clients can enforce ANN/HNSW recall, fallback, graph-shape,
and visit-budget guardrails.
All clients support additive tenant/realm scoping so package users can target
the same per-tenant database layout exposed by `cortex-server` and `/dashboard`.
All clients also expose AQL builder helpers for `RETRIEVE CONTEXT`,
`VERIFY FACT`, and `REMEMBER` so common integrations do not have to assemble
query strings by hand.
`publish/check.sh` validates Python bytecode/tests/wheel packaging, Rust tests
and `cargo package`, SDK version consistency, tenant routing, ANN evaluation surface presence,
and npm package dry-runs when npm is installed.
`make sdk-contract-check` validates live API compatibility by building the
current `cortex-server` binary and running Python, TypeScript, and Rust SDK
smoke tests against real `/v1/*` responses. The smoke contract covers health,
put/get, search, stats, validate, AQL, Context Pack, Verify Fact, Remember,
ingest text, and structured error decoding for `invalid_aql`, `not_found`, and
`invalid_tenant`.
`sdk/release-manifest.json` records the package names, registries, dry-run
commands, deprecation policy, and manual tag-gated publish policy.
`make sdk-release-contract-check` validates that lifecycle contract without
building packages.
`make sdk-release-artifacts-check` packages Rust, Python, and TypeScript
examples into a checksummed tarball under `target/sdk-release-artifacts/` so
the release train carries runnable examples alongside package dry-runs.

Use `make sdk-check` for the local gate. The GitHub `SDK Release` workflow runs
the same preflight on SDK changes and can publish all three packages manually
from a version-matching `v*` tag through the protected `sdk-release`
environment after registry credentials are configured. See
[`docs/SDK_RELEASE.md`](../docs/SDK_RELEASE.md).
