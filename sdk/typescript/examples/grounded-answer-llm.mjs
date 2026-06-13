import { CortexDBClient } from "../cortexdb-client.js";

const client = new CortexDBClient(process.env.CORTEXDB_URL ?? "http://127.0.0.1:8181")
  .withRetries(2, 250)
  .withTimeout(15000);
const llmUrl = process.env.CORTEXDB_LLM_URL ?? "http://127.0.0.1:8000/v1/chat/completions";
const ask = (pack) => fetch(llmUrl, {
  method: "POST",
  headers: { "content-type": "application/json", authorization: `Bearer ${process.env.CORTEXDB_LLM_API_KEY ?? "dummy"}` },
  body: JSON.stringify({ model: process.env.CORTEXDB_LLM_MODEL ?? "local", messages: [{ role: "user", content: JSON.stringify(pack) }] }),
}).then((res) => res.json()).then((json) => json.choices?.[0]?.message?.content ?? "Not enough information.");
const answer = await client.answerWithGroundedContext("default", "default", process.argv.slice(2).join(" ") || "What is in context?", ask);
console.log(JSON.stringify(answer, null, 2));
