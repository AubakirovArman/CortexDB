# Authentication

CortexDB supports optional Bearer token authentication on the HTTP server.
Do not pass bearer tokens as CLI arguments. Use environment variables for local
development and token/policy files for operational runs so secrets do not appear
in process listings or shell history.

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
    {"principal_id":"agent-a","token":"agent-token","role":"data","agent_id":7,"request_quota_per_minute":600,"context_budget_tokens":1000,"tenants":["default","alpha"]},
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
- `context_budget_tokens`: maximum AQL/ContextPack context budget for a
  principal bound to an AgentView.

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

Manage local AgentViews with the CLI before binding tokens to them:

```bash
cortexdb agent create ./data 8 \
  --label finance-agent \
  --read-scope project:investments \
  --write-scope project:investments

cortexdb agent grant ./data 8 project:finance --access read_write
cortexdb agent revoke ./data 8 project:finance --access read
cortexdb agent list ./data
cortexdb --json agent show ./data 8
```

Core Beta keeps AgentViews in the existing durable local
`agent_views/*.view` compatibility bridge. The store writes through the engine
with temp-file, fsync, rename, and parent-directory fsync publication. Auth
policy records bind tokens/principals to `agent_id`; they do not duplicate
AgentView scope sets. Moving AgentViews into system cells is a future migration,
not a B05 requirement.

Review local auth policy files without exposing bearer tokens:

```bash
cortexdb auth-review --policy-store ./auth-policy.json
cortexdb auth-review --tokens-file ./auth.tokens
cortexdb auth-review --tokens-env CORTEXDB_AUTH_TOKENS
cortexdb --json auth-review --policy-store ./auth-policy.json --tokens-file ./auth.tokens
```

The review output includes source, principal ID, role, active/disabled state,
optional `agent_id`, optional quota fields, and optional `capabilities`. It
intentionally does not print token values.
`auth-review --tokens` is intentionally rejected because inline token arguments
are visible in process listings.

Admin tokens can mutate the local JSON policy store when
`CORTEXDB_AUTH_POLICY_STORE_FILE` is configured. These routes are admin-only and
write a rollback snapshot next to the policy file before publishing the mutated
store:

```bash
curl -X POST \
  -H "Authorization: Bearer root-token" \
  -H "Content-Type: application/json" \
  http://127.0.0.1:8181/v1/admin/auth/principal \
  -d '{"principal_id":"agent-b","token":"agent-b-token","role":"data","agent_id":8,"request_quota_per_minute":600,"body_quota_bytes_per_minute":1048576,"queue_quota":2,"context_budget_tokens":1000,"capabilities":["search","read"],"tenants":["default","alpha"]}'

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
AgentView binding, disabled state, quota, context budget, capabilities, tenant
allowlist, and a token fingerprint. It intentionally does not store the raw
bearer token in the cell payload; the JSON policy store remains the credential
source of truth.

Admins can also review the redacted policy store through HTTP:

```bash
curl -H "Authorization: Bearer root-token" \
  http://127.0.0.1:8181/v1/admin/auth/policies
```

The list response has schema `cortexdb.auth_policy_list.v1`, includes role,
AgentView binding, quota, capabilities, tenant allowlist, disabled state, and a
stable token fingerprint, and never returns raw bearer token values.

Agent scope permissions are still stored in `AgentView`, not duplicated in the
policy store. Admins can grant or revoke AgentView scopes through:

```bash
curl -X POST \
  -H "Authorization: Bearer root-token" \
  -H "Content-Type: application/json" \
  http://127.0.0.1:8181/v1/admin/auth/scope/grant \
  -d '{"agent_id":8,"scope":"project:investments","access":"read_write"}'

curl -X POST \
  -H "Authorization: Bearer root-token" \
  -H "Content-Type: application/json" \
  http://127.0.0.1:8181/v1/admin/auth/scope/revoke \
  -d '{"agent_id":8,"scope":"project:investments","access":"read"}'
