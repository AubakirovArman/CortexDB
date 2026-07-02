export interface EvidenceResponse {
  cell_id: number;
  matched_terms: number;
  match_score_q16: number;
  match_kind: string;
  source_trust_q16: number;
  source_trust_category: string;
  citation: string | null;
  payload_text: string;
}

export interface GuardResponse {
  cell_id: number | null;
  code: string;
  message: string;
}

export interface NumericConflictResponse {
  kind: string;
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
  accountability_receipt?: Record<string, unknown> | null;
}
