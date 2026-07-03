// cortexdb-client/errors.ts
var CortexDBError = class _CortexDBError extends Error {
  constructor(message, code = null, status = null, body = null) {
    super(message);
    this.code = code;
    this.status = status;
    this.body = body;
    this.name = "CortexDBError";
  }
  code;
  status;
  body;
  static async fromResponse(response) {
    const body = await response.text();
    try {
      const data = JSON.parse(body);
      return new _CortexDBError(
        String(data.message ?? body),
        data.code ? String(data.code) : null,
        response.status,
        body
      );
    } catch {
      return new _CortexDBError(body, null, response.status, body);
    }
  }
};

// cortexdb-client/aql.ts
function quoteAqlString(value) {
  return `"${value.replaceAll("\\", "\\\\").replaceAll('"', '\\"').replaceAll("\n", "\\n").replaceAll("\r", "\\r").replaceAll("	", "\\t")}"`;
}
function validateAqlIdentifier(field, value) {
  if (!/^[A-Za-z_][A-Za-z0-9_:-]*$/.test(value)) {
    throw new Error(`${field} must be an AQL identifier`);
  }
}
function validateDecimal(field, value) {
  if (value !== void 0 && !/^[0-9]+\.[0-9]+$/.test(value)) {
    throw new Error(`${field} must be a decimal literal`);
  }
}
function validateAqlMode(value) {
  if (value !== void 0 && value !== "fast" && value !== "balanced" && value !== "semantic" && value !== "audit") {
    throw new Error("mode must be fast, balanced, semantic, or audit");
  }
}
function buildRetrieveContextAql(task, brain, options = {}) {
  validateAqlIdentifier("brain", brain);
  validateAqlMode(options.mode);
  if (options.whereClause !== void 0 && options.whereClause.trim().length === 0) {
    throw new Error("whereClause must not be empty");
  }
  validateDecimal("minConfidence", options.minConfidence);
  validateDecimal("sourceTrust", options.sourceTrust);
  const parts = [];
  if (options.explain) parts.push("EXPLAIN");
  parts.push("RETRIEVE CONTEXT FOR TASK", quoteAqlString(task), "IN BRAIN", brain);
  if (options.mode) parts.push("USING MODE", options.mode);
  if (options.budgetTokens !== void 0) {
    parts.push("BUDGET", String(options.budgetTokens), "TOKENS");
  }
  if (options.limitCandidates !== void 0) {
    parts.push("LIMIT", String(options.limitCandidates), "CANDIDATES");
  }
  if (options.whereClause !== void 0) {
    parts.push("WHERE", options.whereClause.trim());
  }
  if (options.requireCitations) parts.push("REQUIRE", "citations");
  if (options.minConfidence !== void 0) {
    parts.push("REQUIRE", "confidence", ">=", options.minConfidence);
  }
  if (options.sourceTrust !== void 0) {
    parts.push("REQUIRE", "source_trust", ">=", options.sourceTrust);
  }
  if (options.freshnessSeconds !== void 0) {
    parts.push("REQUIRE", "freshness", "<=", String(options.freshnessSeconds), "SECONDS");
  }
  return `${parts.join(" ")};`;
}
function buildVerifyFactAql(fact, brain) {
  validateAqlIdentifier("brain", brain);
  return `VERIFY FACT ${quoteAqlString(fact)} IN BRAIN ${brain};`;
}
function buildRememberAql(content, scope, memoryType, ttlSeconds) {
  validateAqlIdentifier("scope", scope);
  validateAqlIdentifier("memoryType", memoryType);
  let statement = `REMEMBER ${quoteAqlString(content)} IN SCOPE ${scope} AS TYPE ${memoryType}`;
  if (ttlSeconds !== void 0) statement += ` TTL ${ttlSeconds} SECONDS`;
  return `${statement};`;
}

