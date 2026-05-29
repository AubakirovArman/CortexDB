use std::collections::{BTreeMap, BTreeSet};

use cortex_storage::hnsw::HnswGraphIndex;

use super::hnsw::{DistanceMetric, HnswIndex};
use super::persisted::search_persisted_vectors;
use super::ScoredCandidate;

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
    InsufficientResults,
    LowRecall,
    NoPersistedSegments,
    UncheckpointedChanges,
}

impl AnnFallbackReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EmptyGraph => "empty_graph",
            Self::InvalidGraph => "invalid_graph",
            Self::InsufficientResults => "insufficient_results",
            Self::LowRecall => "low_recall",
            Self::NoPersistedSegments => "no_persisted_segments",
            Self::UncheckpointedChanges => "uncheckpointed_changes",
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
}

impl Default for AnnSearchPolicy {
    fn default() -> Self {
        Self {
            min_recall_q16: Some(MIN_ANN_RECALL_Q16),
            fallback: true,
            fallback_scan_cap: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnnSearchReport {
    pub path: AnnSearchPath,
    pub fallback_reason: Option<AnnFallbackReason>,
    pub requested_limit: usize,
    pub allowed_candidates: usize,
    pub graph_nodes: usize,
    pub returned_candidates: usize,
    pub recall_q16: Option<u16>,
    pub min_recall_q16: Option<u16>,
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

pub fn search_persisted_ann(
    vectors: &BTreeMap<u32, Vec<i16>>,
    graph: &HnswGraphIndex,
    query: &[i16],
    allowed: &BTreeSet<u32>,
    limit: usize,
) -> AnnSearchOutcome {
    search_persisted_ann_with_policy(
        vectors,
        graph,
        query,
        allowed,
        limit,
        AnnSearchPolicy::default(),
    )
}

pub fn search_persisted_ann_with_policy(
    vectors: &BTreeMap<u32, Vec<i16>>,
    graph: &HnswGraphIndex,
    query: &[i16],
    allowed: &BTreeSet<u32>,
    limit: usize,
    policy: AnnSearchPolicy,
) -> AnnSearchOutcome {
    let available = vectors
        .iter()
        .filter(|(id, vector)| allowed.contains(id) && vector.len() == query.len())
        .count();
    let expected = limit.min(available);
    let graph_nodes = graph.links.len();
    if graph.links.is_empty() {
        if !policy.fallback {
            return fallback_disabled_outcome(
                AnnFallbackReason::EmptyGraph,
                vectors,
                query,
                allowed,
                limit,
                available,
                graph_nodes,
                policy,
            );
        }
        return exact(
            vectors,
            query,
            allowed,
            limit,
            available,
            graph_nodes,
            AnnFallbackReason::EmptyGraph,
        );
    }
    let ann = match search_hnsw(vectors, graph, query, allowed, limit, expected) {
        Ok(ann) => ann,
        Err(reason) => {
            if !policy.fallback {
                return fallback_disabled_outcome(
                    reason,
                    vectors,
                    query,
                    allowed,
                    limit,
                    available,
                    graph_nodes,
                    policy,
                );
            }
            return exact(
                vectors,
                query,
                allowed,
                limit,
                available,
                graph_nodes,
                reason,
            );
        }
    };
    let exact_results =
        search_persisted_vectors(vectors, query, allowed, limit, &DistanceMetric::default());
    let exact_set = exact_results
        .iter()
        .map(|candidate| candidate.cell_id)
        .collect::<BTreeSet<_>>();
    let overlap = ann
        .iter()
        .filter(|candidate| exact_set.contains(&candidate.cell_id))
        .count();
    let recall = recall_q16(overlap, exact_results.len());
    let effective_min_recall = policy.min_recall_q16.unwrap_or(MIN_ANN_RECALL_Q16);
    if recall < effective_min_recall {
        if !policy.fallback {
            return fallback_disabled_outcome(
                AnnFallbackReason::LowRecall,
                vectors,
                query,
                allowed,
                limit,
                available,
                graph_nodes,
                policy,
            );
        }
        return exact_from_results(
            exact_results,
            limit,
            available,
            graph_nodes,
            AnnFallbackReason::LowRecall,
            Some(recall),
            policy.min_recall_q16,
        );
    }
    let returned = ann.len();
    AnnSearchOutcome {
        results: ann,
        report: AnnSearchReport {
            path: AnnSearchPath::HnswGraph,
            fallback_reason: None,
            requested_limit: limit,
            allowed_candidates: available,
            graph_nodes,
            returned_candidates: returned,
            recall_q16: Some(recall),
            min_recall_q16: policy.min_recall_q16,
        },
    }
}

pub fn evaluate_persisted_ann(
    vectors: &BTreeMap<u32, Vec<i16>>,
    graph: &HnswGraphIndex,
    query: &[i16],
    allowed: &BTreeSet<u32>,
    limit: usize,
) -> AnnEvaluationReport {
    evaluate_persisted_ann_with_policy(
        vectors,
        graph,
        query,
        allowed,
        limit,
        AnnSearchPolicy::default(),
    )
}

pub fn evaluate_persisted_ann_with_policy(
    vectors: &BTreeMap<u32, Vec<i16>>,
    graph: &HnswGraphIndex,
    query: &[i16],
    allowed: &BTreeSet<u32>,
    limit: usize,
    policy: AnnSearchPolicy,
) -> AnnEvaluationReport {
    let exact_results =
        search_persisted_vectors(vectors, query, allowed, limit, &DistanceMetric::default());
    let available = vectors
        .iter()
        .filter(|(id, vector)| allowed.contains(id) && vector.len() == query.len())
        .count();
    let expected = limit.min(available);
    let graph_nodes = graph.links.len();
    let ann_outcome = match search_hnsw(vectors, graph, query, allowed, limit, expected) {
        Ok(results) => AnnSearchOutcome {
            report: AnnSearchReport {
                path: AnnSearchPath::HnswGraph,
                fallback_reason: None,
                requested_limit: limit,
                allowed_candidates: available,
                graph_nodes,
                returned_candidates: results.len(),
                recall_q16: None,
                min_recall_q16: policy.min_recall_q16,
            },
            results,
        },
        Err(reason) => exact_from_results(
            exact_results.clone(),
            limit,
            available,
            graph_nodes,
            reason,
            None,
            policy.min_recall_q16,
        ),
    };
    let exact_top_k = exact_results
        .iter()
        .map(|candidate| candidate.cell_id)
        .collect::<Vec<_>>();
    let ann_top_k = ann_outcome
        .results
        .iter()
        .map(|candidate| candidate.cell_id)
        .collect::<Vec<_>>();
    let exact_set = exact_top_k.iter().copied().collect::<BTreeSet<_>>();
    let overlap_count = ann_top_k
        .iter()
        .filter(|candidate| exact_set.contains(candidate))
        .count();
    let recall = recall_q16(overlap_count, exact_results.len());
    let mut search = ann_outcome.report;
    if search.path == AnnSearchPath::HnswGraph {
        search.recall_q16 = Some(recall);
        search.min_recall_q16 = policy.min_recall_q16;
    }
    AnnEvaluationReport {
        search,
        exact_top_k,
        ann_top_k,
        overlap_count,
        recall_q16: recall,
    }
}

fn search_hnsw(
    vectors: &BTreeMap<u32, Vec<i16>>,
    graph: &HnswGraphIndex,
    query: &[i16],
    allowed: &BTreeSet<u32>,
    limit: usize,
    expected: usize,
) -> Result<Vec<ScoredCandidate>, AnnFallbackReason> {
    if graph.links.is_empty() {
        return Err(AnnFallbackReason::EmptyGraph);
    }
    let index = HnswIndex::from_graph(vectors.clone(), graph.clone(), 8, 64);
    if !index.verify_hnsw_integrity() {
        return Err(AnnFallbackReason::InvalidGraph);
    }
    let ann = index.search_allowed(query, allowed, limit);
    if ann.len() < expected {
        Err(AnnFallbackReason::InsufficientResults)
    } else {
        Ok(ann)
    }
}

#[allow(clippy::too_many_arguments)]
fn fallback_disabled_outcome(
    reason: AnnFallbackReason,
    vectors: &BTreeMap<u32, Vec<i16>>,
    query: &[i16],
    allowed: &BTreeSet<u32>,
    limit: usize,
    available: usize,
    graph_nodes: usize,
    policy: AnnSearchPolicy,
) -> AnnSearchOutcome {
    let cap = if policy.fallback {
        policy.fallback_scan_cap.unwrap_or(limit).min(limit)
    } else {
        policy.fallback_scan_cap.unwrap_or(0).min(limit)
    };
    let ann = if cap == 0 {
        Vec::new()
    } else {
        search_persisted_vectors(vectors, query, allowed, cap, &DistanceMetric::default())
    };
    let returned = ann.len();
    AnnSearchOutcome {
        results: ann,
        report: AnnSearchReport {
            path: AnnSearchPath::HnswGraph,
            fallback_reason: Some(reason),
            requested_limit: limit,
            allowed_candidates: available,
            graph_nodes,
            returned_candidates: returned,
            recall_q16: None,
            min_recall_q16: policy.min_recall_q16,
        },
    }
}

fn exact(
    vectors: &BTreeMap<u32, Vec<i16>>,
    query: &[i16],
    allowed: &BTreeSet<u32>,
    limit: usize,
    available: usize,
    graph_nodes: usize,
    reason: AnnFallbackReason,
) -> AnnSearchOutcome {
    let results =
        search_persisted_vectors(vectors, query, allowed, limit, &DistanceMetric::default());
    exact_from_results(results, limit, available, graph_nodes, reason, None, None)
}

fn exact_from_results(
    results: Vec<ScoredCandidate>,
    limit: usize,
    available: usize,
    graph_nodes: usize,
    reason: AnnFallbackReason,
    recall_q16: Option<u16>,
    min_recall_q16: Option<u16>,
) -> AnnSearchOutcome {
    let returned = results.len();
    AnnSearchOutcome {
        results,
        report: AnnSearchReport {
            path: AnnSearchPath::ExactFallback,
            fallback_reason: Some(reason),
            requested_limit: limit,
            allowed_candidates: available,
            graph_nodes,
            returned_candidates: returned,
            recall_q16,
            min_recall_q16,
        },
    }
}

fn recall_q16(overlap_count: usize, expected_count: usize) -> u16 {
    if expected_count == 0 {
        return 65_535;
    }
    ((overlap_count as u64 * 65_535) / expected_count as u64) as u16
}

#[cfg(test)]
mod tests;
