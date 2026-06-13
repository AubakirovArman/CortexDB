use super::{BitmapProgram, RetrievalWeights};
use crate::types::{BrainId, MemoryType, RetrievalMode, ScopeId, Q16};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QualityThresholds {
    pub min_confidence_q16: Q16,
    pub min_source_trust_q16: Q16,
    pub max_freshness_seconds: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextPolicy {
    pub budget_tokens: u32,
    pub candidate_limit: u32,
    pub require_citations: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundRetrievePlan {
    pub brain_id: BrainId,
    pub task: String,
    pub mode: RetrievalMode,
    pub bitmap_program: BitmapProgram,
    pub context_policy: ContextPolicy,
    pub quality_thresholds: QualityThresholds,
    pub weights: RetrievalWeights,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundVerifyFactPlan {
    pub brain_id: BrainId,
    pub fact: String,
    pub max_candidates: u32,
    pub max_evidence: u32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundRememberPlan {
    pub scope_id: ScopeId,
    pub scope_name: String,
    pub memory_type: MemoryType,
    pub content: String,
    pub ttl_seconds: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BoundPlan {
    Retrieve(Box<BoundRetrievePlan>),
    VerifyFact(Box<BoundVerifyFactPlan>),
    Remember(Box<BoundRememberPlan>),
}
