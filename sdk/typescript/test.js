import test from "node:test";
import assert from "node:assert";
import {
  CortexDBClient,
  CortexDBError,
  groundAnswer,
  buildRememberAql,
  buildRetrieveContextAql,
  buildVerifyFactAql,
} from "./cortexdb-client.js";

test("CortexDBClient path helper", () => {
  const client = new CortexDBClient();
  const path = client.path("/v1/search", {
    scope: "project:investments",
    mode: "keyword",
    q: "solar budget",
    limit: 10,
  });
  assert.strictEqual(
    path,
    "/v1/search?scope=project%3Ainvestments&mode=keyword&q=solar+budget&limit=10"
  );
});

test("AQL builder helpers output stable statements", () => {
  const retrieve = buildRetrieveContextAql('budget "audit"\nline', "investment_projects", {
    mode: "balanced",
    budgetTokens: 2048,
    limitCandidates: 10,
    whereClause: 'space = project:investments AND status = "ready"',
    requireCitations: true,
    minConfidence: "0.80",
    sourceTrust: "0.90",
    freshnessSeconds: 86400,
  });
  const client = new CortexDBClient();

  assert.strictEqual(
    retrieve,
    [
      'RETRIEVE CONTEXT FOR TASK "budget \\"audit\\"\\nline"',
      "IN BRAIN investment_projects USING MODE balanced BUDGET 2048 TOKENS",
      'LIMIT 10 CANDIDATES WHERE space = project:investments AND status = "ready"',
      "REQUIRE citations REQUIRE confidence >= 0.80 REQUIRE source_trust >= 0.90",
      "REQUIRE freshness <= 86400 SECONDS;",
    ].join(" "),
  );
  assert.strictEqual(
    buildVerifyFactAql("Solar Plant budget is 1.2B KZT", "investment_projects"),
    'VERIFY FACT "Solar Plant budget is 1.2B KZT" IN BRAIN investment_projects;',
  );
  assert.strictEqual(
    client.buildRememberAql(
      "Use conservative budget assumptions",
      "project:investments",
      "decision",
      3600,
    ),
    'REMEMBER "Use conservative budget assumptions" IN SCOPE project:investments AS TYPE decision TTL 3600 SECONDS;',
  );
});

test("AQL builder helpers reject invalid inputs", () => {
  assert.throws(() => buildVerifyFactAql("x", "bad brain"), /AQL identifier/);
  assert.throws(
    () => buildRememberAql("x", "project:investments", "bad type"),
    /AQL identifier/,
  );
  assert.throws(
    () => buildRetrieveContextAql("x", "brain", { whereClause: " " }),
    /whereClause/,
  );
  assert.throws(
    () => buildRetrieveContextAql("x", "brain", { minConfidence: "0" }),
    /decimal/,
  );
  assert.throws(
    () => buildRetrieveContextAql("x", "brain", { mode: "turbo" }),
    /mode/,
  );
});

test("grounded answer helper builds context verify and citations", async () => {
  const context = {
    schema_version: "context_pack.v1",
    token_budget_tokens: 2048,
    estimated_tokens: 18,
    truncated: false,
    citations_required: true,
    cells: [
      {
        cell_id: 7,
        estimated_tokens: 18,
        citation: "doc://solar#1",
        payload_text: "Solar budget was approved by finance.",
        explain: null,
        source_ref: null,
      },
    ],
    anomalies: [],
  };

  const grounding = groundAnswer(context, "Solar budget was approved.", {
    requireCitations: true,
  });
  assert.strictEqual(grounding.answer_supported, true);
  assert.deepStrictEqual(grounding.spans[0].supported_by_cell_ids, [7]);
  assert.deepStrictEqual(grounding.spans[0].citations, ["doc://solar#1"]);

  class FakeClient extends CortexDBClient {
    calls = [];

    async retrieveContext(scope, statement) {
      this.calls.push(["context", scope, statement]);
      return context;
    }

    async verifyFact(scope, statement) {
      this.calls.push(["verify", scope, statement]);
      return {
        fact: "Solar budget was approved.",
        status: "supported",
        verdict: "supported",
        confidence_q16: 60000,
        evidence: [],
        contradicting_evidence: [],
        guards: [],
        supporting: [],
        contradicting: [],
        numeric_conflicts: [],
      };
    }
  }

  const client = new FakeClient();
  const response = await client.answerWithGroundedContext(
    "project:investments",
    "investment_projects",
    "Was the solar budget approved?",
    (ctx) => `${ctx.cells[0].payload_text}`,
    {
      mode: "audit",
      budgetTokens: 2048,
      limitCandidates: 5,
    },
  );

  assert.strictEqual(response.answer, "Solar budget was approved by finance.");
  assert.strictEqual(response.context, context);
  assert.strictEqual(response.verification.status, "supported");
  assert.strictEqual(response.verification.confidence_q16, 60000);
  assert.strictEqual(response.grounding.answer_supported, true);
  assert.deepStrictEqual(response.citations, ["doc://solar#1"]);
  assert.deepStrictEqual(response.used_context_cell_ids, [7]);
  assert.strictEqual(response.calls, undefined);
  assert.strictEqual(client.calls.length, 2);
  assert.strictEqual(client.calls[0][0], "context");
  assert.match(client.calls[0][2], /RETRIEVE CONTEXT FOR TASK/);
  assert.match(client.calls[1][2], /VERIFY FACT/);
});

