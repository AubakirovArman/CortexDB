use crate::types::{BrainId, MemoryType, RetrievalMode, ScopeId, Q16};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct QualityThresholds {
    pub min_confidence_q16: Q16,
    pub min_source_trust_q16: Q16,
    pub max_freshness_seconds: Option<u64>,
    pub valid_at: Option<String>,
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
    pub bitmap_program: super::BitmapProgram,
    pub context_policy: ContextPolicy,
    pub quality_thresholds: QualityThresholds,
    pub weights: super::RetrievalWeights,
    /// A5/A7.3: per-query MMR diversification lambda (Q16) from `USING DIVERSITY`.
    /// `None` defers to the database default (off), so it does not appear in the
    /// canonical pack unless the query asks for it.
    pub diversity_lambda_q16: Option<u64>,
    /// A7.2: per-query two-stage dense rerank weight (Q16) from `USING RERANK`.
    /// `None` defers to the database default (off).
    pub rerank_weight_q16: Option<u64>,
    /// A4.2: per-query temporal supersession from `SUPPRESS SUPERSEDED`. `false`
    /// defers to the database default (off).
    pub suppress_superseded: bool,
    /// A4.1: per-query recency window in seconds from `RECENCY WINDOW`. `None`
    /// defers to the database default (off).
    pub recency_window_seconds: Option<u64>,
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
