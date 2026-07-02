use std::collections::{BTreeMap, BTreeSet};

use cortex_storage::hnsw::HnswGraphIndex;

use super::search::{
    allowed_ratio_within_bps, search_persisted_ann_with_policy,
    SPARSE_ALLOWED_EXACT_FALLBACK_MAX_CANDIDATES, SPARSE_ALLOWED_EXACT_FALLBACK_MAX_RATIO_BPS,
};
use super::types::{AnnFallbackReason, AnnSearchPath, AnnSearchPolicy, MIN_ANN_RECALL_Q16};

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
    assert_eq!(outcome.report.visited_candidates, 0);
    assert_eq!(outcome.report.max_visited_candidates, Some(1));
}
