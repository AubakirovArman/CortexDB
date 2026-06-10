export type JsonObject = Record<string, unknown>;

export class CortexDBError extends Error {
  constructor(
    message: string,
    public readonly code: string | null = null,
    public readonly status: number | null = null,
    public readonly body: string | null = null,
  ) {
    super(message);
    this.name = "CortexDBError";
  }

  static async fromResponse(response: Response): Promise<CortexDBError> {
    const body = await response.text();
    try {
      const data = JSON.parse(body) as JsonObject;
      return new CortexDBError(
        String(data.message ?? body),
        data.code ? String(data.code) : null,
        response.status,
        body,
      );
    } catch {
      return new CortexDBError(body, null, response.status, body);
    }
  }
}
export type VectorAlgorithm = "ann" | "exact";
export type SearchMode = "keyword" | "vector_exact" | "vector_ann";
export type AnnSearchPath = "hnsw_graph" | "exact_fallback";
export type AnnFallbackReason =
  | "empty_graph"
  | "invalid_graph"
  | "insufficient_results"
  | "low_recall"
  | "visit_budget_exceeded"
  | "no_persisted_segments"
  | "uncheckpointed_changes";

export interface AnnSearchReport {
  path: AnnSearchPath;
  fallback_reason: AnnFallbackReason | null;
  fallback_performed: boolean;
  requested_limit: number;
  allowed_candidates: number;
  graph_nodes: number;
  returned_candidates: number;
  recall_q16: number | null;
  min_recall_q16: number | null;
  hnsw_ef_construction: number;
  require_slo: boolean;
  production_safe: boolean;
  slo_violations: string[];
}

export interface SearchResult {
  cell_id: number;
  score: number;
  lexical_score: number;
  vector_score: number;
  payload: string;
}

export interface SearchResponse {
  search_mode: SearchMode;
  ann_report: AnnSearchReport | null;
  results: SearchResult[];
}

export interface AnnEvaluationResponse {
  available: boolean;
  reason: "requires_persisted_checkpoint_without_wal_tail" | null;
  ann_report: AnnSearchReport | null;
  exact_top_k: number[];
  ann_top_k: number[];
  overlap_count: number;
  recall_q16: number;
}

export interface HealthResponse {
  status: string;
  version: string;
  server_version: string;
}

export interface StatsResponse {
  current_seq: number;
  checkpoint_seq: number;
  live_segments: number;
  retired_segments: number;
  memtable_cells: number;
  memtable_versions: number;
  wal_size_bytes: number;
  wal_writer_records: number;
  wal_writer_bytes: number;
  wal_writer_fsyncs: number;
  wal_writer_batches: number;
}

export interface ValidationResponse {
  ok: boolean;
  manifest_ok: boolean;
  wal_ok: boolean;
  live_segments_checked: number;
  bitmap_indexes_checked: number;
  lexical_indexes_checked: number;
  vector_indexes_checked: number;
  hnsw_graphs_checked: number;
  cells_checked: number;
  wal_records_checked: number;
  wal_safe_truncate_offset: number;
  errors: string[];
}

export interface PutCellResponse {
  seq: number;
  cell_id: number;
}

export interface CellResponse {
  cell_id: number;
  payload: string;
}

export interface CellLookupResponse {
  cell: CellResponse | null;
}

export interface AqlCellResponse {
  cell_id: number;
  payload: string;
}

export interface AqlResponse {
  cells: AqlCellResponse[];
}

export interface ExplainResponse {
  score: number;
  matched_terms: string[];
  why_selected: string;
  base_bm25: number;
  source_trust_bonus: number;
  redundancy_penalty: number;
}

export interface SourceRefResponse {
  source_id: string;
  document_id: string | null;
  page: number | null;
  cell_range: string | null;
  json_path: string | null;
  confidence_q16: number;
}

export interface ContextPackCellResponse {
  cell_id: number;
  estimated_tokens: number;
  citation: string | null;
  payload_text: string;
  explain: ExplainResponse | null;
  source_ref: SourceRefResponse | null;
}

export interface ContextPackAnomalyResponse {
  cell_id: number | null;
  code: string;
  message: string;
}

export interface ContextPackResponse {
  schema_version: string;
  token_budget_tokens: number;
  estimated_tokens: number;
  truncated: boolean;
  citations_required: boolean;
  cells: ContextPackCellResponse[];
  anomalies: ContextPackAnomalyResponse[];
}

