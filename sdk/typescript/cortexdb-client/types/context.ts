import type { RetrieveContextAqlOptions } from "./aql";
import type { VerificationReportResponse } from "./verification";

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
  grounding_report?: AnswerGroundingReportResponse | null;
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
