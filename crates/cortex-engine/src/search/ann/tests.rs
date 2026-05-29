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
fn fallback_disabled_uses_scan_cap() {
    let outcome = search_persisted_ann_with_policy(
        &BTreeMap::from([(1, vec![10, 0]), (2, vec![0, 10])]),
        &HnswGraphIndex::default(),
        &[0, 5],
        &BTreeSet::from([1, 2]),
        2,
        AnnSearchPolicy {
            min_recall_q16: Some(10_000),
            fallback: false,
            fallback_scan_cap: Some(0),
        },
    );

    assert_eq!(outcome.results.len(), 0);
    assert_eq!(
        outcome.report.fallback_reason,
        Some(AnnFallbackReason::EmptyGraph)
    );
    assert_eq!(outcome.report.min_recall_q16, Some(10_000));
    assert_eq!(outcome.report.path, AnnSearchPath::HnswGraph);
}

#[test]
fn fallback_disabled_without_scan_cap_returns_empty_results() {
    let outcome = search_persisted_ann_with_policy(
        &BTreeMap::from([(1, vec![10, 0]), (2, vec![0, 10])]),
        &HnswGraphIndex::default(),
        &[0, 5],
        &BTreeSet::from([1, 2]),
        2,
        AnnSearchPolicy {
            min_recall_q16: Some(10_000),
            fallback: false,
            fallback_scan_cap: None,
        },
    );

    assert_eq!(outcome.results.len(), 0);
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
            dimension: 2,
            metric: 0,
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
            dimension: 2,
            metric: 0,
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

#[test]
fn low_recall_graph_falls_back_to_exact() {
    let outcome = search_persisted_ann(
        &BTreeMap::from([(1, vec![10, 0]), (2, vec![0, 10])]),
        &HnswGraphIndex {
            links: BTreeMap::from([(1, BTreeSet::new())]),
            dimension: 2,
            metric: 0,
        },
        &[0, 10],
        &BTreeSet::from([1, 2]),
        1,
    );

    assert_eq!(outcome.results[0].cell_id, 2);
    assert_eq!(outcome.report.path, AnnSearchPath::ExactFallback);
    assert_eq!(
        outcome.report.fallback_reason,
        Some(AnnFallbackReason::LowRecall)
    );
    assert_eq!(outcome.report.recall_q16, Some(0));
    assert_eq!(outcome.report.min_recall_q16, Some(MIN_ANN_RECALL_Q16));
}

#[test]
fn policy_min_recall_controls_ann_report() {
    let outcome = search_persisted_ann_with_policy(
        &BTreeMap::from([(1, vec![10, 0]), (2, vec![0, 10])]),
        &HnswGraphIndex {
            links: BTreeMap::from([(1, BTreeSet::from([2]))]),
            dimension: 2,
            metric: 0,
        },
        &[0, 10],
        &BTreeSet::from([1, 2]),
        1,
        AnnSearchPolicy {
            min_recall_q16: Some(65_000),
            fallback: false,
            fallback_scan_cap: None,
        },
    );

    assert_eq!(outcome.report.min_recall_q16, Some(65_000));
}

#[test]
fn evaluation_reports_exact_overlap_and_recall() {
    let report = evaluate_persisted_ann(
        &BTreeMap::from([(1, vec![10, 0]), (2, vec![0, 10]), (3, vec![2, 8])]),
        &HnswGraphIndex {
            links: BTreeMap::from([(1, BTreeSet::from([2])), (2, BTreeSet::from([3]))]),
            dimension: 2,
            metric: 0,
        },
        &[0, 10],
        &BTreeSet::from([1, 2, 3]),
        2,
    );

    assert_eq!(report.exact_top_k, vec![2, 3]);
    assert_eq!(report.ann_top_k, vec![2, 3]);
    assert_eq!(report.overlap_count, 2);
    assert_eq!(report.recall_q16, 65_535);
    assert_eq!(report.search.recall_q16, Some(65_535));
    assert_eq!(report.search.min_recall_q16, Some(MIN_ANN_RECALL_Q16));
}

#[test]
fn evaluation_exposes_raw_low_recall_without_guarded_fallback() {
    let report = evaluate_persisted_ann(
        &BTreeMap::from([(1, vec![10, 0]), (2, vec![0, 10])]),
        &HnswGraphIndex {
            links: BTreeMap::from([(1, BTreeSet::new())]),
            dimension: 2,
            metric: 0,
        },
        &[0, 10],
        &BTreeSet::from([1, 2]),
        1,
    );

    assert_eq!(report.search.path, AnnSearchPath::HnswGraph);
    assert_eq!(report.exact_top_k, vec![2]);
    assert_eq!(report.ann_top_k, vec![1]);
    assert_eq!(report.overlap_count, 0);
    assert_eq!(report.recall_q16, 0);
}

#[test]
fn evaluation_treats_empty_exact_set_as_full_recall() {
    let report = evaluate_persisted_ann(
        &BTreeMap::from([(1, vec![10, 0])]),
        &HnswGraphIndex::default(),
        &[0, 10],
        &BTreeSet::new(),
        2,
    );

    assert!(report.exact_top_k.is_empty());
    assert_eq!(report.overlap_count, 0);
    assert_eq!(report.recall_q16, 65_535);
}
