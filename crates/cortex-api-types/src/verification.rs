use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceResponse {
    pub cell_id: u64,
    pub matched_terms: u32,
    #[serde(default)]
    pub match_score_q16: u16,
    #[serde(default)]
    pub match_kind: String,
    pub source_trust_q16: u16,
    #[serde(default)]
    pub source_trust_category: String,
    pub citation: Option<String>,
    pub payload_text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GuardResponse {
    pub cell_id: Option<u64>,
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct NumericConflictResponse {
    pub metric: String,
    pub left: String,
    pub right: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerificationReportResponse {
    pub fact: String,
    pub status: String,
    pub verdict: String,
    #[serde(default)]
    pub confidence_q16: u16,
    pub evidence: Vec<EvidenceResponse>,
    pub contradicting_evidence: Vec<EvidenceResponse>,
    pub guards: Vec<GuardResponse>,
    pub supporting: Vec<EvidenceResponse>,
    pub contradicting: Vec<EvidenceResponse>,
    pub numeric_conflicts: Vec<NumericConflictResponse>,
}