```

`access` accepts `read`, `write`, or `read_write`. These endpoints require an
existing persisted AgentView. Core Alpha keeps roles static (`admin` and
`data`); dynamic custom role definitions remain future work.

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
import { CortexDBClient } from "cortexdb-sdk";

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
  "queue_quota": 2,
  "context_budget_tokens": 1000
}
```

If that principal exhausts any configured quota, the server returns typed
`429 quota_exceeded`. Other principals continue using their own
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
include local chain metadata: `chain_id`, `sequence`, `prev_hash`,
`event_hash`, `mac_key_id`, and `event_mac`. File-backed records include
`scope_decision` with only `allowed`, `denied`, or `not_applicable`; raw scope
labels are not logged. Request bodies, query strings, bearer tokens, and audit
MAC key material are intentionally not logged. Current route categories include
`read`, `write`, `delete`, `aql`, `search`, `context`, `verify`, `ingest`,
`memory`, `admin`, `metrics`, and `health`.

Every HTTP response includes `x-request-id`. Clients may supply a safe
`x-request-id` header to correlate their logs with CortexDB audit records. If
they omit it, the server generates a `cortexdb-<n>` request id. Metrics expose
`request_id_client_provided` and `request_id_generated` so operators can check
whether clients are consistently sending correlation IDs.

To persist route-level audit events to a local JSONL file, set:

```bash
export CORTEXDB_AUDIT_LOG_FILE="./audit/http.jsonl"
export CORTEXDB_AUDIT_LOG_ROTATE_BYTES=104857600
export CORTEXDB_AUDIT_LOG_FSYNC=always
export CORTEXDB_AUDIT_MAC_KEY_ID="local-audit-key-2026q2"
export CORTEXDB_AUDIT_MAC_KEY_HEX="000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"
cargo run -p cortex-server -- ./data 127.0.0.1:8181
```

`CORTEXDB_AUDIT_LOG_FILE` implies audit logging. The server creates parent
directories if needed, appends one JSON object per response, flushes the file,
and uses the configured fsync policy after each event. `always` calls
`sync_data()` per event; `flush` and `flush-only` flush without `sync_data()`.
`CORTEXDB_AUDIT_MAC_KEY_HEX` is required for file-backed audit records and must
be a 32-byte hex value; `CORTEXDB_AUDIT_MAC_KEY_ID` labels the key in the JSONL
record and defaults to `local-audit-key` when omitted. Store the key outside
the audit log and do not pass it as a command-line argument.
`CORTEXDB_AUDIT_LOG_ROTATE_BYTES` rotates the active JSONL file after the
configured byte limit; rotated files are local JSONL segments with independent
chain verification. File sink failures after startup are reported through
`tracing` target `cortexdb_audit` as `sink_error` events; they do not include
request bodies or query strings. The complete record contract is documented in
[`AUDIT_LOG_FORMAT.md`](AUDIT_LOG_FORMAT.md).

Review a persisted audit file with the CLI instead of hand-parsing JSONL:

```bash
cortexdb audit ./audit/http.jsonl --summary --redaction-check
cortexdb audit ./audit/http.jsonl --summary --redaction-check --verify-chain --mac-key-file ./audit/audit-mac.key
cortexdb audit verify ./audit/http.jsonl --mac-key-file ./audit/audit-mac.key
cortexdb audit ./audit/http.jsonl --route /v1/cell --status 403
cortexdb audit ./audit/http.jsonl --action write --tenant-filter tenant-alpha
cortexdb --json audit ./audit/http.jsonl --summary --redaction-check
cortexdb audit-export-siem ./audit/http.jsonl ./audit/siem.jsonl --redaction-check --verify-chain --mac-key-file ./audit/audit-mac.key
```

