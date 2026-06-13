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

client.close();
