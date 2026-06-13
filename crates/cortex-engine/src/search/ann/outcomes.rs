use std::collections::{BTreeMap, BTreeSet};

use super::super::persisted::search_persisted_vectors;
use super::super::ScoredCandidate;
use super::report::finalize_report;
use super::runtime::HnswRuntimeConfig;
use super::types::{
    AnnFallbackReason, AnnSearchOutcome, AnnSearchPath, AnnSearchPolicy, AnnSearchReport,
};

#[allow(clippy::too_many_arguments)]
pub(super) fn fallback_disabled_outcome(
    reason: AnnFallbackReason,
    vectors: &BTreeMap<u32, Vec<i16>>,
    query: &[i16],
    allowed: &BTreeSet<u32>,
    limit: usize,
    available: usize,
    graph_nodes: usize,
    config: HnswRuntimeConfig,
    policy: AnnSearchPolicy,
    visited_candidates: usize,
) -> AnnSearchOutcome {
    let cap = if policy.fallback {
        policy.fallback_scan_cap.unwrap_or(limit).min(limit)
    } else {
        policy.fallback_scan_cap.unwrap_or(0).min(limit)
    };
    let ann = if cap == 0 {
        Vec::new()
    } else {
        search_persisted_vectors(vectors, query, allowed, cap, &config.metric)
    };
    let returned = ann.len();
    AnnSearchOutcome {
        results: ann,
        report: finalize_report(
            AnnSearchReport {
                path: AnnSearchPath::HnswGraph,
                fallback_reason: Some(reason),
                fallback_performed: false,
                requested_limit: limit,
                allowed_candidates: available,
                graph_nodes,
                returned_candidates: returned,
                visited_candidates,
                max_visited_candidates: policy.max_visited_candidates,
                recall_q16: None,
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

#[allow(clippy::too_many_arguments)]
pub(super) fn exact(
    vectors: &BTreeMap<u32, Vec<i16>>,
    query: &[i16],
    allowed: &BTreeSet<u32>,
    limit: usize,
    available: usize,
    graph_nodes: usize,
    config: HnswRuntimeConfig,
    reason: AnnFallbackReason,
    policy: AnnSearchPolicy,
) -> AnnSearchOutcome {
    let results = search_persisted_vectors(vectors, query, allowed, limit, &config.metric);
    exact_from_results(
        results,
        limit,
        available,
        graph_nodes,
        config,
        reason,
        None,
        policy,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn exact_from_results(
    results: Vec<ScoredCandidate>,
    limit: usize,
    available: usize,
    graph_nodes: usize,
    config: HnswRuntimeConfig,
    reason: AnnFallbackReason,
    recall_q16: Option<u16>,
    policy: AnnSearchPolicy,
) -> AnnSearchOutcome {
    let returned = results.len();
    AnnSearchOutcome {
        results,
        report: finalize_report(
            AnnSearchReport {
                path: AnnSearchPath::ExactFallback,
                fallback_reason: Some(reason),
                fallback_performed: true,
                requested_limit: limit,
                allowed_candidates: available,
                graph_nodes,
                returned_candidates: returned,
                visited_candidates: 0,
                max_visited_candidates: policy.max_visited_candidates,
                recall_q16,
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
