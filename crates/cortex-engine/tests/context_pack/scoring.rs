use super::common::prelude::*;
use super::common::retrieved;

#[test]
fn test_deterministic_cosine_similarity() {
    let cells = vec![
        retrieved(1, "scope=project\nvector=100, 200, 300\ncell 1"),
        retrieved(2, "scope=project\nvector=100, 200, 300\ncell 2"), // Identical vectors!
    ];
    let pack = ContextPack::from_retrieved_with_options(
        cells,
        1_000,
        false,
        &ContextPackOptions {
            reduce_redundancy: true,
            redundancy_threshold_q16: 65535, // Exact match
            ..ContextPackOptions::default()
        },
        "",
    );
    // Pruned cell 2 because the vectors are exactly identical (cosine similarity = 65535 >= threshold)
    assert_eq!(pack.cells.len(), 1);
    assert_eq!(pack.anomalies.len(), 1);
}

#[test]
fn test_numeric_guard_coexistence() {
    let cells = vec![
        retrieved(1, "scope=project:investments\nstatus=ready\nproject=Solar\nmetric=budget\nvalue=1200000000\ncurrency=KZT\n\nSolar Plant budget is 1.2B"),
        retrieved(2, "scope=project:investments\nstatus=ready\nproject=Solar\nmetric=budget\nvalue=1400000000\ncurrency=KZT\n\nSolar Plant budget is 1.4B"), // Conflicting values!
    ];
    let pack = ContextPack::from_retrieved_with_options(
        cells,
        2_000,
        false,
        &ContextPackOptions {
            reduce_redundancy: true,
            redundancy_threshold_q16: 10, // Very low threshold, but should not prune!
            ..ContextPackOptions::default()
        },
        "",
    );
    // Both are kept together because they represent different values for same project+metric (Numeric Guard!)
    assert_eq!(pack.cells.len(), 2);
    assert_eq!(pack.visible_conflict_count, 1);
    assert_eq!(pack.conflict_visibility_q16, 32_767);
}

#[test]
fn test_context_pack_scoring_and_explain() {
    let cells = vec![
        retrieved(1, "scope=project:investments\nstatus=ready\nsource=report_q1\nproject=Solar\nmetric=budget\nvalue=12\ncurrency=KZT\n\nSolar Plant budget is 1.2B"),
    ];
    let pack = ContextPack::from_retrieved_with_options(
        cells,
        1_000,
        false,
        &ContextPackOptions::default(),
        "RETRIEVE CONTEXT FOR TASK \"budget\" IN BRAIN investment_projects WHERE space = project:investments LIMIT 10 CANDIDATES;",
    );
    let cell = &pack.cells[0];
    let exp = cell.explain.as_ref().unwrap();
    assert_eq!(exp.matched_terms, vec!["budget"]);
    assert_eq!(exp.base_bm25, 10_000);
    assert!(exp.score > 0);
    assert!(!exp.why_selected.is_empty());
    assert_eq!(exp.score_components.len(), 4);
    let component_names = exp
        .score_components
        .iter()
        .map(|component| component.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        component_names,
        vec![
            "base_bm25",
            "source_trust_bonus",
            "source_freshness_bonus",
            "redundancy_penalty"
        ]
    );
    assert_eq!(exp.source_trust_q16, 32_768);
    assert_eq!(exp.source_trust_category, SourceTrustCategory::Unknown);
    assert_eq!(exp.source_freshness_q16, 0);
    assert_eq!(exp.source_freshness_category.as_str(), "unknown");
    assert!(exp
        .score_components
        .iter()
        .any(|component| component.name == "redundancy_penalty" && component.contribution <= 0));
}

#[test]
fn context_pack_explain_reports_source_trust_category() {
    let cells = vec![retrieved(
        1,
        "scope=project:investments\nstatus=ready\nsource_trust_q16=60000\n\nalpha budget",
    )];
    let pack = ContextPack::from_retrieved_with_options(
        cells,
        1_000,
        false,
        &ContextPackOptions::default(),
        "RETRIEVE CONTEXT FOR TASK \"budget\" IN BRAIN investment_projects;",
    );
    let explain = pack.cells[0].explain.as_ref().unwrap();

    assert_eq!(explain.source_trust_q16, 60_000);
    assert_eq!(explain.source_trust_category, SourceTrustCategory::Official);
    assert!(explain
        .score_components
        .iter()
        .any(|component| component.name == "source_trust_bonus"
            && component.reason.contains("official provenance trust")));
}

#[test]
fn context_pack_explain_reports_calibrated_source_trust_class() {
    let cells = vec![retrieved(
        1,
        "scope=project:investments\nstatus=ready\nsource_trust_class=internal\n\nalpha budget",
    )];
    let pack = ContextPack::from_retrieved_with_options(
        cells,
        1_000,
        false,
        &ContextPackOptions::default(),
        "RETRIEVE CONTEXT FOR TASK \"budget\" IN BRAIN investment_projects;",
    );
    let explain = pack.cells[0].explain.as_ref().unwrap();

    assert_eq!(explain.source_trust_q16, INTERNAL_SOURCE_TRUST_Q16);
    assert_eq!(explain.source_trust_category, SourceTrustCategory::High);
    assert!(explain
        .score_components
        .iter()
        .any(|component| component.name == "source_trust_bonus"
            && component
                .reason
                .contains("internal calibrated provenance trust")));
}