// cortexdb-client/grounding.ts
function uniqueValues(values) {
  const seen = /* @__PURE__ */ new Set();
  const out = [];
  for (const value of values) {
    if (!seen.has(value)) {
      seen.add(value);
      out.push(value);
    }
  }
  return out;
}
function tokenize(text) {
  const stopwords = /* @__PURE__ */ new Set(["a", "an", "and", "the", "or", "of", "to", "in"]);
  const terms = text.toLowerCase().split(/[^\p{L}\p{N}]+/u).filter((term) => term.length > 0 && !stopwords.has(term));
  return [...new Set(terms)].sort();
}
function splitAnswerSpans(answer) {
  const spans = [];
  let start = 0;
  for (let index = 0; index < answer.length; index += 1) {
    const ch = answer[index];
    const decimalDot = ch === "." && /\d/.test(answer[index - 1] ?? "") && /\d/.test(answer[index + 1] ?? "");
    if (ch === "!" || ch === "?" || ch === "\n" || ch === "." && !decimalDot) {
      pushAnswerSpan(answer, start, index + 1, spans);
      start = index + 1;
    }
  }
  pushAnswerSpan(answer, start, answer.length, spans);
  return spans;
}
function pushAnswerSpan(answer, start, end, spans) {
  const raw = answer.slice(start, end);
  const text = raw.trim();
  if (text.length === 0) return;
  const leading = raw.length - raw.trimStart().length;
  const trailing = raw.length - raw.trimEnd().length;
  spans.push([text, start + leading, end - trailing]);
}
function q16Ratio(numerator, denominator) {
  if (denominator === 0) return 65535;
  return Math.floor(numerator * 65535 / denominator);
}
function groundAnswer(context, answer, options = {}) {
  const minSpanSupportQ16 = options.minSpanSupportQ16 ?? 65535;
  const requireCitations = options.requireCitations ?? false;
  const rejectUnsupported = options.rejectUnsupported ?? false;
  const spans = splitAnswerSpans(answer).map(([text, start, end]) => {
    const spanTerms = tokenize(text);
    if (spanTerms.length === 0) {
      return {
        text,
        start_byte: start,
        end_byte: end,
        support_q16: 65535,
        supported: true,
        covered_terms: [],
        missing_terms: [],
        supported_by_cell_ids: [],
        citations: []
      };
    }
    const covered = /* @__PURE__ */ new Set();
    const cellIds = [];
    const citations = [];
    for (const cell of context.cells) {
      const cellTerms = new Set(tokenize(cell.payload_text));
      let matched = false;
      for (const term of spanTerms) {
        if (cellTerms.has(term)) {
          covered.add(term);
          matched = true;
        }
      }
      if (matched) {
        cellIds.push(cell.cell_id);
        if (cell.citation) citations.push(cell.citation);
      }
    }
    const support2 = q16Ratio(covered.size, spanTerms.length);
    const supported = support2 >= minSpanSupportQ16 && (!requireCitations || citations.length > 0);
    return {
      text,
      start_byte: start,
      end_byte: end,
      support_q16: support2,
      supported,
      covered_terms: [...covered].sort(),
      missing_terms: spanTerms.filter((term) => !covered.has(term)),
      supported_by_cell_ids: uniqueValues(cellIds),
      citations: uniqueValues(citations)
    };
  });
  const supportedSpanCount = spans.filter((span) => span.supported).length;
  const unsupportedSpanCount = spans.length - supportedSpanCount;
  const support = spans.length === 0 ? 65535 : Math.floor(spans.reduce((total, span) => total + span.support_q16, 0) / spans.length);
  return {
    answer_supported: unsupportedSpanCount === 0,
    rejected: rejectUnsupported && unsupportedSpanCount > 0,
    support_q16: support,
    supported_span_count: supportedSpanCount,
    unsupported_span_count: unsupportedSpanCount,
    spans
  };
}
function buildGroundedAnswerResponse(params) {
  const grounding = groundAnswer(params.context, params.answer, {
    requireCitations: params.requireCitations,
    minSpanSupportQ16: params.minSpanSupportQ16,
    rejectUnsupported: params.rejectUnsupported
  });
  const citations = uniqueValues([
    ...grounding.spans.flatMap((span) => span.citations),
    ...params.context.cells.flatMap((cell) => cell.citation ? [cell.citation] : [])
  ]);
  const usedContextCellIds = uniqueValues([
    ...grounding.spans.flatMap((span) => span.supported_by_cell_ids),
    ...params.context.cells.map((cell) => cell.cell_id)
  ]);
  return {
    question: params.question,
    answer: params.answer,
    retrieve_statement: params.retrieveStatement,
    verify_statement: params.verifyStatement ?? null,
    context: params.context,
    grounding,
    verification: params.verification ?? null,
    citations,
    used_context_cell_ids: usedContextCellIds,
    rejected: grounding.rejected
  };
}

