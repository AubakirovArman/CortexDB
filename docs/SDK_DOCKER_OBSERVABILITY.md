# SDK, Docker, Observability v0

This is a thin integration surface for Core Alpha smoke paths.

## Docker

`Dockerfile` builds `cortex-server` and `cortexdb` in a Rust builder image and
runs the server as a non-root user:

```bash
docker build -t cortexdb:core-alpha .
docker run --rm -p 8181:8181 -v "$PWD/data:/data" cortexdb:core-alpha
```

## SDK Sketches

Minimal HTTP clients live in `sdk/`:

- `sdk/python/cortexdb_client.py`
- `sdk/typescript/cortexdb-client.ts`

They cover put, get, search, stats, and validate. Packaging metadata is present
in `sdk/python/pyproject.toml` and `sdk/typescript/package.json`. Their APIs are
not frozen.

`sdk/publish/check.sh` runs the current publish preflight: Python bytecode
compile plus `npm pack --dry-run` when npm is available.

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
