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
    NoPersistedSegments,
    UncheckpointedChanges,
}

impl AnnFallbackReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EmptyGraph => "empty_graph",
            Self::InvalidGraph => "invalid_graph",
            Self::InsufficientResults => "insufficient_results",
            Self::NoPersistedSegments => "no_persisted_segments",
            Self::UncheckpointedChanges => "uncheckpointed_changes",
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnnSearchOutcome {
    pub results: Vec<ScoredCandidate>,
    pub report: AnnSearchReport,
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

    let index = HnswIndex::from_graph(vectors.clone(), graph.clone(), 8, 64);
    if !index.verify_hnsw_integrity() {
        return exact(
            vectors,
            query,
            allowed,
            limit,
            available,
            graph_nodes,
            AnnFallbackReason::InvalidGraph,
        );
    }

    let ann = index.search_allowed(query, allowed, limit);
    if ann.len() < expected {
        return exact(
            vectors,
            query,
            allowed,
            limit,
            available,
            graph_nodes,
            AnnFallbackReason::InsufficientResults,
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
    let results = search_persisted_vectors(vectors, query, allowed, limit);
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
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_graph_falls_back_to_exact() {
        let outcome = search_persisted_ann(
            &BTreeMap::from([(1, vec![10, 0]), (2, vec![0, 10])]),
            &HnswGraphIndex::default(),
            &[0, 5],
            &BTreeSet::from([1, 2]),
            1,
        );

        assert_eq!(outcome.results[0].cell_id, 2);
        assert_eq!(outcome.report.path, AnnSearchPath::ExactFallback);
        assert_eq!(
            outcome.report.fallback_reason,
            Some(AnnFallbackReason::EmptyGraph)
        );
    }

    #[test]
    fn invalid_graph_falls_back_to_exact() {
        let outcome = search_persisted_ann(
            &BTreeMap::from([(1, vec![10, 0]), (2, vec![0, 10])]),
            &HnswGraphIndex {
                links: BTreeMap::from([(1, BTreeSet::from([999]))]),
            },
            &[0, 5],
            &BTreeSet::from([1, 2]),
            1,
        );

        assert_eq!(outcome.results[0].cell_id, 2);
        assert_eq!(
            outcome.report.fallback_reason,
            Some(AnnFallbackReason::InvalidGraph)
        );
    }

    #[test]
    fn incomplete_graph_results_fall_back_to_exact() {
        let outcome = search_persisted_ann(
            &BTreeMap::from([(1, vec![10, 0]), (2, vec![0, 10])]),
            &HnswGraphIndex {
                links: BTreeMap::from([(1, BTreeSet::new())]),
            },
            &[0, 5],
            &BTreeSet::from([1, 2]),
            2,
        );

        assert_eq!(outcome.results.len(), 2);
        assert_eq!(outcome.results[0].cell_id, 2);
        assert_eq!(
            outcome.report.fallback_reason,
            Some(AnnFallbackReason::InsufficientResults)
        );
    }
}
