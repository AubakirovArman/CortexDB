# @cortexdb/client

Minimal fetch-based TypeScript client for the current CortexDB HTTP API.

```ts
import { CortexDBClient } from "@cortexdb/client";

const client = new CortexDBClient("http://127.0.0.1:8181");
await client.putCell(1, "scope=default\nstatus=ready\nhello");
console.log(await client.getCell(1));
console.log(await client.search("default", "hello"));
```
