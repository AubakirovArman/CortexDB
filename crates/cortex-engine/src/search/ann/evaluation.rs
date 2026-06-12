use std::collections::{BTreeMap, BTreeSet};

use cortex_storage::hnsw::HnswGraphIndex;

use super::super::persisted::search_persisted_vectors;
use super::report::{finalize_report, recall_q16};
use super::runtime::{hnsw_runtime_config, metric_from_graph};
use super::search::{exact_from_results, search_hnsw};
use super::types::{
    AnnEvaluationReport, AnnFallbackReason, AnnSearchOutcome, AnnSearchPath, AnnSearchPolicy,
    AnnSearchReport,
};

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
        search_persisted_vectors(vectors, query, allowed, limit, &metric_from_graph(graph));
    let exact_top_k = exact_results
        .iter()
        .map(|candidate| candidate.cell_id)
        .collect::<Vec<_>>();
    let available = vectors
        .iter()
        .filter(|(id, vector)| allowed.contains(id) && vector.len() == query.len())
        .count();
    let graph_nodes = graph.links.len();
    let config = hnsw_runtime_config(graph);

    let ann_outcome = match search_hnsw(
        vectors,
        graph,
        query,
        allowed,
        limit,
        config,
        policy.max_visited_candidates,
    ) {
        Ok((results, visited_candidates, budget_exceeded)) => {
            if budget_exceeded {
                exact_from_results(
                    exact_results.clone(),
                    limit,
                    available,
                    graph_nodes,
                    config,
                    AnnFallbackReason::VisitBudgetExceeded,
                    None,
                    policy,
                )
            } else {
                let overlap = results
                    .iter()
                    .filter(|candidate| exact_top_k.iter().any(|exact| exact == &candidate.cell_id))
                    .count();
                let recall = recall_q16(overlap, exact_results.len());
                let max_visited_candidates = policy.max_visited_candidates;
                AnnSearchOutcome {
                    results: results.clone(),
                    report: finalize_report(
                        AnnSearchReport {
                            path: AnnSearchPath::HnswGraph,
                            fallback_reason: None,
                            fallback_performed: false,
                            requested_limit: limit,
                            allowed_candidates: available,
                            graph_nodes,
                            returned_candidates: results.len(),
                            visited_candidates,
                            max_visited_candidates,
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
        }
        Err(reason) => exact_from_results(
            exact_results.clone(),
            limit,
            available,
            graph_nodes,
            config,
            reason,
            None,
            policy,
        ),
    };

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
        search = finalize_report(search, policy);
    }

    AnnEvaluationReport {
        search,
        exact_top_k,
        ann_top_k,
        overlap_count,
        recall_q16: recall,
    }
}
