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
  visited_candidates: number;
  max_visited_candidates?: number | null;
  recall_q16: number | null;
  min_recall_q16: number | null;
  hnsw_max_neighbors: number;
  hnsw_ef_search: number;
  hnsw_ef_construction: number;
  hnsw_layer_count: number;
  upper_graph_edges: number;
  require_slo: boolean;
  production_safe: boolean;
  slo_violations: string[];
}

export interface AnnNoFallbackDecision {
  allowed: boolean;
  reasons: string[];
}

export interface SearchRoutingDecision {
  requested_mode: string;
  selected_strategy: string;
  reason: string;
  text_available: boolean;
  vector_available: boolean;
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
  routing?: SearchRoutingDecision | null;
  rerank?: string | null;
  ann_report: AnnSearchReport | null;
  no_fallback_decision?: AnnNoFallbackDecision | null;
  results: SearchResult[];
}

export interface AnnEvaluationResponse {
  available: boolean;
  reason: "requires_persisted_checkpoint_without_wal_tail" | null;
  ann_report: AnnSearchReport | null;
  no_fallback_decision?: AnnNoFallbackDecision | null;
  exact_top_k: number[];
  ann_top_k: number[];
  overlap_count: number;
  recall_q16: number;
}
