# SDK, Docker, Observability

This is the thin integration surface for Beta smoke paths.

## Docker

`Dockerfile` builds `cortex-server` and `cortexdb` in a Rust builder image and
runs the server as a non-root user. The hardened local container contract is in
[`DOCKER.md`](DOCKER.md). The production-like compose example adds an nginx
reverse proxy, `CORTEXDB_AUTH_TOKENS_FILE`, persistent data and backup volumes,
and a maintenance `backup-sidecar` profile:

```bash
docker build -t cortexdb:local .
docker compose up --build -d
make docker-quickstart-check
make docker-production-compose-check
```

Tagged releases publish `ghcr.io/aubakirovarman/cortexdb:<tag>` through the
release workflow.

## SDK Clients

Dependency-light HTTP clients live in `sdk/` and `crates/cortex-sdk`.
They cover health, put, get, tombstone, flush, compact, AQL, context, verify,
remember, ingestion, keyword search, vector search, stats, and validate.

`make sdk-check` runs the local publish preflight. Actual registry publishing
is manual-only from a `v*` tag through the protected `sdk-release` environment.

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
