use std::collections::BTreeMap;

use crate::database::Database;

use super::super::ann::{
    finalize_report, AnnFallbackReason, AnnSearchPath, AnnSearchPolicy, AnnSearchReport,
};
use super::super::{SearchMode, SearchQuery};

pub(super) fn snapshot_ann_report(
    db: &Database,
    query: SearchQuery<'_>,
    allowed_candidates: usize,
    returned_candidates: usize,
    policy: AnnSearchPolicy,
) -> Option<AnnSearchReport> {
    if query.mode != SearchMode::Vector {
        return None;
    }
    let reason = if db.manifest().live_segments.is_empty() {
        AnnFallbackReason::NoPersistedSegments
    } else {
        AnnFallbackReason::UncheckpointedChanges
    };
    Some(finalize_report(
        AnnSearchReport {
            path: AnnSearchPath::ExactFallback,
            fallback_reason: Some(reason),
            fallback_performed: true,
            requested_limit: query.limit,
            allowed_candidates,
            graph_nodes: 0,
            returned_candidates,
            visited_candidates: 0,
            max_visited_candidates: policy.max_visited_candidates,
            recall_q16: None,
            min_recall_q16: policy.min_recall_q16,
            hnsw_max_neighbors: 0,
            hnsw_ef_search: 0,
            hnsw_layer_count: 0,
            hnsw_ef_construction: 0,
            upper_graph_edges: 0,
            require_slo: policy.require_slo,
            production_safe: true,
            slo_violations: Vec::new(),
            sparse_exact_fallback_ratio_bps: None,
        },
        policy,
    ))
}

pub(super) fn persisted_exact_fallback_report(
    requested_limit: usize,
    allowed_candidates: usize,
    returned_candidates: usize,
    policy: AnnSearchPolicy,
) -> AnnSearchReport {
    finalize_report(
        AnnSearchReport {
            path: AnnSearchPath::ExactFallback,
            fallback_reason: Some(AnnFallbackReason::HnswDisabled),
            fallback_performed: true,
            requested_limit,
            allowed_candidates,
            graph_nodes: 0,
            returned_candidates,
            visited_candidates: allowed_candidates,
            max_visited_candidates: policy.max_visited_candidates,
            recall_q16: None,
            min_recall_q16: policy.min_recall_q16,
            hnsw_max_neighbors: 0,
            hnsw_ef_search: 0,
            hnsw_layer_count: 0,
            hnsw_ef_construction: 0,
            upper_graph_edges: 0,
            require_slo: policy.require_slo,
            production_safe: true,
            slo_violations: Vec::new(),
            sparse_exact_fallback_ratio_bps: None,
        },
        policy,
    )
}

pub(super) fn persisted_hnsw_fault_fallback_report(
    requested_limit: usize,
    allowed_candidates: usize,
    returned_candidates: usize,
    policy: AnnSearchPolicy,
) -> AnnSearchReport {
    finalize_report(
        AnnSearchReport {
            path: AnnSearchPath::ExactFallback,
            fallback_reason: Some(AnnFallbackReason::InvalidGraph),
            fallback_performed: true,
            requested_limit,
            allowed_candidates,
            graph_nodes: 0,
            returned_candidates,
            visited_candidates: 0,
            max_visited_candidates: policy.max_visited_candidates,
            recall_q16: None,
            min_recall_q16: policy.min_recall_q16,
            hnsw_max_neighbors: 0,
            hnsw_ef_search: 0,
            hnsw_layer_count: 0,
            hnsw_ef_construction: 0,
            upper_graph_edges: 0,
            require_slo: policy.require_slo,
            production_safe: true,
            slo_violations: Vec::new(),
            sparse_exact_fallback_ratio_bps: None,
        },
        policy,
    )
}

pub(super) fn persisted_hnsw_stale_fallback_report(
    graph: &cortex_storage::hnsw::HnswGraphIndex,
    requested_limit: usize,
    allowed_candidates: usize,
    returned_candidates: usize,
    policy: AnnSearchPolicy,
) -> AnnSearchReport {
    finalize_report(
        AnnSearchReport {
            path: AnnSearchPath::ExactFallback,
            fallback_reason: Some(AnnFallbackReason::StaleGraph),
            fallback_performed: true,
            requested_limit,
            allowed_candidates,
            graph_nodes: graph.links.len(),
            returned_candidates,
            visited_candidates: 0,
            max_visited_candidates: policy.max_visited_candidates,
            recall_q16: None,
            min_recall_q16: policy.min_recall_q16,
            hnsw_max_neighbors: graph.max_neighbors as usize,
            hnsw_ef_search: graph.ef_search as usize,
            hnsw_layer_count: graph.layer_count as usize,
            hnsw_ef_construction: graph.ef_construction as usize,
            upper_graph_edges: graph
                .upper_layers
                .values()
                .flat_map(|links| links.values())
                .map(|neighbors| neighbors.len())
                .sum(),
            require_slo: policy.require_slo,
            production_safe: true,
            slo_violations: Vec::new(),
            sparse_exact_fallback_ratio_bps: None,
        },
        policy,
    )
}

pub(super) fn persisted_graph_is_stale(
    vectors: &BTreeMap<u32, Vec<i16>>,
    graph: &cortex_storage::hnsw::HnswGraphIndex,
    query: &[i16],
    allowed: &std::collections::BTreeSet<u32>,
) -> bool {
    vectors.iter().any(|(candidate, vector)| {
        allowed.contains(candidate)
            && vector.len() == query.len()
            && !graph.links.contains_key(candidate)
    })
}