The audit viewer supports filters by route, status, action, and tenant. The
summary output includes counts by action, status, tenant, and route. The
`--redaction-check` flag fails if records contain query strings or body-like
fields, which keeps route-level audit review separate from request payloads.
The `--verify-chain` flag validates local sequence continuity, chained
SHA-256 event hashes, and HMAC-SHA-256 `event_mac` values for
`cortexdb.audit.v2` records when `--mac-key-file` is supplied. Without the MAC
key, keyed v2 records fail chain verification; legacy v1 hash-chain records
remain readable as compatibility records. LLM inference audit decisions also
include safe decision metadata such as provider, model, outcome, citation
count, and guardrail reason in the hashed/MACed event surface. When configured
receipt emission returns an `accountability_receipt.v1`, HTTP audit records
commit the receipt hash in `accountability_receipt_hash`; prompts, request
bodies, bearer tokens, and secrets remain excluded. This is a local
tamper-evidence foundation, not a compliance-certified audit ledger.
`cortexdb audit verify <audit.jsonl>` is the short fail-closed alias for
`cortexdb audit <audit.jsonl> --summary --verify-chain`. If the configured file
sink ends with a malformed chained record, server startup fails instead of
silently resetting the chain; rotate or repair the audit file explicitly.

`audit-export-siem` writes normalized JSONL records with schema
`cortexdb.siem.audit.v1`. It preserves route metadata, principal metadata,
request IDs, status, duration, audit-chain fields, `mac_key_id`, and
`event_mac`, but does not add request bodies, query strings, bearer tokens, or
MAC key material. Use `--redaction-check`, `--verify-chain`, and
`--mac-key-file` before exporting v2 audit logs to fail closed on unsafe local
audit input.
The local export, retention, and redaction boundary is defined in
[`AUDIT_EXPORT_RETENTION_POLICY.md`](archive/AUDIT_EXPORT_RETENTION_POLICY.md) and
validated by `make audit-export-retention-check`.

## Receipt Signing Key Custody

The current receipt-key surface provides Ed25519 node key custody for
configured `accountability_receipt.v1` JSON emission. When
`CORTEXDB_RECEIPT_SIGNING_KEY_FILE` or `CORTEXDB_RECEIPT_SIGNING_KEY_HEX` is
configured, JSON ContextPack and verification responses include signed receipt
objects. Without a configured receipt signing key, the additive
`accountability_receipt` field remains absent. When a configured JSON response
does include a receipt, file-backed audit v2 records commit the receipt hash in
`accountability_receipt_hash` so the local audit chain/MAC covers the returned
receipt without storing the receipt body.

Configured receipt emission also binds receipt header `db_instance_id` to a
durable local database-instance identity. Server startup creates or reads
`cortexdb.database_instance_identity.json` in the database root using
`cortexdb.database_instance_identity.v1`; this value is reused across tenants
for the same local database instance and is not derived from the request tenant.
If receipt signing is configured but the identity file is invalid, startup and
receipt emission fail closed.

Generate a local receipt signing key and export its public key:

```bash
cortexdb receipt-key generate ./keys/receipt-key.json --key-id local-receipt-key-2026q2 --public-key-file ./keys/receipt-key.public.json
cortexdb receipt-key export-public ./keys/receipt-key.json ./keys/receipt-key.public.json
```

Rotate to a new key id, keep a dual-trust manifest for historical
verification, and write a receipt/audit re-anchor record:

```bash
cortexdb receipt-key rotate ./keys/receipt-key.json ./keys/receipt-key-next.json ./keys/receipt-trust.json --new-key-id local-receipt-key-2026q3 --reanchor-file ./keys/receipt-reanchor.json --audit-chain-head 0000000000000000000000000000000000000000000000000000000000000000 --audit-sequence 0
cortexdb receipt-key verify-reanchor ./keys/receipt-reanchor.json --trust-file ./keys/receipt-trust.json
```

The private key file uses `cortexdb.receipt_signing_key.v1`, the public file
uses `cortexdb.receipt_public_key.v1`, and the rotation manifest uses
`cortexdb.receipt_trust.v1`. The re-anchor record uses
`cortexdb.receipt_audit_reanchor.v1` and is signed by both previous and current
receipt keys over the old/new public keys, trust manifest hash, audit chain
head, and audit sequence. CLI output intentionally prints key ids and file paths
only, not `signing_seed_hex`.

For server-side custody preflight, prefer a key file:

