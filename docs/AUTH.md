# Authentication

CortexDB supports optional Bearer token authentication on the HTTP server.

## Enabling Auth

Set the `CORTEXDB_AUTH_TOKEN` environment variable before starting the server:

```bash
export CORTEXDB_AUTH_TOKEN="my-secret-token"
cargo run -p cortex-server -- ./data 127.0.0.1:8181
```

When `CORTEXDB_AUTH_TOKEN` is set, **every** request must include a valid `Authorization` header.
The legacy token is treated as an admin token for backward compatibility.

For multiple tokens, set `CORTEXDB_AUTH_TOKENS`:

```bash
export CORTEXDB_AUTH_TOKENS="admin:root-token,data:app-token,data:agent-token:7"
cargo run -p cortex-server -- ./data 127.0.0.1:8181
```

Each comma-separated entry is:

```text
role:token
role:token:agent_id
```

For operational token rotation without restarting the server, point
`CORTEXDB_AUTH_TOKENS_FILE` at a local policy file:

```bash
cat > ./auth.tokens <<'EOF'
# One policy per line; comments and blank lines are ignored.
admin:root-token
data:app-token
data:agent-token:7
EOF

export CORTEXDB_AUTH_TOKENS_FILE="./auth.tokens"
cargo run -p cortex-server -- ./data 127.0.0.1:8181
```

The file uses the same `role:token[:agent_id]` format as
`CORTEXDB_AUTH_TOKENS` and is re-read for every request. Replacing the file
therefore rotates tokens without a process restart. If the configured file is
missing, empty, or invalid, authentication fails closed and no token from that
file is accepted.

For a durable local policy-store shape with explicit principals and disabled
principal lifecycle, point `CORTEXDB_AUTH_POLICY_STORE_FILE` at a JSON file:

```bash
cat > ./auth-policy.json <<'EOF'
{
  "schema_version": "cortexdb.auth_policy.v1",
  "principals": [
    {"principal_id":"admin-a","token":"root-token","role":"admin"},
    {"principal_id":"agent-a","token":"agent-token","role":"data","agent_id":7,"request_quota_per_minute":600,"tenants":["default","alpha"]},
    {"principal_id":"old-agent","token":"disabled-token","role":"data","disabled":true}
  ]
}
EOF

export CORTEXDB_AUTH_POLICY_STORE_FILE="./auth-policy.json"
cargo run -p cortex-server -- ./data 127.0.0.1:8181
```

The policy store is re-read for every request. Disabled principals are ignored,
invalid JSON or invalid policy entries fail closed, and active entries use the
same `admin`/`data` role plus optional `agent_id` behavior as token-file
policies.

The canonical policy-store schema is `cortexdb.auth_policy.v1`. A legacy
`cortexdb.auth_policy.v0` file with a top-level `tokens` array is accepted only
as an in-memory read migration into v1 principals. Unknown schema versions fail
closed and do not authenticate.

Policy-store principals may also set local fixed-window quota fields:

- `request_quota_per_minute`: request count per 60-second window;
- `body_quota_bytes_per_minute`: accepted request body bytes per 60-second
  window;
- `queue_quota`: concurrent actor queue/in-flight command permits for that
  token/principal.

Quota values must be greater than zero. Raw bearer tokens are not exposed as
quota keys; principals use `principal_id`, while static token policies use an
internal token fingerprint.

Policy-store principals may also set `capabilities` to restrict an otherwise
valid role to selected API action classes. Supported values are `admin`, `aql`,
`context`, `delete`, `ingest`, `inference`, `memory`, `metrics`, `read`,
`search`, `verify`, and `write`. Omitting `capabilities` preserves the default
role behavior. An empty, duplicate, or unknown capability fails closed.

Policy-store principals may also set `tenants` to restrict which database
realms the token can access. Omitting `tenants` preserves the default
all-tenant local behavior. When present, the list must contain one or more safe
tenant IDs using the same validation as the HTTP `tenant=<realm>` query
parameter. Empty lists, duplicate tenants, and path-like or scope-like tenant
values fail closed.

Roles:

- `admin`: can access all authenticated API routes, including stats, validate,
  flush, compact, metrics, and data routes.
