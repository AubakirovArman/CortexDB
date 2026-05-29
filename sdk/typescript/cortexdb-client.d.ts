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

export class CortexDBClient {
  constructor(baseUrl?: string, token?: string, tenant?: string);
  withTenant(tenant: string): CortexDBClient;
  health(): Promise<HealthResponse>;
  putCell(cellId: number, payload: string): Promise<PutCellResponse>;
  getCell(cellId: number): Promise<CellLookupResponse>;
  tombstoneCell(cellId: number): Promise<JsonObject>;
  flush(): Promise<JsonObject>;
  compact(): Promise<JsonObject>;
  search(scope: string, query: string, limit?: number): Promise<SearchResponse>;
  searchVector(scope: string, vector: number[], limit?: number, algorithm?: VectorAlgorithm): Promise<SearchResponse>;
  evaluateAnn(scope: string, vector: number[], limit?: number): Promise<AnnEvaluationResponse>;
  aql(scope: string, statement: string): Promise<AqlResponse>;
  retrieveContext(scope: string, statement: string): Promise<ContextPackResponse>;
  verifyFact(scope: string, statement: string): Promise<VerificationReportResponse>;
  remember(scope: string, statement: string): Promise<RememberResponse>;
  ingestText(scope: string, text: string, source?: string): Promise<IngestResponse>;
  ingestJson(scope: string, document: string, source?: string): Promise<IngestResponse>;
  ingestCsv(scope: string, document: string, source?: string): Promise<IngestResponse>;
  ingestionJob(jobId: number): Promise<JsonObject>;
  ingestionJobResponse(jobId: number): Promise<IngestionJobResponse>;
  deleteIngestionJob(jobId: number): Promise<DeleteJobResponse>;
  retryIngestionJob(jobId: number): Promise<IngestionJobResponse>;
  validate(): Promise<ValidationResponse>;
  stats(): Promise<StatsResponse>;
}
