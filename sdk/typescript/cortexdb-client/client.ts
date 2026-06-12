import { buildGroundedAnswerResponse } from "./grounding";
import { buildRememberAql, buildRetrieveContextAql, buildVerifyFactAql } from "./aql";
import { CortexDBError, type JsonObject } from "./errors";
import type { AnnEvaluationResponse, AqlResponse, CellLookupResponse, ContextPackResponse, DeleteJobResponse, GroundedAnswerOptions, GroundedAnswerResponse, HealthResponse, IngestResponse, IngestionJobResponse, PutCellResponse, RememberResponse, SearchResponse, StatsResponse, ValidationResponse, VerificationReportResponse, VectorAlgorithm } from "./types";

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
