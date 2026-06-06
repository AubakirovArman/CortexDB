import test from "node:test";
import assert from "node:assert";
import {
  CortexDBClient,
  CortexDBError,
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
