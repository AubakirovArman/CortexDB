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
            max_visited_candidates: None,
            require_slo: false,
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
            max_visited_candidates: None,
            require_slo: false,
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
            max_visited_candidates: None,
            require_slo: false,
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
fn hnsw_budget_exceeded_falls_back_to_exact() {
    let outcome = search_persisted_ann_with_policy(
        &BTreeMap::from([(1, vec![10, 0]), (2, vec![0, 10]), (3, vec![9, 1])]),
        &HnswGraphIndex {
            links: BTreeMap::from([
                (1, BTreeSet::from([2, 3])),
                (2, BTreeSet::from([1, 3])),
                (3, BTreeSet::from([1, 2])),
            ]),
            dimension: 2,
            metric: 0,
        },
        &[0, 10],
        &BTreeSet::from([1, 2, 3]),
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
        Some(AnnFallbackReason::VisitBudgetExceeded)
    );
    assert_eq!(outcome.report.visited_candidates, 0);
    assert_eq!(outcome.report.max_visited_candidates, Some(1));
}

#[test]
fn slo_report_flags_fallback_as_not_production_safe() {
    let outcome = search_persisted_ann_with_policy(
        &BTreeMap::from([(1, vec![10, 0]), (2, vec![0, 10])]),
        &HnswGraphIndex::default(),
        &[0, 5],
        &BTreeSet::from([1, 2]),
        1,
        AnnSearchPolicy {
            min_recall_q16: Some(MIN_ANN_RECALL_Q16),
            fallback: true,
            fallback_scan_cap: None,
            max_visited_candidates: None,
            require_slo: true,
        },
    );

    assert_eq!(outcome.report.path, AnnSearchPath::ExactFallback);
    assert!(outcome.report.fallback_performed);
    assert!(outcome.report.require_slo);
    assert!(!outcome.report.production_safe);
    assert!(outcome
        .report
        .slo_violations
        .contains(&AnnSloViolation::EmptyGraph));
}

#[test]
fn slo_report_marks_healthy_graph_as_production_safe() {
    let outcome = search_persisted_ann_with_policy(
        &BTreeMap::from([(1, vec![10, 0]), (2, vec![0, 10]), (3, vec![1, 9])]),
        &HnswGraphIndex {
            links: BTreeMap::from([(1, BTreeSet::from([2])), (2, BTreeSet::from([3]))]),
            dimension: 2,
            metric: 0,
        },
        &[0, 10],
        &BTreeSet::from([1, 2, 3]),
        2,
        AnnSearchPolicy {
            min_recall_q16: Some(MIN_ANN_RECALL_Q16),
            fallback: true,
            fallback_scan_cap: None,
            max_visited_candidates: None,
            require_slo: true,
        },
    );

    assert_eq!(outcome.report.path, AnnSearchPath::HnswGraph);
    assert!(!outcome.report.fallback_performed);
    assert!(outcome.report.production_safe);
    assert!(outcome.report.slo_violations.is_empty());
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
