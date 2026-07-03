import { answerWithGroundedContext } from "./answering";
import { buildRememberAql, buildRetrieveContextAql, buildVerifyFactAql } from "./aql";
import type { JsonObject } from "./errors";
import { requestJson, scopedPath, type ClientOptions, type FetchLike } from "./transport";
import type { AnnEvaluationResponse, AqlResponse, CellLookupResponse, ContextPackResponse, DeleteJobResponse, GroundedAnswerOptions, GroundedAnswerResponse, HealthResponse, IngestResponse, IngestionJobResponse, PutCellResponse, RememberResponse, SearchResponse, StatsResponse, ValidationResponse, VerificationReportResponse, VectorAlgorithm } from "./types";

export class CortexDBClient {
  constructor(
    private readonly baseUrl = "http://127.0.0.1:8181",
    private readonly token?: string,
    private readonly tenant?: string,
    private readonly maxRetries = 0,
    private readonly retryDelayMs = 500,
    private readonly timeoutMs = 10_000,
    private readonly fetchImpl: FetchLike = globalThis.fetch.bind(globalThis),
  ) {}

  withTenant(tenant: string): CortexDBClient {
    return this.copy({ tenant });
  }

  withRetries(maxRetries: number, retryDelayMs = 500): CortexDBClient {
    return this.copy({ maxRetries, retryDelayMs });
  }

  withTimeout(timeoutMs: number): CortexDBClient {
    return this.copy({ timeoutMs });
  }

  withOptions(options: ClientOptions): CortexDBClient {
    return this.copy({ timeoutMs: options.timeoutMs, fetchImpl: options.fetch });
  }

  withSession(): CortexDBClient {
    return this;
  }

  close(): void {
    // Fetch keeps connection pooling inside the runtime; no explicit close hook exists.
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

  /**
   * Commit an optimistic-concurrency agent transaction (F04-B6.3). A conflict is
   * a normal response with `outcome === "conflict"`, not an HTTP error — read
   * `outcome` rather than relying on the status code.
   */
  agentTransaction(request: JsonObject): Promise<JsonObject> {
    return this.request("POST", "/v1/transactions", JSON.stringify(request));
  }

  /** Commit a durable SharedSequenced agent handoff (F04-B6.3 / F08-B6.1). */
  agentHandoff(request: JsonObject): Promise<JsonObject> {
    return this.request("POST", "/v1/handoff", JSON.stringify(request));
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
    return answerWithGroundedContext(this, scope, brain, question, answerer, options);
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
    return requestJson({
      baseUrl: this.baseUrl,
      path: this.scoped(path),
      method,
      token: this.token,
      body,
      maxRetries: this.maxRetries,
      retryDelayMs: this.retryDelayMs,
      timeoutMs: this.timeoutMs,
      fetch: this.fetchImpl,
    });
  }

  private path(path: string, query: Record<string, string | number>): string {
    const params = new URLSearchParams();
    for (const [key, value] of Object.entries(query)) {
      params.set(key, String(value));
    }
    return `${path}?${params.toString()}`;
  }

  private scoped(path: string): string {
    return scopedPath(path, this.tenant);
  }

  private copy(overrides: {
    tenant?: string;
    maxRetries?: number;
    retryDelayMs?: number;
    timeoutMs?: number;
    fetchImpl?: FetchLike;
  }): CortexDBClient {
    return new CortexDBClient(
      this.baseUrl,
      this.token,
      overrides.tenant ?? this.tenant,
      overrides.maxRetries ?? this.maxRetries,
      overrides.retryDelayMs ?? this.retryDelayMs,
      overrides.timeoutMs ?? this.timeoutMs,
      overrides.fetchImpl ?? this.fetchImpl,
    );
  }
}
