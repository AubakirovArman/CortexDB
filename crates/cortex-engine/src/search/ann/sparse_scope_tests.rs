use std::collections::{BTreeMap, BTreeSet};

use cortex_storage::hnsw::HnswGraphIndex;

use super::search::{
    search_persisted_ann_with_policy, SPARSE_ALLOWED_EXACT_FALLBACK_MAX_CANDIDATES,
};
use super::types::{AnnFallbackReason, AnnSearchPath, AnnSearchPolicy, MIN_ANN_RECALL_Q16};

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