// cortexdb-client/answering.ts
async function answerWithGroundedContext(client, scope, brain, question, answerer, options = {}) {
  const requireCitations = options.requireCitations ?? true;
  const retrieveStatement = buildRetrieveContextAql(question, brain, {
    ...options,
    requireCitations
  });
  const context = await client.retrieveContext(scope, retrieveStatement);
  const answer = await answerer(context);
  const verifyAnswer = options.verifyAnswer ?? true;
  const verifyStatement = verifyAnswer && answer.trim().length > 0 ? buildVerifyFactAql(answer, brain) : null;
  const verification = verifyStatement ? await client.verifyFact(scope, verifyStatement) : null;
  return buildGroundedAnswerResponse({
    question,
    answer,
    retrieveStatement,
    verifyStatement,
    context,
    verification,
    requireCitations,
    minSpanSupportQ16: options.minSpanSupportQ16,
    rejectUnsupported: options.rejectUnsupported
  });
}

// cortexdb-client/transport.ts
async function requestJson(options) {
  const url = `${options.baseUrl}${options.path}`;
  const body = encodeBody(options.body);
  let attempt = 0;
  while (true) {
    const controller = options.timeoutMs > 0 ? new AbortController() : null;
    const timeout = controller ? setTimeout(() => controller.abort(), options.timeoutMs) : null;
    try {
      const response = await options.fetch(url, buildInit(options, body, controller));
      if (!response.ok) {
        if (attempt < options.maxRetries && await isRetryableResponse(response)) {
          attempt += 1;
          await sleep(options.retryDelayMs * attempt);
          continue;
        }
        throw await CortexDBError.fromResponse(response);
      }
      return response.json();
    } catch (error) {
      if (error instanceof CortexDBError) throw error;
      if (attempt < options.maxRetries) {
        attempt += 1;
        await sleep(options.retryDelayMs * attempt);
        continue;
      }
      throw error;
    } finally {
      if (timeout) clearTimeout(timeout);
    }
  }
}
async function isRetryableResponse(response) {
  if (response.status === 502 || response.status === 504) return true;
  if (response.status !== 503) return false;
  const code = await responseErrorCode(response);
  return code === "database_busy" || code === "service_unavailable";
}
function scopedPath(path, tenant) {
  if (!tenant || tenant === "default") return path;
  const params = new URLSearchParams({ tenant });
  return `${path}${path.includes("?") ? "&" : "?"}${params.toString()}`;
}
function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
function buildInit(options, body, controller) {
  const headers = {};
  if (options.token) headers.authorization = `Bearer ${options.token}`;
  if (body !== void 0) headers["content-type"] = "application/json";
  return {
    method: options.method,
    headers,
    body,
    signal: controller?.signal
  };
}
function encodeBody(body) {
  if (body === void 0) return void 0;
  return typeof body === "string" ? body : JSON.stringify(body);
}
async function responseErrorCode(response) {
  try {
    const text = await response.clone().text();
    const data = JSON.parse(text);
    return data.code || data.error ? String(data.code ?? data.error) : null;
  } catch {
    return null;
  }
}