export interface AnswerGroundingSpanResponse {
  text: string;
  start_byte: number;
  end_byte: number;
  support_q16: number;
  supported: boolean;
  covered_terms: string[];
  missing_terms: string[];
  supported_by_cell_ids: number[];
  citations: string[];
}

export interface AnswerGroundingReportResponse {
  answer_supported: boolean;
  rejected: boolean;
  support_q16: number;
  supported_span_count: number;
  unsupported_span_count: number;
  spans: AnswerGroundingSpanResponse[];
}

export interface GroundedAnswerOptions extends RetrieveContextAqlOptions {
  minSpanSupportQ16?: number;
  rejectUnsupported?: boolean;
  verifyAnswer?: boolean;
}

export interface GroundedAnswerResponse {
  question: string;
  answer: string;
  retrieve_statement: string;
  verify_statement: string | null;
  context: ContextPackResponse;
  grounding: AnswerGroundingReportResponse;
  verification: VerificationReportResponse | null;
  citations: string[];
  used_context_cell_ids: number[];
  rejected: boolean;
}

export interface EvidenceResponse {
  cell_id: number;
  matched_terms: number;
  source_trust_q16: number;
  citation: string | null;
  payload_text: string;
}

export interface GuardResponse {
  cell_id: number | null;
  code: string;
  message: string;
}

export interface NumericConflictResponse {
  metric: string;
  left: string;
  right: string;
}

export interface VerificationReportResponse {
  fact: string;
  status: string;
  verdict: string;
  confidence_q16: number;
  evidence: EvidenceResponse[];
  contradicting_evidence: EvidenceResponse[];
  guards: GuardResponse[];
  supporting: EvidenceResponse[];
  contradicting: EvidenceResponse[];
  numeric_conflicts: NumericConflictResponse[];
}

export interface IngestResponse {
  rows_ingested: number;
  chunks_ingested: number;
  facts_ingested: number;
  first_cell_id: number | null;
  job_id: number | null;
}

export type IngestionJobStatus = "queued" | "running" | "completed" | "failed" | "cancelled";

export interface IngestionJobResponse {
  job_id: number;
  label: string;
  status: IngestionJobStatus;
  total_items: number | null;
  completed_items: number;
  failed_items: number;
  last_cell_id: number | null;
  message: string | null;
  retry_count: number;
  max_retries: number;
}

export interface DeleteJobResponse {
  deleted: boolean;
}

export interface RememberResponse {
  seq: number;
  cell_id: number;
  ttl_seconds: number | null;
}

export type AqlRetrievalMode = "fast" | "balanced" | "semantic" | "audit";

export interface RetrieveContextAqlOptions {
  mode?: AqlRetrievalMode;
  budgetTokens?: number;
  limitCandidates?: number;
  whereClause?: string;
  requireCitations?: boolean;
  minConfidence?: string;
  sourceTrust?: string;
  freshnessSeconds?: number;
  explain?: boolean;
}

function quoteAqlString(value: string): string {
  return `"${value
    .replaceAll("\\", "\\\\")
    .replaceAll("\"", "\\\"")
    .replaceAll("\n", "\\n")
    .replaceAll("\r", "\\r")
    .replaceAll("\t", "\\t")}"`;
}

function validateAqlIdentifier(field: string, value: string): void {
  if (!/^[A-Za-z_][A-Za-z0-9_:-]*$/.test(value)) {
    throw new Error(`${field} must be an AQL identifier`);
  }
}

function validateDecimal(field: string, value: string | undefined): void {
  if (value !== undefined && !/^[0-9]+\.[0-9]+$/.test(value)) {
    throw new Error(`${field} must be a decimal literal`);
  }
}

function validateAqlMode(value: AqlRetrievalMode | undefined): void {
  if (
    value !== undefined &&
    value !== "fast" &&
    value !== "balanced" &&
    value !== "semantic" &&
    value !== "audit"
  ) {
    throw new Error("mode must be fast, balanced, semantic, or audit");
  }
}

