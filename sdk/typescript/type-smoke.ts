import { CortexDBClient } from "./cortexdb-client.js";
import type { ClientOptions, ContextPackResponse, HealthResponse, OpenApiHealthResponse } from "./cortexdb-client";

const options: ClientOptions = {
  timeoutMs: 2500,
  fetch: async () => new Response(JSON.stringify({
    status: "ok",
    version: "test",
    server_version: "test",
  })),
};

const client = new CortexDBClient("http://127.0.0.1:8181")
  .withRetries(1, 0)
  .withTimeout(2500)
  .withOptions(options)
  .withSession();

const health: HealthResponse = await client.health();
if (health.status !== "ok") throw new Error("unexpected health response");

const openApiHealth: OpenApiHealthResponse = health;
if (openApiHealth.server_version !== "test") throw new Error("unexpected OpenAPI health type");

const context: ContextPackResponse = {
  schema_version: "context_pack.v1",
  token_budget_tokens: 128,
  estimated_tokens: 0,
  truncated: false,
  citations_required: true,
  cells: [],
  anomalies: [],
};
if (context.schema_version !== "context_pack.v1") {
  throw new Error("unexpected context schema");
}

const txClient = new CortexDBClient("http://127.0.0.1:8181").withOptions({
  timeoutMs: 2500,
  fetch: async () =>
    new Response(JSON.stringify({ outcome: "committed", idempotent_replay: false })),
});
const tx = await txClient.agentTransaction({ scope: "agent:one", base_seq: 0, operations: [] });
if (tx.outcome !== "committed") throw new Error("unexpected transaction outcome");
txClient.close();

const handoffClient = new CortexDBClient("http://127.0.0.1:8181").withOptions({
  timeoutMs: 2500,
  fetch: async () =>
    new Response(JSON.stringify({ level: "shared_sequenced", handoff_cell_id: 1 })),
});
const handoff = await handoffClient.agentHandoff({
  source_agent_id: 1,
  target_agent_id: 2,
  scope: "shared:project",
  pack_hash: "h",
  pack_seq: 0,
  required_after_seq: 0,
});
if (handoff.level !== "shared_sequenced") throw new Error("unexpected handoff level");
handoffClient.close();

client.close();