- `data`: can access data routes and health checks, but not admin or metrics
  routes.

If an `agent_id` is present, that token is also bound to the persisted
`AgentView` with the same ID, so scope permissions are enforced per token.

Review local auth policy files without exposing bearer tokens:

```bash
cortexdb auth-review --policy-store ./auth-policy.json
cortexdb auth-review --tokens-file ./auth.tokens
cortexdb --json auth-review --policy-store ./auth-policy.json --tokens-file ./auth.tokens
```

The review output includes source, principal ID, role, active/disabled state,
optional `agent_id`, optional quota fields, and optional `capabilities`. It
intentionally does not print token values.

Admin tokens can mutate the local JSON policy store when
`CORTEXDB_AUTH_POLICY_STORE_FILE` is configured. These routes are admin-only and
write a rollback snapshot next to the policy file before publishing the mutated
store:

```bash
curl -X POST \
  -H "Authorization: Bearer root-token" \
  -H "Content-Type: application/json" \
  http://127.0.0.1:8181/v1/admin/auth/principal \
  -d '{"principal_id":"agent-b","token":"agent-b-token","role":"data","agent_id":8,"request_quota_per_minute":600,"body_quota_bytes_per_minute":1048576,"queue_quota":2,"capabilities":["search","read"],"tenants":["default","alpha"]}'

curl -X DELETE \
  -H "Authorization: Bearer root-token" \
  "http://127.0.0.1:8181/v1/admin/auth/principal?principal_id=agent-b"

curl -X POST \
  -H "Authorization: Bearer root-token" \
  http://127.0.0.1:8181/v1/admin/auth/policy/rollback
```

The mutation response has schema `cortexdb.auth_policy_mutation.v1` and reports
the action, affected principal, active/disabled principal counts, and whether a
rollback snapshot exists. This is still a local file-backed admin lifecycle, not
external identity or enterprise compliance certification.

After a successful mutation, the server also mirrors the effective policy
metadata into durable redacted CortexDB cells under `_system:auth_policy`. The
mirror is written through the database actor and records principal ID, role,
AgentView binding, disabled state, quota, capabilities, tenant allowlist, and a
token fingerprint. It intentionally does not store the raw bearer token in the
cell payload; the JSON policy store remains the credential source of truth.

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
- CortexDB auth is **transport-first**. Core Alpha supports a legacy single
  admin token, static multi-token `admin`/`data` policies, and file-backed
  token rotation through `CORTEXDB_AUTH_TOKENS_FILE`.
- `CORTEXDB_AUTH_POLICY_STORE_FILE` adds a local durable policy-store file for
  explicit principals and disabled-principal lifecycle, but it is not yet a
  full enterprise RBAC administration system.
- Sessions, distributed quotas, external identity providers, and
  compliance-grade audit chains are future work.
- For multi-user deployments, run separate tenant realms with network-level isolation.

## Binding Auth To An AgentView

Core Alpha can map bearer tokens to persisted `AgentView` records. This lets
scope-bound HTTP data routes reuse the same readable/writable scope policy as
AQL and ContextPack execution.

```bash
export CORTEXDB_AUTH_TOKEN="my-secret-token"
export CORTEXDB_AUTH_AGENT_ID=7
cargo run -p cortex-server -- ./data 127.0.0.1:8181
```

or:

```bash
export CORTEXDB_AUTH_TOKENS="data:agent-token:7,admin:root-token"
cargo run -p cortex-server -- ./data 127.0.0.1:8181
```

When a token has an agent ID:

- the server loads `agent_views/7.view` from the active tenant database;
- scope-bound reads such as `/v1/search`, `/v1/context`, `/v1/aql`, and
  `/v1/verify` must target a readable scope;
- scope-bound writes such as `/v1/cell`, `/v1/remember`, `/v1/forget`, and
  `/v1/ingest/*` must target a writable scope;
- denied requests return `403 permission_denied`;
- admin and metrics routes require an `admin` token.

If the persisted AgentView is missing, scope-bound data routes return
`403 permission_denied`. The legacy `CORTEXDB_AUTH_AGENT_ID` setting still
requires `CORTEXDB_AUTH_TOKEN`; for per-token agent bindings, prefer
`CORTEXDB_AUTH_TOKENS`.

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
a replacement for reverse-proxy quotas, distributed quota state, or API gateway
controls.