export function buildRetrieveContextAql(
  task: string,
  brain: string,
  options: RetrieveContextAqlOptions = {},
): string {
  validateAqlIdentifier("brain", brain);
  validateAqlMode(options.mode);
  if (options.whereClause !== undefined && options.whereClause.trim().length === 0) {
    throw new Error("whereClause must not be empty");
  }
  validateDecimal("minConfidence", options.minConfidence);
  validateDecimal("sourceTrust", options.sourceTrust);

  const parts: string[] = [];
  if (options.explain) parts.push("EXPLAIN");
  parts.push("RETRIEVE CONTEXT FOR TASK", quoteAqlString(task), "IN BRAIN", brain);
  if (options.mode) parts.push("USING MODE", options.mode);
  if (options.budgetTokens !== undefined) {
    parts.push("BUDGET", String(options.budgetTokens), "TOKENS");
  }
  if (options.limitCandidates !== undefined) {
    parts.push("LIMIT", String(options.limitCandidates), "CANDIDATES");
  }
  if (options.whereClause !== undefined) {
    parts.push("WHERE", options.whereClause.trim());
  }
  if (options.requireCitations) parts.push("REQUIRE", "citations");
  if (options.minConfidence !== undefined) {
    parts.push("REQUIRE", "confidence", ">=", options.minConfidence);
  }
  if (options.sourceTrust !== undefined) {
    parts.push("REQUIRE", "source_trust", ">=", options.sourceTrust);
  }
  if (options.freshnessSeconds !== undefined) {
    parts.push("REQUIRE", "freshness", "<=", String(options.freshnessSeconds), "SECONDS");
  }
  return `${parts.join(" ")};`;
}

export function buildVerifyFactAql(fact: string, brain: string): string {
  validateAqlIdentifier("brain", brain);
  return `VERIFY FACT ${quoteAqlString(fact)} IN BRAIN ${brain};`;
}

export function buildRememberAql(
  content: string,
  scope: string,
  memoryType: string,
  ttlSeconds?: number,
): string {
  validateAqlIdentifier("scope", scope);
  validateAqlIdentifier("memoryType", memoryType);
  let statement = `REMEMBER ${quoteAqlString(content)} IN SCOPE ${scope} AS TYPE ${memoryType}`;
  if (ttlSeconds !== undefined) statement += ` TTL ${ttlSeconds} SECONDS`;
  return `${statement};`;
}

function uniqueValues<T>(values: T[]): T[] {
  const seen = new Set<T>();
  const out: T[] = [];
  for (const value of values) {
    if (!seen.has(value)) {
      seen.add(value);
      out.push(value);
    }
  }
  return out;
}

function tokenize(text: string): string[] {
  const stopwords = new Set(["a", "an", "and", "the", "or", "of", "to", "in"]);
  const terms = text
    .toLowerCase()
    .split(/[^\p{L}\p{N}]+/u)
    .filter((term) => term.length > 0 && !stopwords.has(term));
  return [...new Set(terms)].sort();
}

function splitAnswerSpans(answer: string): Array<[string, number, number]> {
  const spans: Array<[string, number, number]> = [];
  let start = 0;
  for (let index = 0; index < answer.length; index += 1) {
    const ch = answer[index];
    const decimalDot = ch === "." && /\d/.test(answer[index - 1] ?? "") && /\d/.test(answer[index + 1] ?? "");
    if (ch === "!" || ch === "?" || ch === "\n" || (ch === "." && !decimalDot)) {
      pushAnswerSpan(answer, start, index + 1, spans);
      start = index + 1;
    }
  }
  pushAnswerSpan(answer, start, answer.length, spans);
  return spans;
}

function pushAnswerSpan(
  answer: string,
  start: number,
  end: number,
  spans: Array<[string, number, number]>,
): void {
  const raw = answer.slice(start, end);
  const text = raw.trim();
  if (text.length === 0) return;
  const leading = raw.length - raw.trimStart().length;
  const trailing = raw.length - raw.trimEnd().length;
  spans.push([text, start + leading, end - trailing]);
}

function q16Ratio(numerator: number, denominator: number): number {
  if (denominator === 0) return 65535;
  return Math.floor((numerator * 65535) / denominator);
}

export function groundAnswer(
  context: ContextPackResponse,
  answer: string,
  options: {
    minSpanSupportQ16?: number;
    requireCitations?: boolean;
    rejectUnsupported?: boolean;
  } = {},
): AnswerGroundingReportResponse {
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
        citations: [],
      };
    }
    const covered = new Set<string>();
    const cellIds: number[] = [];
    const citations: string[] = [];
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
    const support = q16Ratio(covered.size, spanTerms.length);
    const supported = support >= minSpanSupportQ16 && (!requireCitations || citations.length > 0);
    return {
      text,
      start_byte: start,
      end_byte: end,
      support_q16: support,
      supported,
      covered_terms: [...covered].sort(),
      missing_terms: spanTerms.filter((term) => !covered.has(term)),
      supported_by_cell_ids: uniqueValues(cellIds),
      citations: uniqueValues(citations),
    };
  });
  const supportedSpanCount = spans.filter((span) => span.supported).length;
  const unsupportedSpanCount = spans.length - supportedSpanCount;
  const support = spans.length === 0
    ? 65535
    : Math.floor(spans.reduce((total, span) => total + span.support_q16, 0) / spans.length);
  return {
    answer_supported: unsupportedSpanCount === 0,
    rejected: rejectUnsupported && unsupportedSpanCount > 0,
    support_q16: support,
    supported_span_count: supportedSpanCount,
    unsupported_span_count: unsupportedSpanCount,
    spans,
  };
}

