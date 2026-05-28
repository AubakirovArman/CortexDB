# SDK Quickstart

CortexDB provides official SDKs for **Rust**, **Python**, and **TypeScript**.
All SDKs share the same typed response models and support tenant + Bearer token authentication.

## Version Alignment

| Artifact | Version |
|---|---|
| Server | `0.1.0` |
| OpenAPI | `0.1.0-core-alpha` |
| Rust SDK (`cortex-sdk`) | `0.1.0` |
| Python SDK (`cortexdb-client`) | `0.1.0` |
| TypeScript SDK (`@cortexdb/client`) | `0.1.0` |

## Rust

```toml
[dependencies]
cortex-sdk = "0.1.0"
```

```rust
use cortex_sdk::CortexDbClient;

let client = CortexDbClient::new("http://127.0.0.1:8181")
    .with_token("secret")
    .with_tenant("tenant:alpha");

let health = client.health_response()?;
println!("Server version: {}", health.server_version);

let put = client.put_cell_response(1, "hello world")?;
let search = client.search_keyword_response("default", "hello", 10)?;
```

See `crates/cortex-sdk/examples/basic.rs` for a runnable example.

## Python

```bash
pip install cortexdb-client
```

```python
from cortexdb_client import CortexDBClient

client = CortexDBClient("http://127.0.0.1:8181")
client.token = "secret"
client.tenant = "tenant:alpha"

health = client.health_response()
print(f"Server version: {health.server_version}")

put = client.put_cell_response(1, "hello world")
search = client.search_response("default", "hello", limit=10)
```

## TypeScript

```bash
npm install @cortexdb/client
```

```typescript
import { CortexDBClient } from "@cortexdb/client";

const client = new CortexDBClient("http://127.0.0.1:8181", "secret", "tenant:alpha");

const health = await client.health();
console.log(`Server version: ${health.server_version}`);

const put = await client.putCell(1, "hello world");
const search = await client.search("default", "hello", 10);
```

### ESM / CJS Policy

`@cortexdb/client` ships both ESM (`.js`) and CommonJS (`.cjs`) builds.
- **ESM** is the primary format (`import`).
- **CJS** is provided for backward compatibility (`require`).
- The source `.ts` file is included for TypeScript consumers who prefer their own compilation.

## Authentication & Tenant

All SDKs support the same auth model:

| SDK | Token | Tenant |
|---|---|---|
| Rust | `.with_token("...")` | `.with_tenant("...")` |
| Python | `client.token = "..."` | `client.tenant = "..."` |
| TypeScript | `new CortexDBClient(url, "...")` | `new CortexDBClient(url, token, "...")` or `.withTenant("...")` |

## Typed vs Raw Responses

Every endpoint has two methods:

| Typed | Raw (`dict`/`JsonObject`) |
|---|---|
| `client.health_response()` | `client.health()` |
| `client.put_cell_response(...)` | `client.put_cell(...)` |
| `client.search_response(...)` | `client.search(...)` |

Use typed methods for compile-time safety. Use raw methods when experimenting or when the server returns fields not yet modeled in the SDK.
