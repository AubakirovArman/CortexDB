export type JsonObject = Record<string, unknown>;
export type VectorAlgorithm = "ann" | "exact";
export type SearchMode = "keyword" | "vector_exact" | "vector_ann";
export type AnnSearchPath = "hnsw_graph" | "exact_fallback";
export type AnnFallbackReason =
  | "empty_graph"
  | "invalid_graph"
  | "insufficient_results"
  | "low_recall"
  | "no_persisted_segments"
  | "uncheckpointed_changes";

export interface AnnSearchReport {
  path: AnnSearchPath;
  fallback_reason: AnnFallbackReason | null;
  requested_limit: number;
  allowed_candidates: number;
  graph_nodes: number;
  returned_candidates: number;
  recall_q16: number | null;
  min_recall_q16: number | null;
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
  token_budget_tokens: number;
  estimated_tokens: number;
  truncated: boolean;
  citations_required: boolean;
  cells: ContextPackCellResponse[];
  anomalies: ContextPackAnomalyResponse[];
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

export interface RememberResponse {
  seq: number;
  cell_id: number;
  ttl_seconds: number | null;
}

export class CortexDBClient {
  constructor(
    private readonly baseUrl = "http://127.0.0.1:8181",
    private readonly token?: string,
    private readonly tenant?: string,
  ) {}

  withTenant(tenant: string): CortexDBClient {
    return new CortexDBClient(this.baseUrl, this.token, tenant);
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
    const response = await fetch(`${this.baseUrl}${this.scoped(path)}`, init);
    if (!response.ok) throw new Error(await response.text());
    return response.json();
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
