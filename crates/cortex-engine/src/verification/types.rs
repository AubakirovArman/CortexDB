use cortex_aql::Q16;
use cortex_core::CellId;

use super::numeric;
use crate::source_trust::SourceTrustCategory;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerificationStatus {
    Supported,
    Insufficient,
    Contradicted,
    Mixed,
}

impl VerificationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Supported => "supported",
            Self::Insufficient => "insufficient",
            Self::Contradicted => "contradicted",
            Self::Mixed => "mixed_evidence",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerificationMatchKind {
    ExactText,
    SemanticEntailment,
    NumericEntailment,
    SemanticContradiction,
    NumericContradiction,
    GraphContradiction,
}

impl VerificationMatchKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ExactText => "exact_text",
            Self::SemanticEntailment => "semantic_entailment",
            Self::NumericEntailment => "numeric_entailment",
            Self::SemanticContradiction => "semantic_contradiction",
            Self::NumericContradiction => "numeric_contradiction",
            Self::GraphContradiction => "graph_contradiction",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationEvidence {
    pub cell_id: CellId,
    pub matched_terms: u32,
    pub match_score_q16: Q16,
    pub match_kind: VerificationMatchKind,
    pub source_trust_q16: Q16,
    pub source_trust_category: SourceTrustCategory,
    pub citation: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationReport {
    pub fact: String,
    pub status: VerificationStatus,
    pub confidence_q16: Q16,
    pub evidence: Vec<VerificationEvidence>,
    pub contradicting_evidence: Vec<VerificationEvidence>,
    pub guards: Vec<VerificationGuard>,
    pub numeric_conflicts: Vec<VerificationNumericConflict>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerificationGuardCode {
    MissingCitation,
    NumericMismatch,
    StaleFact,
}

impl VerificationGuardCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MissingCitation => "missing_citation",
            Self::NumericMismatch => "numeric_mismatch",
            Self::StaleFact => "stale_fact",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationGuard {
    pub cell_id: Option<CellId>,
    pub code: VerificationGuardCode,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerificationNumericConflict {
    pub cell_id: CellId,
    pub metric: String,
    pub left: String,
    pub right: String,
    pub fact_value: numeric::NumericValue,
    pub evidence_value: numeric::NumericValue,
}
