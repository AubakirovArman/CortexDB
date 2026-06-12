use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub struct EvidenceResponse {
    pub cell_id: u64,
    pub matched_terms: u32,
    pub match_score_q16: u16,
    pub match_kind: String,
    pub source_trust_q16: u16,
    pub source_trust_category: String,
    pub citation: Option<String>,
    pub payload_text: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct GuardResponse {
    pub cell_id: Option<u64>,
    pub code: String,
    pub message: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct NumericConflictResponse {
    pub metric: String,
    pub left: String,
    pub right: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct VerificationReportResponse {
    pub fact: String,
    pub status: String,
    pub verdict: String,
    pub confidence_q16: u16,
    pub evidence: Vec<EvidenceResponse>,
    pub contradicting_evidence: Vec<EvidenceResponse>,
    pub guards: Vec<GuardResponse>,
    pub supporting: Vec<EvidenceResponse>,
    pub contradicting: Vec<EvidenceResponse>,
    pub numeric_conflicts: Vec<NumericConflictResponse>,
}
