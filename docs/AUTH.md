# Authentication

CortexDB supports optional Bearer token authentication on the HTTP server.

## Enabling Auth

Set the `CORTEXDB_AUTH_TOKEN` environment variable before starting the server:

```bash
export CORTEXDB_AUTH_TOKEN="my-secret-token"
cargo run -p cortex-server -- ./data 127.0.0.1:8181
```

When `CORTEXDB_AUTH_TOKEN` is set, **every** request must include a valid `Authorization` header.

## Sending Requests with Auth

Include the `Authorization: Bearer <token>` header in every request:

```bash
curl -H "Authorization: Bearer my-secret-token" \
  http://127.0.0.1:8181/v1/health

curl -X POST \
  -H "Authorization: Bearer my-secret-token" \
  -H "Content-Type: text/plain" \
  "http://127.0.0.1:8181/v1/cell?cell_id=1" \
  -d "hello world"
```

## Unauthorized Response

If the token is missing or invalid, the server returns:

```json
{
  "code": "unauthorized",
  "error": "unauthorized",
  "message": "missing or invalid authorization"
}
```

with HTTP status `401 Unauthorized`.

## CLI with Auth

The CLI binary `cortexdb` operates directly on the local filesystem and **does not use HTTP auth**. CLI commands bypass the server entirely and open the database directly.

If you need to script against the authenticated HTTP API, use the SDKs:

### Python

```python
from cortexdb_client import CortexDBClient

client = CortexDBClient(
    "http://127.0.0.1:8181",
    token="my-secret-token"
)
```

### TypeScript

```typescript
import { CortexDBClient } from "@cortexdb/client";

const client = new CortexDBClient("http://127.0.0.1:8181", "my-secret-token");
```

### Rust

```rust
use cortex_sdk::CortexDbClient;

let client = CortexDbClient::new("http://127.0.0.1:8181")
    .with_token("my-secret-token");
```

## Security Notes

- Use a strong, random token in production (e.g. 32+ bytes from `/dev/urandom`).
- Pass the token via environment variable, never commit it to version control.
- CortexDB auth is **transport-first**. Core Alpha supports one configured
  bearer token and can optionally bind that token to a persisted `AgentView`.
- Full multi-user RBAC, sessions, and per-token policy stores are future work.
- For multi-user deployments, run separate tenant realms with network-level isolation.

## Binding Auth To An AgentView

Core Alpha can map the configured bearer token to one persisted `AgentView`.
This lets scope-bound HTTP data routes reuse the same readable/writable scope
policy as AQL and ContextPack execution.

```bash
export CORTEXDB_AUTH_TOKEN="my-secret-token"
export CORTEXDB_AUTH_AGENT_ID=7
cargo run -p cortex-server -- ./data 127.0.0.1:8181
```

When `CORTEXDB_AUTH_AGENT_ID` is set:

- the server loads `agent_views/7.view` from the active tenant database;
- scope-bound reads such as `/v1/search`, `/v1/context`, `/v1/aql`, and
  `/v1/verify` must target a readable scope;
- scope-bound writes such as `/v1/cell`, `/v1/remember`, `/v1/forget`, and
  `/v1/ingest/*` must target a writable scope;
- denied requests return `403 permission_denied`;
- admin/health/metrics routes remain guarded by the bearer token, but do not
  yet use a separate admin-role model.

If the persisted AgentView is missing, scope-bound data routes return
`403 permission_denied`. `CORTEXDB_AUTH_AGENT_ID` requires
`CORTEXDB_AUTH_TOKEN`; the server rejects that configuration at startup.

## Server Backpressure

The HTTP server routes database work through a bounded per-tenant
`DatabaseActor` queue. The default capacity is `1024`. Override it with:

```bash
export CORTEXDB_ACTOR_QUEUE_CAPACITY=2048
cargo run -p cortex-server -- ./data 127.0.0.1:8181
```

Use a lower value to fail fast under load, or a higher value to absorb short
bursts. Invalid or zero values are rejected at startup. When the queue is full,
the server returns `503 database_busy` instead of silently accepting unlimited
work.

## Request Rate Limit

Rate limiting is disabled by default. For exposed local deployments, configure a
coarse process-wide fixed-window limit:

```bash
export CORTEXDB_RATE_LIMIT_PER_MINUTE=600
cargo run -p cortex-server -- ./data 127.0.0.1:8181
```

When the 60-second window is exhausted, the server returns:

```json
{
  "code": "rate_limited",
  "error": "rate_limited",
  "message": "request rate limit exceeded"
}
```

with HTTP status `429 Too Many Requests`. This is a Core Alpha safety guard, not
a replacement for reverse-proxy quotas, per-user authorization, or API gateway
controls.

## Audit Logging

Audit logging is disabled by default. Enable it when you need an operational
trail for API access:

```bash
export CORTEXDB_AUDIT_LOG=true
cargo run -p cortex-server -- ./data 127.0.0.1:8181
```

Audit events are emitted through `tracing` with target `cortexdb_audit`. They
include route category, method, path, tenant, status code, stable error code,
and duration. Request bodies and query strings are intentionally not logged.
Current route categories include `read`, `write`, `delete`, `aql`, `search`,
`context`, `verify`, `ingest`, `memory`, `admin`, `metrics`, and `health`.

To persist route-level audit events to a local JSONL file, set:

```bash
export CORTEXDB_AUDIT_LOG_FILE="./audit/http.jsonl"
cargo run -p cortex-server -- ./data 127.0.0.1:8181
```

`CORTEXDB_AUDIT_LOG_FILE` implies audit logging. The server creates parent
directories if needed, appends one JSON object per response, flushes the file,
and calls `sync_data()` after each event. File sink failures after startup are
reported through `tracing` target `cortexdb_audit` as `sink_error` events; they
do not include request bodies or query strings.

## Browser CORS

CORS is disabled by default. Same-origin dashboard usage does not need CORS.
If a browser application on another origin must call the HTTP API, configure one
exact trusted origin:

```bash
export CORTEXDB_CORS_ALLOW_ORIGIN="https://app.example"
cargo run -p cortex-server -- ./data 127.0.0.1:8181
```

The server then allows `GET`, `POST`, `DELETE`, and `OPTIONS` requests from
that origin with `Authorization` and `Content-Type` headers. Wildcard browser
origins are intentionally not supported in Core Alpha because bearer-token API
access should not be exposed broadly.
