use std::collections::{BTreeMap, BTreeSet};

use cortex_storage::hnsw::HnswGraphIndex;

use super::search::{
    allowed_ratio_within_bps, search_persisted_ann_sampled, search_persisted_ann_with_policy,
    SPARSE_ALLOWED_EXACT_FALLBACK_MAX_CANDIDATES, SPARSE_ALLOWED_EXACT_FALLBACK_MAX_RATIO_BPS,
};
use super::types::{AnnFallbackReason, AnnSearchPath, AnnSearchPolicy, MIN_ANN_RECALL_Q16};

// A3.3: build a small, well-connected HNSW graph over a full allowed set so a
// query takes the HNSW path (no sparse/budget/insufficient fallback).
fn dense_ring_graph() -> (BTreeMap<u32, Vec<i16>>, HnswGraphIndex, BTreeSet<u32>) {
    let mut vectors = BTreeMap::new();
    let mut links = BTreeMap::new();
    let n = 40u32;
    for id in 1..=n {
        vectors.insert(id, vec![id as i16, (n - id) as i16]);
        // Ring + skip links for connectivity.
        let a = id % n + 1;
        let b = (id + 3) % n + 1;
        links.insert(id, BTreeSet::from([a, b]));
    }
    let graph = HnswGraphIndex {
        links,
        dimension: 2,
        metric: 0,
        ..HnswGraphIndex::default()
    };
    let allowed: BTreeSet<u32> = (1..=n).collect();
    (vectors, graph, allowed)
}

// A3.3: the sampled (unsampled-query) path serves ANN with the collection's
// windowed recall and does NOT recompute exact recall — the report carries the
// supplied windowed value verbatim, and it still returns ANN results.
#[test]
fn sampled_path_serves_ann_with_windowed_recall() {
    let (vectors, graph, allowed) = dense_ring_graph();
    let policy = AnnSearchPolicy {
        min_recall_q16: Some(1_000), // low floor so the windowed value passes
        fallback: true,
        fallback_scan_cap: None,
        max_visited_candidates: None,
        require_slo: false,
    };
    let windowed = 50_000u16;
    let outcome =
        search_persisted_ann_sampled(&vectors, &graph, &[20, 20], &allowed, 3, policy, windowed);

    assert_eq!(outcome.report.path, AnnSearchPath::HnswGraph);
    assert_eq!(
        outcome.report.recall_q16,
        Some(windowed),
        "the report must carry the windowed recall, not a recomputed one"
    );
    assert!(!outcome.results.is_empty());
}

// A3.3: a windowed value below the floor still falls back to exact serving
// (correctness preserved even on the skip path).
#[test]
fn sampled_path_below_floor_falls_back_to_exact() {
    let (vectors, graph, allowed) = dense_ring_graph();
    let policy = AnnSearchPolicy {
        min_recall_q16: Some(50_000),
        fallback: true,
        fallback_scan_cap: None,
        max_visited_candidates: None,
        require_slo: false,
    };
    let outcome =
        search_persisted_ann_sampled(&vectors, &graph, &[20, 20], &allowed, 3, policy, 10_000);

    assert_eq!(outcome.report.path, AnnSearchPath::ExactFallback);
    assert_eq!(
        outcome.report.fallback_reason,
        Some(AnnFallbackReason::LowRecall)
    );
}

#[test]
fn allowed_ratio_threshold_is_explicit_and_codified() {
    // A3.2: the sparse-scope exact fallback triggers at <= 25% allowed ratio.
    let graph = 20_000;
    // 1% visibility -> exact fallback.
    assert!(allowed_ratio_within_bps(
        200,
        graph,
        SPARSE_ALLOWED_EXACT_FALLBACK_MAX_RATIO_BPS
    ));
    // Exactly 25% -> still within the threshold (inclusive).
    assert!(allowed_ratio_within_bps(
        5_000,
        graph,
        SPARSE_ALLOWED_EXACT_FALLBACK_MAX_RATIO_BPS
    ));
    // 25.005% -> above the threshold, HNSW traversal.
    assert!(!allowed_ratio_within_bps(
        5_001,
        graph,
        SPARSE_ALLOWED_EXACT_FALLBACK_MAX_RATIO_BPS
    ));
    // Empty graph never routes to the ratio fallback.
    assert!(!allowed_ratio_within_bps(
        1,
        0,
        SPARSE_ALLOWED_EXACT_FALLBACK_MAX_RATIO_BPS
    ));
    // Codified check matches the previous implicit `available * 4 <= graph`.
    for available in [0usize, 1, 100, 5_000, 5_001, 10_000] {
        assert_eq!(
            allowed_ratio_within_bps(
                available,
                graph,
                SPARSE_ALLOWED_EXACT_FALLBACK_MAX_RATIO_BPS
            ),
            available.saturating_mul(4) <= graph,
            "ratio codification must preserve the prior behavior at {available}"
        );
    }
}

#[test]
fn sparse_allowed_set_routes_to_exact_before_hnsw_budget() {
    let mut vectors = BTreeMap::new();
    let mut links = BTreeMap::new();
    for id in 1..=96 {
        vectors.insert(id, vec![0, id as i16]);
        links.insert(id, BTreeSet::from([id + 1]));
    }
    vectors.insert(97, vec![0, 97]);
    links.insert(97, BTreeSet::new());
    vectors.insert(1_000, vec![100, 0]);
    vectors.insert(1_001, vec![90, 0]);

    let outcome = search_persisted_ann_with_policy(
        &vectors,
        &HnswGraphIndex {
            links,
            dimension: 2,
            metric: 0,
            ..HnswGraphIndex::default()
        },
        &[100, 0],
        &BTreeSet::from([1_000, 1_001]),
        2,
        AnnSearchPolicy {
            min_recall_q16: Some(MIN_ANN_RECALL_Q16),
            fallback: true,
            fallback_scan_cap: None,
            max_visited_candidates: Some(1),
            require_slo: false,
        },
    );

    assert_eq!(outcome.report.path, AnnSearchPath::ExactFallback);
    assert_eq!(
        outcome.report.fallback_reason,
        Some(AnnFallbackReason::SparseAllowedSet)
    );
    assert_eq!(outcome.results.len(), 2);
    assert_eq!(outcome.results[0].cell_id, 1_000);
    assert_eq!(outcome.report.allowed_candidates, 2);
    assert!(outcome.report.allowed_candidates <= SPARSE_ALLOWED_EXACT_FALLBACK_MAX_CANDIDATES);
    // A3.2: the qualifying allowed/graph ratio (bps) is recorded on the sparse
    // exact-fallback path, and is within the codified threshold.
    let ratio = outcome
        .report
        .sparse_exact_fallback_ratio_bps
        .expect("sparse fallback ratio recorded");
    assert!(
        ratio <= SPARSE_ALLOWED_EXACT_FALLBACK_MAX_RATIO_BPS,
        "recorded ratio {ratio} bps must be within the {SPARSE_ALLOWED_EXACT_FALLBACK_MAX_RATIO_BPS} bps threshold"
    );
    assert_eq!(outcome.report.visited_candidates, 0);
    assert_eq!(outcome.report.max_visited_candidates, Some(1));
}
