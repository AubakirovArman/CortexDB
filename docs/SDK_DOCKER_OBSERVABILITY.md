# SDK, Docker, Observability v0

This is a thin integration surface for Core Alpha smoke paths.

## Docker

`Dockerfile` builds `cortex-server` and `cortexdb` in a Rust builder image and
runs the server as a non-root user:

```bash
docker build -t cortexdb:core-alpha .
docker run --rm -p 8181:8181 -v "$PWD/data:/data" cortexdb:core-alpha
```

## SDK Clients

Dependency-light HTTP clients live in `sdk/`:

- `sdk/python/cortexdb_client.py`
- `sdk/typescript/cortexdb-client.js` plus `cortexdb-client.d.ts`
- `crates/cortex-sdk`

They cover health, put, get, tombstone, flush, compact, AQL, context, verify,
remember, ingestion, keyword search, vector search, stats, and validate.
Packaging metadata is present in
`sdk/python/pyproject.toml`, `sdk/typescript/package.json`, and
`crates/cortex-sdk/Cargo.toml`. Their APIs are usable for Core Alpha smoke
integrations and may still receive additive changes.

`sdk/publish/check.sh` runs the current publish preflight: Python bytecode,
Python unit tests, Python wheel build, Rust SDK tests, `cargo package`, JS
syntax check, and `npm pack --dry-run` when npm is available.
`make sdk-check` exposes the same gate locally, and the GitHub `SDK Release`
workflow runs it on SDK changes. Actual publishing is manual-only from a `v*`
tag through the protected `sdk-release` environment and is documented in
[`SDK_RELEASE.md`](SDK_RELEASE.md).

## Observability

Core Alpha exposes operational smoke endpoints:

```text
GET /v1/health
GET /v1/stats
GET /v1/validate
```

The CLI mirrors the same checks:

```bash
cortexdb stats ./data
cortexdb validate ./data
```

`examples/metrics/smoke.sh` probes the three HTTP endpoints.

## Demo Dataset

`examples/demo/` contains a tiny investment-project dataset and a matching AQL
retrieve query. It is meant for manual CLI/server smoke tests, not benchmarks.
