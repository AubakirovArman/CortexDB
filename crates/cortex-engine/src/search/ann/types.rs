#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnnSearchPath {
    HnswGraph,
    ExactFallback,
}

impl AnnSearchPath {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::HnswGraph => "hnsw_graph",
            Self::ExactFallback => "exact_fallback",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnnFallbackReason {
    EmptyGraph,
    InvalidGraph,
    StaleGraph,
    InsufficientResults,
    LowRecall,
    VisitBudgetExceeded,
    NoPersistedSegments,
    UncheckpointedChanges,
    HnswDisabled,
}

impl AnnFallbackReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EmptyGraph => "empty_graph",
            Self::InvalidGraph => "invalid_graph",
            Self::StaleGraph => "stale_graph",
            Self::InsufficientResults => "insufficient_results",
            Self::LowRecall => "low_recall",
            Self::VisitBudgetExceeded => "visit_budget_exceeded",
            Self::NoPersistedSegments => "no_persisted_segments",
            Self::UncheckpointedChanges => "uncheckpointed_changes",
            Self::HnswDisabled => "hnsw_disabled",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnnSloViolation {
    EmptyGraph,
    InvalidGraph,
    StaleGraph,
    InsufficientResults,
    LowRecall,
    VisitBudgetExceeded,
    NoPersistedSegments,
    UncheckpointedChanges,
    HnswDisabled,
    RecallBelowMinimum,
    WeakMultiLayerTopology,
}

impl AnnSloViolation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EmptyGraph => "empty_graph",
            Self::InvalidGraph => "invalid_graph",
            Self::StaleGraph => "stale_graph",
            Self::InsufficientResults => "insufficient_results",
            Self::LowRecall => "low_recall",
            Self::VisitBudgetExceeded => "visit_budget_exceeded",
            Self::NoPersistedSegments => "no_persisted_segments",
            Self::UncheckpointedChanges => "uncheckpointed_changes",
            Self::HnswDisabled => "hnsw_disabled",
            Self::RecallBelowMinimum => "recall_below_minimum",
            Self::WeakMultiLayerTopology => "weak_multi_layer_topology",
        }
    }
}

/// Minimum acceptable ANN recall threshold.
/// Q16_ONE = 65_535 represents 100% recall.
/// Production default is 75% (49_151) to allow approximate results
/// while preserving correctness through exact fallback when recall drops.
pub const MIN_ANN_RECALL_Q16: u16 = 49_151;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnnSearchPolicy {
    pub min_recall_q16: Option<u16>,
    pub fallback: bool,
    pub fallback_scan_cap: Option<usize>,
    pub max_visited_candidates: Option<usize>,
    pub require_slo: bool,
}

impl Default for AnnSearchPolicy {
    fn default() -> Self {
        Self {
            min_recall_q16: Some(MIN_ANN_RECALL_Q16),
            fallback: true,
            fallback_scan_cap: None,
            max_visited_candidates: None,
            require_slo: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnnSearchReport {
    pub path: AnnSearchPath,
    pub fallback_reason: Option<AnnFallbackReason>,
    pub fallback_performed: bool,
    pub requested_limit: usize,
    pub allowed_candidates: usize,
    pub graph_nodes: usize,
    pub returned_candidates: usize,
    pub visited_candidates: usize,
    pub max_visited_candidates: Option<usize>,
    pub recall_q16: Option<u16>,
    pub min_recall_q16: Option<u16>,
    pub hnsw_max_neighbors: usize,
    pub hnsw_ef_search: usize,
    pub hnsw_layer_count: usize,
    pub hnsw_ef_construction: usize,
    pub upper_graph_edges: usize,
    pub require_slo: bool,
    pub production_safe: bool,
    pub slo_violations: Vec<AnnSloViolation>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnnSearchOutcome {
    pub results: Vec<ScoredCandidate>,
    pub report: AnnSearchReport,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnnEvaluationReport {
    pub search: AnnSearchReport,
    pub exact_top_k: Vec<u32>,
    pub ann_top_k: Vec<u32>,
    pub overlap_count: usize,
    pub recall_q16: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct AnnMetrics {
    pub graph_nodes: usize,
    pub total_edges: usize,
    pub persisted_segments: usize,
    pub has_checkpoint: bool,
    pub has_uncheckpointed_changes: bool,
    pub deleted_vectors: usize,
    pub rebuild_count: u64,
}
use super::super::ScoredCandidate;
