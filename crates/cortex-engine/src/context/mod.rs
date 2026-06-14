use cortex_core::CellId;

use crate::query::metadata::SourceRef;
use crate::source_trust::SourceTrustCategory;
use crate::tool_registry::ToolRecommendation;

mod answerability;
mod conflicts;
pub mod dedup;
pub mod explain;
pub mod export;
mod freshness;
mod grounding;
mod large_cell;
mod pack;
mod scoring;
mod span;
mod token_estimator;
mod trace;

pub use answerability::DEFAULT_ANSWERABILITY_THRESHOLD_Q16;
pub use explain::{ContextCellExplain, ContextCellExplainOutcome};
pub use export::ContextPackExportFormat;
pub use freshness::SourceFreshnessCategory;
pub use grounding::{
    AnswerGroundingOptions, AnswerGroundingReport, AnswerGroundingSpan,
    DEFAULT_GROUNDING_THRESHOLD_Q16,
};
pub use large_cell::ContextLargeCellPolicy;
pub(crate) use pack::ContextPackBuilder;
pub use token_estimator::{estimate_tokens, estimate_tokens_for_profile, ContextTokenProfile};
pub use trace::{
    ContextPipelineCellTrace, ContextPipelineStageTrace, ContextPipelineTrace,
    ContextPipelineVerificationTrace, ContextScoreComponentTrace,
};

pub const DEFAULT_REDUNDANCY_THRESHOLD_Q16: u16 = 32_768;
pub const DEFAULT_CITATION_OVERHEAD_TOKENS: u32 = 8;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextPackOptions {
    pub token_budget_tokens: u32,
    pub require_citations: bool,
    pub reduce_redundancy: bool,
    pub redundancy_threshold_q16: u16,
    pub citation_overhead_tokens: u32,
    pub token_profile: ContextTokenProfile,
    pub large_cell_policy: ContextLargeCellPolicy,
    pub span_level_packing: bool,
    pub span_context_lines: usize,
}

impl Default for ContextPackOptions {
    fn default() -> Self {
        Self {
            token_budget_tokens: 0,
            require_citations: false,
            reduce_redundancy: false,
            redundancy_threshold_q16: DEFAULT_REDUNDANCY_THRESHOLD_Q16,
            citation_overhead_tokens: DEFAULT_CITATION_OVERHEAD_TOKENS,
            token_profile: ContextTokenProfile::default(),
            large_cell_policy: ContextLargeCellPolicy::default(),
            span_level_packing: false,
            span_context_lines: 2,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextScoreComponent {
    pub name: String,
    pub value: u32,
    pub contribution: i32,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextExplain {
    pub score: u32,
    pub matched_terms: Vec<String>,
    pub why_selected: String,
    pub score_components: Vec<ContextScoreComponent>,
    pub base_bm25: u32,
    pub source_trust_q16: u16,
    pub source_trust_category: SourceTrustCategory,
    pub source_trust_bonus: u32,
    pub source_freshness_q16: u16,
    pub source_freshness_category: SourceFreshnessCategory,
    pub source_freshness_bonus: u32,
    pub redundancy_penalty: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextPackCell {
    pub cell_id: CellId,
    pub payload: Vec<u8>,
    pub metadata: crate::query::CellMetadata,
    pub estimated_tokens: u32,
    pub citation: Option<String>,
    pub provenance: Option<ContextSpanProvenance>,
    pub explain: Option<ContextExplain>,
    pub access_decision: Option<ContextAccessDecision>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextAccessDecision {
    pub cell_id: CellId,
    pub decision: ContextAccessDecisionOutcome,
    pub policy: String,
    pub reason: String,
    pub scope: String,
    pub scope_id: u64,
    pub agent_id: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextAccessDecisionOutcome {
    Allowed,
    NotRecorded,
}

impl ContextAccessDecisionOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Allowed => "allowed",
            Self::NotRecorded => "not_recorded",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextSpanProvenance {
    pub source_cell_id: CellId,
    pub source_byte_start: usize,
    pub source_byte_end: usize,
    pub source_line_start: u32,
    pub source_line_end: u32,
    pub source_ref: Option<SourceRef>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextPack {
    pub cells: Vec<ContextPackCell>,
    pub token_budget_tokens: u32,
    pub estimated_tokens: u32,
    pub truncated: bool,
    pub citations_required: bool,
    pub answerability_q16: u16,
    pub conflict_visibility_q16: u16,
    pub visible_conflict_count: u32,
    pub anomalies: Vec<ContextPackAnomaly>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextPackWithTools {
    pub pack: ContextPack,
    pub tool_recommendations: Vec<ToolRecommendation>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextPackAnomalyCode {
    RedundantCell,
    MissingCitation,
    TokenOverload,
    ScopeMismatch,
    InsufficientContext,
    VisibleConflict,
}

impl ContextPackAnomalyCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RedundantCell => "redundant_cell",
            Self::MissingCitation => "missing_citation",
            Self::TokenOverload => "token_overload",
            Self::ScopeMismatch => "scope_mismatch",
            Self::InsufficientContext => "insufficient_context",
            Self::VisibleConflict => "visible_conflict",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextPackAnomaly {
    pub cell_id: Option<CellId>,
    pub code: ContextPackAnomalyCode,
    pub message: String,
    pub why_excluded: Option<String>,
}
