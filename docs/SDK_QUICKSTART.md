# SDK Quickstart

## Rust

```bash
cargo add cortex-sdk
```

```rust
use cortex_sdk::CortexDBClient;

fn main() {
    let client = CortexDBClient::new("http://127.0.0.1:8181");
    let response = client.search_keyword("project:investments", "budget", 10)
        .unwrap();
    println!("Found {} results", response.results.len());
}
```

## Python

```bash
pip install cortexdb-client
```

```python
from cortexdb_client import CortexDBClient

client = CortexDBClient("http://127.0.0.1:8181")
response = client.search_keyword("project:investments", "budget", limit=10)
print(f"Found {len(response.results)} results")
```

## TypeScript / JavaScript

```bash
npm install @cortexdb/client
```

```typescript
import { CortexDBClient } from "@cortexdb/client";

const client = new CortexDBClient("http://127.0.0.1:8181");
const response = await client.searchKeyword("project:investments", "budget", 10);
console.log(`Found ${response.results.length} results`);
```

## Tenant Scoping

All SDKs support per-tenant database realms:

```rust
let client = CortexDBClient::new("http://127.0.0.1:8181")
    .with_tenant("tenant:alpha");
```

```python
client = CortexDBClient("http://127.0.0.1:8181").with_tenant("tenant:alpha")
```

```typescript
const client = new CortexDBClient("http://127.0.0.1:8181", undefined, "tenant:alpha");
```

## ANN Evaluation

```rust
let eval = client.evaluate_ann("project:investments", &[1, 2, 3], 20).unwrap();
println!("ANN available: {}, recall: {:?}", eval.available, eval.recall_q16);
```
