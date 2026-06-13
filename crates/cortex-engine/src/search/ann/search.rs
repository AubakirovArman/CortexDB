use std::collections::{BTreeMap, BTreeSet};

use cortex_storage::hnsw::HnswGraphIndex;

use super::super::hnsw::HnswIndex;
use super::super::persisted::search_persisted_vectors;
use super::super::ScoredCandidate;
use super::outcomes::{exact, exact_from_results, fallback_disabled_outcome};
use super::report::{finalize_report, recall_q16};
use super::runtime::{hnsw_runtime_config, HnswRuntimeConfig};
use super::types::{
    AnnFallbackReason, AnnSearchOutcome, AnnSearchPath, AnnSearchPolicy, AnnSearchReport,
    MIN_ANN_RECALL_Q16,
};

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
    let config = hnsw_runtime_config(graph);

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
                config,
                policy,
                0,
            );
        }
        return exact(
            vectors,
            query,
            allowed,
            limit,
            available,
            graph_nodes,
            config,
            AnnFallbackReason::EmptyGraph,
            policy,
        );
    }

    let (ann, visited_candidates, budget_exceeded) = match search_hnsw(
        vectors,
        graph,
        query,
        allowed,
        limit,
        config,
        policy.max_visited_candidates,
    ) {
        Ok(value) => value,
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
                    config,
                    policy,
                    0,
                );
            }
            return exact(
                vectors,
                query,
                allowed,
                limit,
                available,
                graph_nodes,
                config,
                reason,
                policy,
            );
        }
    };

    if budget_exceeded {
        if !policy.fallback {
            return fallback_disabled_outcome(
                AnnFallbackReason::VisitBudgetExceeded,
                vectors,
                query,
                allowed,
                limit,
                available,
                graph_nodes,
                config,
                policy,
                visited_candidates,
            );
        }

        return exact(
            vectors,
            query,
            allowed,
            limit,
            available,
            graph_nodes,
            config,
            AnnFallbackReason::VisitBudgetExceeded,
            policy,
        );
    }

    if ann.len() < expected {
        if !policy.fallback {
            return fallback_disabled_outcome(
                AnnFallbackReason::InsufficientResults,
                vectors,
                query,
                allowed,
                limit,
                available,
                graph_nodes,
                config,
                policy,
                visited_candidates,
            );
        }
        return exact(
            vectors,
            query,
            allowed,
            limit,
            available,
            graph_nodes,
            config,
            AnnFallbackReason::InsufficientResults,
            policy,
        );
    }

    let exact_results = search_persisted_vectors(vectors, query, allowed, limit, &config.metric);
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
                config,
                policy,
                visited_candidates,
            );
        }
        return exact_from_results(
            exact_results,
            limit,
            available,
            graph_nodes,
            config,
            AnnFallbackReason::LowRecall,
            Some(recall),
            policy,
        );
    }

    let returned_candidates = ann.len();
    AnnSearchOutcome {
        results: ann,
        report: finalize_report(
            AnnSearchReport {
                path: AnnSearchPath::HnswGraph,
                fallback_reason: None,
                fallback_performed: false,
                requested_limit: limit,
                allowed_candidates: available,
                graph_nodes,
                returned_candidates,
                visited_candidates,
                max_visited_candidates: policy.max_visited_candidates,
                recall_q16: Some(recall),
                min_recall_q16: policy.min_recall_q16,
                hnsw_max_neighbors: config.max_neighbors,
                hnsw_ef_search: config.ef_search,
                hnsw_layer_count: config.layer_count,
                hnsw_ef_construction: config.ef_construction,
                upper_graph_edges: config.upper_graph_edges,
                require_slo: policy.require_slo,
                production_safe: true,
                slo_violations: Vec::new(),
            },
            policy,
        ),
    }
}

pub(super) fn search_hnsw(
    vectors: &BTreeMap<u32, Vec<i16>>,
    graph: &HnswGraphIndex,
    query: &[i16],
    allowed: &BTreeSet<u32>,
    limit: usize,
    config: HnswRuntimeConfig,
    max_visited_candidates: Option<usize>,
) -> Result<(Vec<ScoredCandidate>, usize, bool), AnnFallbackReason> {
    if graph.links.is_empty() {
        return Err(AnnFallbackReason::EmptyGraph);
    }
    let index = HnswIndex::from_graph(
        vectors.clone(),
        graph.clone(),
        config.max_neighbors,
        config.ef_search,
    );
    if !index.verify_hnsw_integrity() {
        return Err(AnnFallbackReason::InvalidGraph);
    }
    let (results, visited, budget_exceeded) =
        index.search_allowed_with_budget(query, allowed, limit, max_visited_candidates);
    Ok((results, visited, budget_exceeded))
}
