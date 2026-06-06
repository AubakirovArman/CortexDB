import { CortexDBClient } from "../cortexdb-client.js";

const client = new CortexDBClient("http://127.0.0.1:8181");

const health = await client.health();
console.log(`server_version=${health.server_version}`);

const put = await client.putCell(
  1,
  "scope=default\nstatus=ready\ntype=fact\nsource=typescript-sdk\n\nhello world",
);
console.log(`put_seq=${put.seq}`);

const lookup = await client.getCell(1);
console.log(`cell_found=${lookup.cell !== null}`);

const search = await client.search("default", "hello");
console.log(`search_results=${search.results.length}`);

const aql = client.buildRetrieveContextAql("hello", "default", { limitCandidates: 10 });
const context = await client.retrieveContext(
  "default",
  aql,
);
console.log(`context_tokens=${context.estimated_tokens}`);
