# @cortexdb/client

Fetch-based JavaScript/TypeScript client for the Core Alpha CortexDB HTTP API.

```ts
import { CortexDBClient } from "@cortexdb/client";

const client = new CortexDBClient("http://127.0.0.1:8181");
await client.putCell(1, "scope=default\nstatus=ready\nhello");
console.log(await client.getCell(1));
const results = await client.search("default", "hello");
console.log(results.search_mode, results.ann_report, results.results);
const annEval = await client.evaluateAnn("default", [1, 2, 3]);
console.log(annEval.available, annEval.recall_q16);
console.log(await client.ingestText("default", "hello from sdk"));
```

The npm package publishes `cortexdb-client.js` plus `cortexdb-client.d.ts`.
`cortexdb-client.ts` is kept as a source reference. Publication is not
automatic; run `../publish/check.sh` before cutting a package release.