test("CortexDBClient scoped helper", () => {
  const client = new CortexDBClient("http://127.0.0.1:8181", undefined, "tenant:alpha");
  assert.strictEqual(
    client.scoped("/v1/stats"),
    "/v1/stats?tenant=tenant%3Aalpha"
  );
  assert.strictEqual(
    client.scoped("/v1/search?scope=project%3Ainvestments"),
    "/v1/search?scope=project%3Ainvestments&tenant=tenant%3Aalpha"
  );
});

test("CortexDBClient decodes mock contract", () => {
  const response = {
    search_mode: "vector_ann",
    ann_report: {
      path: "exact_fallback",
      fallback_reason: "no_persisted_segments",
      fallback_performed: true,
      requested_limit: 20,
      allowed_candidates: 1,
      graph_nodes: 0,
      returned_candidates: 1,
      recall_q16: null,
      min_recall_q16: null,
      hnsw_ef_construction: 64,
      require_slo: true,
      production_safe: false,
      slo_violations: ["no_persisted_segments"],
    },
    results: [
      {
        cell_id: 1,
        score: 42,
        lexical_score: 0,
        vector_score: 42,
        payload: "scope=default\nstatus=ready\nhello",
      },
    ],
  };

  assert.strictEqual(response.search_mode, "vector_ann");
  assert.strictEqual(response.results[0].cell_id, 1);
  assert.strictEqual(response.ann_report.fallback_reason, "no_persisted_segments");
  assert.strictEqual(response.ann_report.recall_q16, null);
  assert.strictEqual(response.ann_report.hnsw_ef_construction, 64);
  assert.strictEqual(response.ann_report.production_safe, false);
});

test("CortexDBClient retries database_busy and propagates timeout signal", async () => {
  const calls = [];
  const fetchImpl = async (url, init) => {
    calls.push({ url: String(url), init });
    if (calls.length === 1) {
      return new Response(
        JSON.stringify({ code: "database_busy", message: "busy" }),
        { status: 503 },
      );
    }
    return new Response(
      JSON.stringify({ status: "ok", version: "test", server_version: "test" }),
      { status: 200 },
    );
  };
  const client = new CortexDBClient("http://127.0.0.1:8181")
    .withRetries(1, 0)
    .withTimeout(2500)
    .withOptions({ fetch: fetchImpl });

  const response = await client.health();

  assert.strictEqual(response.status, "ok");
  assert.strictEqual(calls.length, 2);
  assert.strictEqual(calls[0].url, "http://127.0.0.1:8181/v1/health");
  assert.ok(calls[0].init.signal instanceof AbortSignal);
});

test("CortexDBClient does not retry generic internal errors", async () => {
  let calls = 0;
  const fetchImpl = async () => {
    calls += 1;
    return new Response(
      JSON.stringify({ code: "internal", message: "broken" }),
      { status: 500 },
    );
  };
  const client = new CortexDBClient("http://127.0.0.1:8181")
    .withRetries(3, 0)
    .withOptions({ fetch: fetchImpl });

  await assert.rejects(() => client.health(), /broken/);
  assert.strictEqual(calls, 1);
});

test("CortexDBError decodes full Core Alpha taxonomy", async () => {
  const codes = [
    "not_found",
    "bad_request",
    "unauthorized",
    "forbidden",
    "payload_too_large",
    "rate_limited",
    "service_unavailable",
    "internal",
    "invalid_aql",
    "permission_denied",
    "database_busy",
    "storage_corruption",
    "invalid_tenant",
  ];

  for (const code of codes) {
    const error = await CortexDBError.fromResponse(
      new Response(
        JSON.stringify({ code, error: code, message: "message" }),
        { status: 400 },
      ),
    );
    assert.strictEqual(error.code, code);
    assert.strictEqual(error.status, 400);
    assert.ok(error.body?.includes(code));
  }
});

test("Dashboard console endpoint mapping contract", () => {
  // Simulates front-end mapping of forms to client actions
  const forms = {
    search: { scope: "project:investments", mode: "keyword", q: "budget", limit: 20 },
    ann_eval: { scope: "project:investments", vector: [0, 10], limit: 20 },
    ingest: { scope: "project:investments", source: "dashboard", type: "text", document: "Solar Plant budget approved." },
  };

  const client = new CortexDBClient();
  const searchPath = client.path("/v1/search", {
    scope: forms.search.scope,
    mode: forms.search.mode,
    q: forms.search.q,
    limit: forms.search.limit,
  });
  assert.strictEqual(
    searchPath,
    "/v1/search?scope=project%3Ainvestments&mode=keyword&q=budget&limit=20"
  );

  const evalPath = client.path("/v1/search/ann-evaluate", {
    scope: forms.ann_eval.scope,
    vector: forms.ann_eval.vector.join(","),
    limit: forms.ann_eval.limit,
  });
  assert.strictEqual(
    evalPath,
    "/v1/search/ann-evaluate?scope=project%3Ainvestments&vector=0%2C10&limit=20"
  );

  const ingestPath = client.path("/v1/ingest/" + forms.ingest.type, {
    scope: forms.ingest.scope,
    source: forms.ingest.source,
  });
  assert.strictEqual(
    ingestPath,
    "/v1/ingest/text?scope=project%3Ainvestments&source=dashboard"
  );
});