export function buildGroundedAnswerResponse(params: {
  question: string;
  answer: string;
  retrieveStatement: string;
  verifyStatement?: string | null;
  context: ContextPackResponse;
  verification?: VerificationReportResponse | null;
  requireCitations?: boolean;
  minSpanSupportQ16?: number;
  rejectUnsupported?: boolean;
}): GroundedAnswerResponse {
  const grounding = groundAnswer(params.context, params.answer, {
    requireCitations: params.requireCitations,
    minSpanSupportQ16: params.minSpanSupportQ16,
    rejectUnsupported: params.rejectUnsupported,
  });
  const citations = uniqueValues([
    ...grounding.spans.flatMap((span) => span.citations),
    ...params.context.cells.flatMap((cell) => cell.citation ? [cell.citation] : []),
  ]);
  const usedContextCellIds = uniqueValues([
    ...grounding.spans.flatMap((span) => span.supported_by_cell_ids),
    ...params.context.cells.map((cell) => cell.cell_id),
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
    rejected: grounding.rejected,
  };
}

export class CortexDBClient {
  constructor(
    private readonly baseUrl = "http://127.0.0.1:8181",
    private readonly token?: string,
    private readonly tenant?: string,
    private readonly maxRetries = 0,
    private readonly retryDelayMs = 500,
  ) {}

  withTenant(tenant: string): CortexDBClient {
    return new CortexDBClient(this.baseUrl, this.token, tenant, this.maxRetries, this.retryDelayMs);
  }

  withRetries(maxRetries: number, retryDelayMs = 500): CortexDBClient {
    return new CortexDBClient(this.baseUrl, this.token, this.tenant, maxRetries, retryDelayMs);
  }

  buildRetrieveContextAql(
    task: string,
    brain: string,
    options: RetrieveContextAqlOptions = {},
  ): string {
    return buildRetrieveContextAql(task, brain, options);
  }

  buildVerifyFactAql(fact: string, brain: string): string {
    return buildVerifyFactAql(fact, brain);
  }

  buildRememberAql(content: string, scope: string, memoryType: string, ttlSeconds?: number): string {
    return buildRememberAql(content, scope, memoryType, ttlSeconds);
  }

  health(): Promise<HealthResponse> {
    return this.request("GET", "/v1/health");
  }

  putCell(cellId: number, payload: string): Promise<PutCellResponse> {
    return this.request("POST", this.path("/v1/cell", { cell_id: cellId }), payload);
  }

  getCell(cellId: number): Promise<CellLookupResponse> {
    return this.request("GET", this.path("/v1/cell", { cell_id: cellId }));
  }

  tombstoneCell(cellId: number): Promise<JsonObject> {
    return this.request("DELETE", this.path("/v1/cell", { cell_id: cellId }));
  }

  flush(): Promise<JsonObject> {
    return this.request("POST", "/v1/flush");
  }

  compact(): Promise<JsonObject> {
    return this.request("POST", "/v1/compact");
  }

  search(scope: string, query: string, limit = 20): Promise<SearchResponse> {
    return this.request("POST", this.path("/v1/search", {
      scope,
      mode: "keyword",
      q: query,
      limit,
    }));
  }

  searchVector(
    scope: string,
    vector: number[],
    limit = 20,
    algorithm: VectorAlgorithm = "ann",
  ): Promise<SearchResponse> {
    return this.request("POST", this.path("/v1/search", {
      scope,
      mode: "vector",
      algorithm,
      vector: vector.join(","),
      limit,
    }));
  }

  evaluateAnn(
    scope: string,
    vector: number[],
    limit = 20,
  ): Promise<AnnEvaluationResponse> {
    return this.request("POST", this.path("/v1/search/ann-evaluate", {
      scope,
      vector: vector.join(","),
      limit,
    }));
  }

  aql(scope: string, statement: string): Promise<AqlResponse> {
    return this.request("POST", this.path("/v1/aql", { scope }), statement);
  }

  retrieveContext(scope: string, statement: string): Promise<ContextPackResponse> {
    return this.request("POST", this.path("/v1/context", { scope }), statement);
  }

  async answerWithGroundedContext(
    scope: string,
    brain: string,
    question: string,
    answerer: (context: ContextPackResponse) => string | Promise<string>,
    options: GroundedAnswerOptions = {},
  ): Promise<GroundedAnswerResponse> {
    const requireCitations = options.requireCitations ?? true;
    const retrieveStatement = buildRetrieveContextAql(question, brain, {
      ...options,
      requireCitations,
    });
    const context = await this.retrieveContext(scope, retrieveStatement);
    const answer = await answerer(context);
    const verifyAnswer = options.verifyAnswer ?? true;
    const verifyStatement = verifyAnswer && answer.trim().length > 0
      ? buildVerifyFactAql(answer, brain)
      : null;
    const verification = verifyStatement ? await this.verifyFact(scope, verifyStatement) : null;
    return buildGroundedAnswerResponse({
      question,
      answer,
      retrieveStatement,
      verifyStatement,
      context,
      verification,
      requireCitations,
      minSpanSupportQ16: options.minSpanSupportQ16,
      rejectUnsupported: options.rejectUnsupported,
    });
  }

  verifyFact(scope: string, statement: string): Promise<VerificationReportResponse> {
    return this.request("POST", this.path("/v1/verify", { scope }), statement);
  }

  remember(scope: string, statement: string): Promise<RememberResponse> {
    return this.request("POST", this.path("/v1/remember", { scope }), statement);
  }

  ingestText(scope: string, text: string, source = "typescript_sdk"): Promise<IngestResponse> {
    return this.request("POST", this.path("/v1/ingest/text", {
      scope,
      source,
    }), text);
  }

  ingestJson(scope: string, document: string, source = "typescript_sdk"): Promise<IngestResponse> {
    return this.request("POST", this.path("/v1/ingest/json", {
      scope,
      source,
    }), document);
  }

  ingestCsv(scope: string, document: string, source = "typescript_sdk"): Promise<IngestResponse> {
    return this.request("POST", this.path("/v1/ingest/csv", {
      scope,
      source,
    }), document);
  }

  ingestionJob(jobId: number): Promise<JsonObject> {
    return this.request("GET", `/v1/ingest/jobs/${jobId}`);
  }

  ingestionJobResponse(jobId: number): Promise<IngestionJobResponse> {
    return this.request("GET", `/v1/ingest/jobs/${jobId}`);
  }

  deleteIngestionJob(jobId: number): Promise<DeleteJobResponse> {
    return this.request("DELETE", `/v1/ingest/jobs/${jobId}`);
  }

  retryIngestionJob(jobId: number): Promise<IngestionJobResponse> {
    return this.request("POST", `/v1/ingest/jobs/${jobId}/retry`);
  }

  validate(): Promise<ValidationResponse> {
    return this.request("GET", "/v1/validate");
  }

  stats(): Promise<StatsResponse> {
    return this.request("GET", "/v1/stats");
  }

  private async request(method: string, path: string, body?: unknown): Promise<unknown> {
    const headers: Record<string, string> = {};
    if (this.token) headers.authorization = `Bearer ${this.token}`;
    const init: RequestInit = { method, headers };
    if (body !== undefined) {
      init.body = typeof body === "string" ? body : JSON.stringify(body);
      headers["content-type"] = "application/json";
    }
    const url = `${this.baseUrl}${this.scoped(path)}`;
    let attempt = 0;
    while (true) {
      try {
        const response = await fetch(url, init);
        if (!response.ok) {
          if (this.isRetryable(response.status) && attempt < this.maxRetries) {
            attempt += 1;
            await this.sleep(this.retryDelayMs * attempt);
            continue;
          }
          throw await CortexDBError.fromResponse(response);
        }
        return response.json();
      } catch (error) {
        if (error instanceof CortexDBError) throw error;
        if (attempt < this.maxRetries) {
          attempt += 1;
          await this.sleep(this.retryDelayMs * attempt);
          continue;
        }
        throw error;
      }
    }
  }

  private isRetryable(status: number): boolean {
    return [500, 502, 503, 504].includes(status);
  }

  private sleep(ms: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }

  private path(path: string, query: Record<string, string | number>): string {
    const params = new URLSearchParams();
    for (const [key, value] of Object.entries(query)) {
      params.set(key, String(value));
    }
    return `${path}?${params.toString()}`;
  }

  private scoped(path: string): string {
    if (!this.tenant || this.tenant === "default") return path;
    const params = new URLSearchParams({ tenant: this.tenant });
    return `${path}${path.includes("?") ? "&" : "?"}${params.toString()}`;
  }
}
