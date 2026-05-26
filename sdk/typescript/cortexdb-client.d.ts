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

export class CortexDBClient {
  constructor(baseUrl?: string, token?: string, tenant?: string);
  withTenant(tenant: string): CortexDBClient;
  health(): Promise<JsonObject>;
  putCell(cellId: number, payload: string): Promise<JsonObject>;
  getCell(cellId: number): Promise<JsonObject>;
  tombstoneCell(cellId: number): Promise<JsonObject>;
  flush(): Promise<JsonObject>;
  compact(): Promise<JsonObject>;
  search(scope: string, query: string, limit?: number): Promise<SearchResponse>;
  searchVector(
    scope: string,
    vector: number[],
    limit?: number,
    algorithm?: VectorAlgorithm,
  ): Promise<SearchResponse>;
  evaluateAnn(scope: string, vector: number[], limit?: number): Promise<AnnEvaluationResponse>;
  aql(scope: string, statement: string): Promise<JsonObject>;
  retrieveContext(scope: string, statement: string): Promise<JsonObject>;
  verifyFact(scope: string, statement: string): Promise<JsonObject>;
  remember(scope: string, statement: string): Promise<JsonObject>;
  ingestText(scope: string, text: string, source?: string): Promise<JsonObject>;
  ingestJson(scope: string, document: string, source?: string): Promise<JsonObject>;
  ingestCsv(scope: string, document: string, source?: string): Promise<JsonObject>;
  ingestionJob(jobId: number): Promise<JsonObject>;
  validate(): Promise<JsonObject>;
  stats(): Promise<JsonObject>;
}
