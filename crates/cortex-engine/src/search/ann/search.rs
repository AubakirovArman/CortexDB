use std::collections::{BTreeMap, BTreeSet};

use cortex_storage::hnsw::HnswGraphIndex;

use super::super::hnsw::HnswIndex;
use super::super::persisted::search_persisted_vectors;
use super::super::ScoredCandidate;
use super::outcomes::{exact, exact_from_results, fallback_disabled_outcome};
use super::report::{finalize_report, recall_q16};
use super::runtime::hnsw_runtime_config;
use super::types::{
    AnnFallbackReason, AnnSearchOutcome, AnnSearchPath, AnnSearchPolicy, AnnSearchReport,
    MIN_ANN_RECALL_Q16,
};

pub(super) const SPARSE_ALLOWED_EXACT_FALLBACK_MAX_CANDIDATES: usize = 64;

/// A3.2: explicit allowed-ratio threshold for the sparse-scope exact fallback.
/// When the permission-allowed set is at most this fraction of the graph
/// (in basis points; 2500 = 25%), an ANN query over it routes to an exact scan
/// of the allowed set (recall 1.0) instead of a budgeted HNSW traversal that
/// would spend its beam on out-of-scope nodes. Codifies the previous implicit
/// `available * 4 <= graph_nodes` check.
pub(super) const SPARSE_ALLOWED_EXACT_FALLBACK_MAX_RATIO_BPS: u64 = 2_500;

/// Returns true when `available` is at most `max_ratio_bps` basis points of
/// `graph_nodes`. Integer-only and overflow-safe.
pub(super) fn allowed_ratio_within_bps(
    available: usize,
    graph_nodes: usize,
    max_ratio_bps: u64,
) -> bool {
    if graph_nodes == 0 {
        return false;
    }
    // available/graph_nodes <= max_ratio_bps/10000, cross-multiplied.
    (available as u128) * 10_000 <= (graph_nodes as u128) * u128::from(max_ratio_bps)
}

/// A3.2: the allowed-set/graph-nodes ratio in basis points (integer-only), for
/// recording in `AnnSearchReport` when the sparse-scope exact fallback is chosen.
pub(super) fn sparse_allowed_ratio_bps(available: usize, graph_nodes: usize) -> u64 {
    if graph_nodes == 0 {
        return 0;
    }
    u64::try_from((available as u128) * 10_000 / (graph_nodes as u128)).unwrap_or(u64::MAX)
}

/// A3.3: the recall step's mode. `Exact` recomputes exact recall on the query
/// (the default / sampled path, byte-identical to pre-A3.3 behavior); `Windowed`
/// serves an unsampled guarded query with the collection's windowed recall and
/// skips the exact scan — the sampling perf win.
#[derive(Clone, Copy, Debug)]
pub enum RecallMode {
    Exact,
    Windowed(u16),
}

pub fn search_persisted_ann_with_policy(
    vectors: &BTreeMap<u32, Vec<i16>>,
    graph: &HnswGraphIndex,
    query: &[i16],
    allowed: &BTreeSet<u32>,
    limit: usize,
    policy: AnnSearchPolicy,
) -> AnnSearchOutcome {
    search_persisted_ann_inner(
        vectors,
        graph,
        query,
        allowed,
        limit,
        policy,
        RecallMode::Exact,
        None,
    )
}

/// A3.3: serve ANN for an unsampled guarded-recall query without recomputing exact
/// recall — the recall carried in the report is the collection's windowed value.
pub fn search_persisted_ann_sampled(
    vectors: &BTreeMap<u32, Vec<i16>>,
    graph: &HnswGraphIndex,
    query: &[i16],
    allowed: &BTreeSet<u32>,
    limit: usize,
    policy: AnnSearchPolicy,
    windowed_recall_q16: u16,
) -> AnnSearchOutcome {
    search_persisted_ann_inner(
        vectors,
        graph,
        query,
        allowed,
        limit,
        policy,
        RecallMode::Windowed(windowed_recall_q16),
        None,
    )
}

/// A3.3 perf: reuse a pre-built + verified `HnswIndex` (avoiding the per-query
/// rebuild) for the persisted read path. `recall_mode` selects the exact
/// (sampled) or windowed (unsampled) recall path.
#[allow(clippy::too_many_arguments)]
pub fn search_persisted_ann_cached(
    vectors: &BTreeMap<u32, Vec<i16>>,
    graph: &HnswGraphIndex,
    query: &[i16],
    allowed: &BTreeSet<u32>,
    limit: usize,
    policy: AnnSearchPolicy,
    recall_mode: RecallMode,
    cached_index: &HnswIndex,
) -> AnnSearchOutcome {
    search_persisted_ann_inner(
        vectors,
        graph,
        query,
        allowed,
        limit,
        policy,
        recall_mode,
        Some(cached_index),
    )
}