```bash
export CORTEXDB_RECEIPT_SIGNING_KEY_FILE="./keys/receipt-key.json"
cargo run -p cortex-server -- ./data 127.0.0.1:8181
```

For short-lived local test environments, the server can also parse
`CORTEXDB_RECEIPT_SIGNING_KEY_ID` plus `CORTEXDB_RECEIPT_SIGNING_KEY_HEX`.
Do not pass receipt signing seeds as command-line arguments, and do not store
private key JSON beside public audit logs or exported ContextPacks.

For non-local custody, the server can call an external receipt signer command
instead of loading `signing_seed_hex`:

```bash
export CORTEXDB_RECEIPT_EXTERNAL_SIGNER_COMMAND="./bin/receipt-signer"
export CORTEXDB_RECEIPT_EXTERNAL_SIGNER_KEY_ID="receipt-key-2026q3"
export CORTEXDB_RECEIPT_EXTERNAL_SIGNER_PUBLIC_KEY_HEX="<64 hex chars>"
export CORTEXDB_RECEIPT_EXTERNAL_SIGNER_REF="kms://operator-owned/receipt-key-2026q3"
cargo run -p cortex-server -- ./data 127.0.0.1:8181
```

The command receives JSON schema
`cortexdb.receipt_external_sign_request.v1` on stdin and returns
`cortexdb.receipt_external_signature.v1` on stdout. The server verifies the
returned Ed25519 signature against the configured public key and fails closed
if the command is unavailable, returns a mismatched key id/public key, or emits
an invalid signature. This external command path is not by itself a KMS/HSM
custody claim.

Production-grade KMS/HSM custody requires an operator evidence file with schema
`cortexdb.receipt_kms_hsm_custody_evidence.v1`. The evidence must bind
`key_id`, `public_key_hex`, `signer_ref`, provider key reference, signing domain
`cortexdb.accountability_receipt.sign.v1`, non-exportable key policy, disabled
local-seed fallback, a `runtime_signing_probe` signed by the same runtime public
key over `cortexdb.accountability_receipt.sign.v1 || 0x00 ||
canonical_header_hex bytes`, a signature-verified `production_origin_proof`,
and at least two hashed custody artifacts. The custody gate accepts it only when
the expected runtime binding and expected key-attestor trust-anchor binding are
passed explicitly:

```bash
make receipt-kms-hsm-custody-check \
  RECEIPT_KMS_HSM_CUSTODY_EVIDENCE="./evidence/receipt-kms-hsm.json" \
  RECEIPT_KMS_HSM_EXPECTED_KEY_ID="receipt-key-2026q3" \
  RECEIPT_KMS_HSM_EXPECTED_PUBLIC_KEY_HEX="<64 hex chars>" \
  RECEIPT_KMS_HSM_EXPECTED_SIGNER_REF="kms://operator-owned/receipt-key-2026q3" \
  RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_KEY_ID="root-key-attestor-2026" \
  RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_PUBLIC_KEY_HEX="<64 hex chars>" \
  RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_REF="https://trust.example/attestors/root-key-attestor" \
  RECEIPT_PRODUCTION_ORIGIN_EXPECTED_KEY_ATTESTOR_PUBLIC_KEY_REF="https://trust.example/keys/root-key-attestor.pub"
```

Without this evidence, the `receipt-kms-hsm-custody-check` report continues to
show `kms_hsm_custody=false`. Operator-shaped evidence with a valid runtime
signing probe but without `production_origin_proof` is not sufficient for a
KMS/HSM custody claim.

## RBAC Roadmap

Core Alpha keeps route authorization intentionally small: static `admin` and
`data` roles plus optional AgentView binding. The JSON policy-store file adds a
durable local principal list and disabled-principal lifecycle, while the broader
enterprise design remains tracked in
[`RBAC_POLICY_STORE_DESIGN.md`](archive/RBAC_POLICY_STORE_DESIGN.md) and
[`ENTERPRISE_RBAC_COMPLIANCE_DESIGN.md`](archive/ENTERPRISE_RBAC_COMPLIANCE_DESIGN.md).
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
