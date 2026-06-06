use cortex_core::CellId;

use crate::source_trust::SourceTrustCategory;

pub mod dedup;
pub mod explain;
pub mod export;
mod pack;

pub use export::ContextPackExportFormat;
pub use pack::estimate_tokens;

pub const DEFAULT_REDUNDANCY_THRESHOLD_Q16: u16 = 32_768;
pub const DEFAULT_CITATION_OVERHEAD_TOKENS: u32 = 8;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextPackOptions {
    pub token_budget_tokens: u32,
    pub require_citations: bool,
    pub reduce_redundancy: bool,
    pub redundancy_threshold_q16: u16,
    pub citation_overhead_tokens: u32,
}

impl Default for ContextPackOptions {
    fn default() -> Self {
        Self {
            token_budget_tokens: 0,
            require_citations: false,
            reduce_redundancy: false,
            redundancy_threshold_q16: DEFAULT_REDUNDANCY_THRESHOLD_Q16,
            citation_overhead_tokens: DEFAULT_CITATION_OVERHEAD_TOKENS,
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
    pub redundancy_penalty: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextPackCell {
    pub cell_id: CellId,
    pub payload: Vec<u8>,
    pub estimated_tokens: u32,
    pub citation: Option<String>,
    pub explain: Option<ContextExplain>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextPack {
    pub cells: Vec<ContextPackCell>,
    pub token_budget_tokens: u32,
    pub estimated_tokens: u32,
    pub truncated: bool,
    pub citations_required: bool,
    pub anomalies: Vec<ContextPackAnomaly>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextPackAnomalyCode {
    RedundantCell,
    MissingCitation,
    TokenOverload,
    ScopeMismatch,
}

impl ContextPackAnomalyCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RedundantCell => "redundant_cell",
            Self::MissingCitation => "missing_citation",
            Self::TokenOverload => "token_overload",
            Self::ScopeMismatch => "scope_mismatch",
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