When `CORTEXDB_AUTH_POLICY_STORE_FILE` is used, each active principal may set
local per-principal/per-token quotas:

```json
{
  "principal_id": "agent-a",
  "token": "agent-token",
  "role": "data",
  "request_quota_per_minute": 600,
  "body_quota_bytes_per_minute": 1048576,
  "queue_quota": 2
}
```

If that principal exhausts any configured quota, the server returns the same
typed `rate_limited` response. Other principals continue using their own
independent quota windows. `/v1/metrics` exposes aggregate quota counters for
allowed/rejected request checks, body bytes, and queue permits, but not raw
tokens or principal IDs.

## Audit Logging

Audit logging is disabled by default. Enable it when you need an operational
trail for API access:

```bash
export CORTEXDB_AUDIT_LOG=true
cargo run -p cortex-server -- ./data 127.0.0.1:8181
```

Audit events are emitted through `tracing` with target `cortexdb_audit`. They
include route category, method, path, tenant, `request_id`, status code, stable
error code, duration, and authenticated principal metadata when available:
`principal_id`, `auth_role`, and `auth_agent_id`. File-backed audit records also
include local chain metadata: `chain_id`, `sequence`, `prev_hash`, and
`event_hash`. Request bodies, query strings, and bearer tokens are intentionally
not logged. Current route categories include `read`, `write`, `delete`, `aql`,
`search`, `context`, `verify`, `ingest`, `memory`, `admin`, `metrics`, and
`health`.

Every HTTP response includes `x-request-id`. Clients may supply a safe
`x-request-id` header to correlate their logs with CortexDB audit records. If
they omit it, the server generates a `cortexdb-<n>` request id.

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

Review a persisted audit file with the CLI instead of hand-parsing JSONL:

```bash
cortexdb audit ./audit/http.jsonl --summary --redaction-check
cortexdb audit ./audit/http.jsonl --summary --redaction-check --verify-chain
cortexdb audit verify ./audit/http.jsonl
cortexdb audit ./audit/http.jsonl --route /v1/cell --status 403
cortexdb audit ./audit/http.jsonl --action write --tenant-filter tenant-alpha
cortexdb --json audit ./audit/http.jsonl --summary --redaction-check
cortexdb audit-export-siem ./audit/http.jsonl ./audit/siem.jsonl --redaction-check --verify-chain
```

The audit viewer supports filters by route, status, action, and tenant. The
summary output includes counts by action, status, tenant, and route. The
`--redaction-check` flag fails if records contain query strings or body-like
fields, which keeps route-level audit review separate from request payloads.
The `--verify-chain` flag validates local sequence continuity and chained event
hashes, detecting line deletion, reordering, and edited route metadata in
chain-v1 audit files. This is a local tamper-evidence foundation, not a
compliance-certified audit ledger. `cortexdb audit verify <audit.jsonl>` is the
short fail-closed alias for `cortexdb audit <audit.jsonl> --summary
--verify-chain`. If the configured file sink ends with a malformed chained
record, server startup fails instead of silently resetting the chain; rotate or
repair the audit file explicitly.

`audit-export-siem` writes normalized JSONL records with schema
`cortexdb.siem.audit.v1`. It preserves route metadata, principal metadata,
request IDs, status, duration, and audit-chain fields, but does not add request
bodies, query strings, or bearer tokens. Use `--redaction-check` and
`--verify-chain` before exporting to fail closed on unsafe local audit input.

## RBAC Roadmap

Core Alpha keeps route authorization intentionally small: static `admin` and
`data` roles plus optional AgentView binding. The JSON policy-store file adds a
durable local principal list and disabled-principal lifecycle, while the broader
enterprise design remains tracked in
[`RBAC_POLICY_STORE_DESIGN.md`](RBAC_POLICY_STORE_DESIGN.md) and
[`ENTERPRISE_RBAC_COMPLIANCE_DESIGN.md`](ENTERPRISE_RBAC_COMPLIANCE_DESIGN.md).
Until that layer is complete, do not treat CortexDB as a full multi-user RBAC or
compliance system.

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
