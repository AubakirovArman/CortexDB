mod evaluation;
mod guarded_recall;
mod outcomes;
mod report;
mod runtime;
mod search;
mod types;

#[cfg(test)]
use super::hnsw::{DistanceMetric, HnswIndex};
#[cfg(test)]
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
mod guarded_latency_tests;
#[cfg(test)]
mod sparse_scope_tests;
#[cfg(test)]
mod tests;

pub use evaluation::{evaluate_persisted_ann, evaluate_persisted_ann_with_policy};
pub use guarded_recall::{
    should_sample_recall, GuardedRecallState, GuardedServingMode, RecallWindow,
    GUARDED_RECALL_SAMPLE_RATE, GUARDED_RECALL_WARMUP_QUERIES, GUARDED_RECALL_WINDOW,
};
pub(crate) use report::finalize_report;
pub use search::{
    build_verified_hnsw_index, search_persisted_ann_cached, search_persisted_ann_sampled,
    search_persisted_ann_with_policy, RecallMode,
};
pub use types::{
    AnnEvaluationReport, AnnFallbackReason, AnnMetrics, AnnSearchOutcome, AnnSearchPath,
    AnnSearchPolicy, AnnSearchReport, AnnSloViolation, MIN_ANN_RECALL_Q16,
};
