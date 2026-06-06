# SDK Quickstart

CortexDB provides official SDKs for **Rust**, **Python**, and **TypeScript**.
All SDKs share the same typed response models and support tenant + Bearer token authentication.

## Version Alignment

For public registry availability and beta publication status, see
[`SDK_PUBLICATION_STATUS.md`](SDK_PUBLICATION_STATUS.md). The local release
gates prove package construction and live local server e2e behavior even before
public registry publication.

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

See `crates/cortex-sdk/examples/basic.rs` for a runnable example and
`crates/cortex-sdk/examples/live_contract.rs` for the live API contract smoke.

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
retrieve = client.build_retrieve_context_aql("hello", "default", limit_candidates=10)
context = client.context_response("default", retrieve)
verify = client.verify_response("default", client.build_verify_fact_aql("hello world", "default"))
remember = client.remember_response("default", client.build_remember_aql("hello", "default", "decision", ttl_seconds=3600))
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
const retrieve = client.buildRetrieveContextAql("hello", "default", { limitCandidates: 10 });
const context = await client.retrieveContext("default", retrieve);
const verify = await client.verifyFact("default", client.buildVerifyFactAql("hello world", "default"));
const remember = await client.remember("default", client.buildRememberAql("hello", "default", "decision", 3600));
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
| `client.context_response(...)` | `client.context(...)` |
| `client.verify_response(...)` | `client.verify(...)` |
| `client.remember_response(...)` | `client.remember(...)` |

Use typed methods for compile-time safety. Use raw methods when experimenting or when the server returns fields not yet modeled in the SDK.

Rust also exposes stable ContextPack agent exports:

```rust
let prompt = client.context_prompt("default", aql)?;
let markdown = client.context_markdown("default", aql)?;
```

The Rust SDK models ContextPack v1 with typed public structs:

```rust
use cortex_sdk::{ContextPackAnomalyV1, ContextPackCellV1, ContextPackV1};

let pack: ContextPackV1 = client.context_response("default", aql)?;
assert!(pack.is_v1());

for cell in &pack.cells {
    println!("cell={} tokens={}", cell.cell_id, cell.estimated_tokens);
}

let overloads = pack.anomaly_count("token_overload");
```

The `ContextPackV1` aliases cover selected cells, source refs, explain details,
and anomalies. They support both `serde_json` decode and encode so downstream
agents can persist, snapshot, or forward packs without using ad-hoc JSON maps.

Rust verification helpers keep VERIFY FACT requests and conflict handling typed:

```rust
use cortex_sdk::{VerifyConflict, VerifyRequest, VerifyResult};

let request = VerifyRequest::fact("default", "Budget is 1.2B KZT", "default")?;
let report = client.verify_request_response(&request)?;

match report.result() {
    VerifyResult::Supported => println!("supported"),
    VerifyResult::Contradicted | VerifyResult::MixedEvidence => {
        for conflict in report.conflicts() {
            match conflict {
                VerifyConflict::ContradictingEvidence(evidence) => {
                    println!("conflicting cell {}", evidence.cell_id);
                }
                VerifyConflict::Numeric(numeric) => {
                    println!("{}: {} vs {}", numeric.metric, numeric.left, numeric.right);
                }
            }
        }
    }
    _ => println!("not enough database evidence"),
}

let markdown = client.verify_request_export(&request.markdown())?;
```

## Live Contract Gate

Before publishing or changing SDK contracts, run:

```bash
make sdk-contract-check
```

This builds the current `cortex-server` binary and runs Python, TypeScript, and
Rust SDK smoke tests against real `/v1/*` responses. The gate covers health,
put/get, search, stats, validate, AQL, Context Pack, Verify Fact, Remember,
ingest text, tenant routing, Bearer auth, and structured error responses such
as `unauthorized`, `invalid_aql`, `not_found`, and `invalid_tenant`.
