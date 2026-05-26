use std::collections::{BTreeMap, BTreeSet};

use cortex_storage::hnsw::HnswGraphIndex;

use super::hnsw::HnswIndex;
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

pub const MIN_ANN_RECALL_Q16: u16 = 65_535;

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

pub fn search_persisted_ann(
    vectors: &BTreeMap<u32, Vec<i16>>,
    graph: &HnswGraphIndex,
    query: &[i16],
    allowed: &BTreeSet<u32>,
    limit: usize,
) -> AnnSearchOutcome {
    let available = vectors.keys().filter(|id| allowed.contains(id)).count();
    let expected = limit.min(available);
    let graph_nodes = graph.links.len();
    if graph.links.is_empty() {
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
            return exact(
                vectors,
                query,
                allowed,
                limit,
                available,
                graph_nodes,
                reason,
            )
        }
    };
    let exact_results = search_persisted_vectors(vectors, query, allowed, limit);
    let exact_set = exact_results
        .iter()
        .map(|candidate| candidate.cell_id)
        .collect::<BTreeSet<_>>();
    let overlap = ann
        .iter()
        .filter(|candidate| exact_set.contains(&candidate.cell_id))
        .count();
    let recall = recall_q16(overlap, exact_results.len());
    if recall < MIN_ANN_RECALL_Q16 {
        return exact_from_results(
            exact_results,
            limit,
            available,
            graph_nodes,
            AnnFallbackReason::LowRecall,
            Some(recall),
            Some(MIN_ANN_RECALL_Q16),
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
            min_recall_q16: Some(MIN_ANN_RECALL_Q16),
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
    let exact_results = search_persisted_vectors(vectors, query, allowed, limit);
    let available = vectors.keys().filter(|id| allowed.contains(id)).count();
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
                min_recall_q16: None,
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
            None,
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
        search.min_recall_q16 = Some(MIN_ANN_RECALL_Q16);
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

fn exact(
    vectors: &BTreeMap<u32, Vec<i16>>,
    query: &[i16],
    allowed: &BTreeSet<u32>,
    limit: usize,
    available: usize,
    graph_nodes: usize,
    reason: AnnFallbackReason,
) -> AnnSearchOutcome {
    let results = search_persisted_vectors(vectors, query, allowed, limit);
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