/// A3.3 perf: build + integrity-verify the `HnswIndex` for a persisted (vectors,
/// graph) using the SAME resolved runtime config as the per-query rebuild in
/// `search_hnsw` (the `0 -> default` mapping in `hnsw_runtime_config`). Both the
/// per-query None path and the cross-query cache go through this one builder, so a
/// cached index is byte-identical to what a per-query rebuild would have produced.
/// `None` means the graph failed the integrity walk (caller reports/handles it as
/// the `InvalidGraph` fallback).
pub fn build_verified_hnsw_index(
    vectors: &BTreeMap<u32, Vec<i16>>,
    graph: &HnswGraphIndex,
) -> Option<HnswIndex> {
    let config = hnsw_runtime_config(graph);
    let index = HnswIndex::from_graph(
        vectors.clone(),
        graph.clone(),
        config.max_neighbors,
        config.ef_search,
    );
    if !index.verify_hnsw_integrity() {
        return None;
    }
    Some(index)
}

#[allow(clippy::too_many_arguments)]
fn search_persisted_ann_inner(
    vectors: &BTreeMap<u32, Vec<i16>>,
    graph: &HnswGraphIndex,
    query: &[i16],
    allowed: &BTreeSet<u32>,
    limit: usize,
    policy: AnnSearchPolicy,
    recall_mode: RecallMode,
    cached_index: Option<&HnswIndex>,
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

    if should_use_sparse_allowed_exact_fallback(available, graph_nodes, policy) {
        return exact(
            vectors,
            query,
            allowed,
            limit,
            available,
            graph_nodes,
            config,
            AnnFallbackReason::SparseAllowedSet,
            policy,
        );
    }

    let (ann, visited_candidates, budget_exceeded) = match search_hnsw(
        vectors,
        graph,
        query,
        allowed,
        limit,
        policy.max_visited_candidates,
        cached_index,
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

    let effective_min_recall = policy.min_recall_q16.unwrap_or(MIN_ANN_RECALL_Q16);
    let (recall, precomputed_exact) = match recall_mode {
        // Default (sampled / non-guarded) path: recompute exact recall on this
        // query — byte-identical to the pre-A3.3 behavior.
        RecallMode::Exact => {
            let exact_results =
                search_persisted_vectors(vectors, query, allowed, limit, &config.metric);
            let exact_set = exact_results
                .iter()
                .map(|candidate| candidate.cell_id)
                .collect::<BTreeSet<_>>();
            let overlap = ann
                .iter()
                .filter(|candidate| exact_set.contains(&candidate.cell_id))
                .count();
            let recall = recall_q16(overlap, exact_results.len());
            (recall, Some(exact_results))
        }
        // A3.3: an unsampled guarded query serves ANN with the collection's
        // windowed recall and skips the exact scan (the sampling perf win).
        RecallMode::Windowed(windowed_recall_q16) => (windowed_recall_q16, None),
    };

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
        // A windowed query that dips below floor still needs an exact result set to
        // serve from; compute it now (rare — the guarded caller routes a degraded
        // collection to exact serving before it reaches this path).
        let exact_results = precomputed_exact.unwrap_or_else(|| {
            search_persisted_vectors(vectors, query, allowed, limit, &config.metric)
        });
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
                sparse_exact_fallback_ratio_bps: None,
            },
            policy,
        ),
    }
}

pub(super) fn should_use_sparse_allowed_exact_fallback(
    available: usize,
    graph_nodes: usize,
    policy: AnnSearchPolicy,
) -> bool {
    policy.fallback
        && policy.max_visited_candidates.is_some()
        && available > 0
        && available <= SPARSE_ALLOWED_EXACT_FALLBACK_MAX_CANDIDATES
        && allowed_ratio_within_bps(
            available,
            graph_nodes,
            SPARSE_ALLOWED_EXACT_FALLBACK_MAX_RATIO_BPS,
        )
}

pub(super) fn search_hnsw(
    vectors: &BTreeMap<u32, Vec<i16>>,
    graph: &HnswGraphIndex,
    query: &[i16],
    allowed: &BTreeSet<u32>,
    limit: usize,
    max_visited_candidates: Option<usize>,
    cached_index: Option<&HnswIndex>,
) -> Result<(Vec<ScoredCandidate>, usize, bool), AnnFallbackReason> {
    if graph.links.is_empty() {
        return Err(AnnFallbackReason::EmptyGraph);
    }
    // A3.3 perf: a caller that already holds a built + verified HnswIndex for this
    // (vectors, graph) generation passes it here, avoiding a per-query O(n)
    // `from_graph` clone and an O(n·edges) integrity walk — the dominant cost of
    // the persisted ANN query. `None` rebuilds + verifies as before (byte-identical
    // behavior for every existing caller).
    let built;
    let index = match cached_index {
        Some(index) => index,
        None => {
            built = match build_verified_hnsw_index(vectors, graph) {
                Some(index) => index,
                None => return Err(AnnFallbackReason::InvalidGraph),
            };
            &built
        }
    };
    let (results, visited, budget_exceeded) =
        index.search_allowed_with_budget(query, allowed, limit, max_visited_candidates);
    Ok((results, visited, budget_exceeded))
}
