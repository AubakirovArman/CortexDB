mod evaluation;
mod report;
mod runtime;
mod search;
mod types;

#[cfg(test)]
use super::hnsw::{DistanceMetric, HnswIndex};
#[cfg(test)]
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
mod tests;

pub use evaluation::{evaluate_persisted_ann, evaluate_persisted_ann_with_policy};
pub(crate) use report::finalize_report;
pub use search::{search_persisted_ann, search_persisted_ann_with_policy};
pub use types::{
    AnnEvaluationReport, AnnFallbackReason, AnnMetrics, AnnSearchOutcome, AnnSearchPath,
    AnnSearchPolicy, AnnSearchReport, AnnSloViolation, MIN_ANN_RECALL_Q16,
};