// cortexdb-client/client.ts
var CortexDBClient = class _CortexDBClient {
  constructor(baseUrl = "http://127.0.0.1:8181", token, tenant, maxRetries = 0, retryDelayMs = 500, timeoutMs = 1e4, fetchImpl = globalThis.fetch.bind(globalThis)) {
    this.baseUrl = baseUrl;
    this.token = token;
    this.tenant = tenant;
    this.maxRetries = maxRetries;
    this.retryDelayMs = retryDelayMs;
    this.timeoutMs = timeoutMs;
    this.fetchImpl = fetchImpl;
  }
  baseUrl;
  token;
  tenant;
  maxRetries;
  retryDelayMs;
  timeoutMs;
  fetchImpl;
  withTenant(tenant) {
    return this.copy({ tenant });
  }
  withRetries(maxRetries, retryDelayMs = 500) {
    return this.copy({ maxRetries, retryDelayMs });
  }
  withTimeout(timeoutMs) {
    return this.copy({ timeoutMs });
  }
  withOptions(options) {
    return this.copy({ timeoutMs: options.timeoutMs, fetchImpl: options.fetch });
  }
  withSession() {
    return this;
  }
  close() {
  }
  buildRetrieveContextAql(task, brain, options = {}) {
    return buildRetrieveContextAql(task, brain, options);
  }
  buildVerifyFactAql(fact, brain) {
    return buildVerifyFactAql(fact, brain);
  }
  buildRememberAql(content, scope, memoryType, ttlSeconds) {
    return buildRememberAql(content, scope, memoryType, ttlSeconds);
  }
  health() {
    return this.request("GET", "/v1/health");
  }
  putCell(cellId, payload) {
    return this.request("POST", this.path("/v1/cell", { cell_id: cellId }), payload);
  }
  getCell(cellId) {
    return this.request("GET", this.path("/v1/cell", { cell_id: cellId }));
  }
  tombstoneCell(cellId) {
    return this.request("DELETE", this.path("/v1/cell", { cell_id: cellId }));
  }
  flush() {
    return this.request("POST", "/v1/flush");
  }
  compact() {
    return this.request("POST", "/v1/compact");
  }
  /**
   * Commit an optimistic-concurrency agent transaction (F04-B6.3). A conflict is
   * a normal response with `outcome === "conflict"`, not an HTTP error — read
   * `outcome` rather than relying on the status code.
   */
  agentTransaction(request) {
    return this.request("POST", "/v1/transactions", JSON.stringify(request));
  }
  /** Commit a durable SharedSequenced agent handoff (F04-B6.3 / F08-B6.1). */
  agentHandoff(request) {
    return this.request("POST", "/v1/handoff", JSON.stringify(request));
  }
  search(scope, query, limit = 20) {
    return this.request("POST", this.path("/v1/search", {
      scope,
      mode: "keyword",
      q: query,
      limit
    }));
  }
  searchVector(scope, vector, limit = 20, algorithm = "ann") {
    return this.request("POST", this.path("/v1/search", {
      scope,
      mode: "vector",
      algorithm,
      vector: vector.join(","),
      limit
    }));
  }
  evaluateAnn(scope, vector, limit = 20) {
    return this.request("POST", this.path("/v1/search/ann-evaluate", {
      scope,
      vector: vector.join(","),
      limit
    }));
  }
  aql(scope, statement) {
    return this.request("POST", this.path("/v1/aql", { scope }), statement);
  }
  retrieveContext(scope, statement) {
    return this.request("POST", this.path("/v1/context", { scope }), statement);
  }
  async answerWithGroundedContext(scope, brain, question, answerer, options = {}) {
    return answerWithGroundedContext(this, scope, brain, question, answerer, options);
  }
  verifyFact(scope, statement) {
    return this.request("POST", this.path("/v1/verify", { scope }), statement);
  }
  remember(scope, statement) {
    return this.request("POST", this.path("/v1/remember", { scope }), statement);
  }
  ingestText(scope, text, source = "typescript_sdk") {
    return this.request("POST", this.path("/v1/ingest/text", {
      scope,
      source
    }), text);
  }
  ingestJson(scope, document, source = "typescript_sdk") {
    return this.request("POST", this.path("/v1/ingest/json", {
      scope,
      source
    }), document);
  }
  ingestCsv(scope, document, source = "typescript_sdk") {
    return this.request("POST", this.path("/v1/ingest/csv", {
      scope,
      source
    }), document);
  }
  ingestionJob(jobId) {
    return this.request("GET", `/v1/ingest/jobs/${jobId}`);
  }
  ingestionJobResponse(jobId) {
    return this.request("GET", `/v1/ingest/jobs/${jobId}`);
  }
  deleteIngestionJob(jobId) {
    return this.request("DELETE", `/v1/ingest/jobs/${jobId}`);
  }
  retryIngestionJob(jobId) {
    return this.request("POST", `/v1/ingest/jobs/${jobId}/retry`);
  }
  validate() {
    return this.request("GET", "/v1/validate");
  }
  stats() {
    return this.request("GET", "/v1/stats");
  }
  async request(method, path, body) {
    return requestJson({
      baseUrl: this.baseUrl,
      path: this.scoped(path),
      method,
      token: this.token,
      body,
      maxRetries: this.maxRetries,
      retryDelayMs: this.retryDelayMs,
      timeoutMs: this.timeoutMs,
      fetch: this.fetchImpl
    });
  }
  path(path, query) {
    const params = new URLSearchParams();
    for (const [key, value] of Object.entries(query)) {
      params.set(key, String(value));
    }
    return `${path}?${params.toString()}`;
  }
  scoped(path) {
    return scopedPath(path, this.tenant);
  }
  copy(overrides) {
    return new _CortexDBClient(
      this.baseUrl,
      this.token,
      overrides.tenant ?? this.tenant,
      overrides.maxRetries ?? this.maxRetries,
      overrides.retryDelayMs ?? this.retryDelayMs,
      overrides.timeoutMs ?? this.timeoutMs,
      overrides.fetchImpl ?? this.fetchImpl
    );
  }
};
export {
  CortexDBClient,
  CortexDBError,
  buildGroundedAnswerResponse,
  buildRememberAql,
  buildRetrieveContextAql,
  buildVerifyFactAql,
  groundAnswer
};
