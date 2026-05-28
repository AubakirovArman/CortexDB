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
    auth_token="my-secret-token"
)
```

### TypeScript

```typescript
import { CortexDBClient } from "@cortexdb/client";

const client = new CortexDBClient("http://127.0.0.1:8181", {
  authToken: "my-secret-token"
});
```

### Rust

```rust
use cortex_sdk::CortexDBClient;

let client = CortexDBClient::new("http://127.0.0.1:8181")
    .with_auth_token("my-secret-token");
```

## Security Notes

- Use a strong, random token in production (e.g. 32+ bytes from `/dev/urandom`).
- Pass the token via environment variable, never commit it to version control.
- CortexDB auth is **transport-only**; there is no user model, RBAC, or session management.
- For multi-user deployments, run separate tenant realms with network-level isolation.
