use cortex_core::CellId;
use cortex_engine::{
    ContextCellExplain, ContextCellExplainOutcome, ContextPack, ContextPackAnomalyCode,
    ContextPackOptions, RetrievedCell, SourceFreshnessCategory, SourceTrustCategory,
};

fn retrieved(cell_id: u64, payload: &str) -> RetrievedCell {
    RetrievedCell::from_payload(CellId(cell_id), payload.as_bytes().to_vec())
}

#[test]
fn context_pack_explain_v2_reports_selection_source_trust_and_score_components() {
    let cells = vec![
        retrieved(
            1,
            "scope=project:investments\nstatus=ready\nsource_trust_q16=60000\ncreated_unix_seconds=100\n\nsolar budget financing official disclosure",
        ),
        retrieved(
            2,
            "scope=project:investments\nstatus=ready\nsource_trust_q16=60000\ncreated_unix_seconds=200\n\nsolar budget financing official disclosure update",
        ),
    ];
    let pack = ContextPack::from_retrieved_with_options(
        cells,
        1_000,
        false,
        &ContextPackOptions::default(),
        "RETRIEVE CONTEXT FOR TASK \"solar budget\" IN BRAIN investment_projects;",
    );

    assert_eq!(pack.cells.len(), 2);
    let first = pack.cells[0].explain.as_ref().unwrap();
    assert_eq!(first.matched_terms, vec!["solar", "budget"]);
    assert_eq!(first.source_trust_q16, 60_000);
    assert_eq!(first.source_trust_category, SourceTrustCategory::Official);
    assert_eq!(
        first.source_freshness_category,
        SourceFreshnessCategory::Stale
    );
    assert!(first.why_selected.contains("high provenance source trust"));

    let components = first
        .score_components
        .iter()
        .map(|component| component.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        components,
        vec![
            "base_bm25",
            "source_trust_bonus",
            "source_freshness_bonus",
            "redundancy_penalty"
        ]
    );

    let second = pack.cells[1].explain.as_ref().unwrap();
    assert_eq!(
        second.source_freshness_category,
        SourceFreshnessCategory::Current
    );
    assert!(second.source_freshness_bonus > first.source_freshness_bonus);
    assert!(second.redundancy_penalty > 0);
    assert!(second.score_components.iter().any(|component| {
        component.name == "redundancy_penalty"
            && component.contribution < 0
            && component.reason.contains("weighted Jaccard")
    }));
}

#[test]
fn context_pack_explain_v2_reports_redundancy_exclusion_reason() {
    let cells = vec![
        retrieved(
            1,
            "scope=project:investments\nstatus=ready\n\nsolar budget financing disclosure",
        ),
        retrieved(
            2,
            "scope=project:investments\nstatus=ready\n\nsolar budget financing disclosure",
        ),
    ];
    let pack = ContextPack::from_retrieved_with_options(
        cells,
        1_000,
        false,
        &ContextPackOptions {
            reduce_redundancy: true,
            redundancy_threshold_q16: 10,
            ..ContextPackOptions::default()
        },
        "RETRIEVE CONTEXT FOR TASK \"solar budget\" IN BRAIN investment_projects;",
    );

    assert_eq!(pack.cells.len(), 1);
    assert_eq!(pack.anomalies.len(), 1);
    assert_eq!(
        pack.anomalies[0].code,
        ContextPackAnomalyCode::RedundantCell
    );
    let reason = pack.anomalies[0].why_excluded.as_deref().unwrap();
    assert!(reason.contains("reduce_redundancy"));
    assert!(reason.contains("threshold"));
}

#[test]
fn context_pack_explain_v2_reports_token_budget_exclusion_reason() {
    let cells = vec![
        retrieved(
            1,
            "scope=project:investments\nstatus=ready\n\nsmall budget note",
        ),
        retrieved(
            2,
            "scope=project:investments\nstatus=ready\n\nthis budget note is deliberately long enough to overflow the remaining token budget after the first cell is selected",
        ),
    ];
    let pack = ContextPack::from_retrieved_with_options(
        cells,
        20,
        false,
        &ContextPackOptions::default(),
        "RETRIEVE CONTEXT FOR TASK \"budget\" IN BRAIN investment_projects;",
    );

    assert_eq!(pack.cells.len(), 1);
    assert!(pack.truncated);
    assert_eq!(pack.anomalies.len(), 1);
    assert_eq!(
        pack.anomalies[0].code,
        ContextPackAnomalyCode::TokenOverload
    );
    let reason = pack.anomalies[0].why_excluded.as_deref().unwrap();
    assert!(reason.contains("estimated_tokens"));
    assert!(reason.contains("token_budget_tokens"));
}

#[test]
fn context_pack_explain_cell_reports_selected_contract() {
    let pack = ContextPack::from_retrieved_with_options(
        vec![retrieved(
            1,
            "scope=project:investments\nstatus=ready\nsource_trust_q16=60000\n\nsolar budget financing official disclosure",
        )],
        1_000,
        false,
        &ContextPackOptions::default(),
        "RETRIEVE CONTEXT FOR TASK \"solar budget\" IN BRAIN investment_projects;",
    );

    let explain = pack.explain_cell(CellId(1));
    assert_eq!(explain.outcome, ContextCellExplainOutcome::Selected);
    assert_eq!(explain.first_excluding_stage, None);
    assert_eq!(explain.matched_terms, vec!["solar", "budget"]);
    assert_eq!(explain.score, Some(210_824));
    assert_eq!(
        explain.why_selected.as_deref(),
        Some("Selected due to high provenance source trust and relevant query terms")
    );
    assert_eq!(
        explain_snapshot(&explain),
        "cell_id=1 outcome=selected first_excluding_stage=null why_selected=Selected due to high provenance source trust and relevant query terms why_excluded=null matched_terms=solar,budget score_components=base_bm25,source_trust_bonus,source_freshness_bonus,redundancy_penalty"
    );
}

#[test]
fn context_pack_explain_cell_reports_excluded_first_stage() {
    let cells = vec![
        retrieved(
            1,
            "scope=project:investments\nstatus=ready\n\nsolar budget financing disclosure",
        ),
        retrieved(
            2,
            "scope=project:investments\nstatus=ready\n\nsolar budget financing disclosure",
        ),
    ];
    let pack = ContextPack::from_retrieved_with_options(
        cells,
        1_000,
        false,
        &ContextPackOptions {
            reduce_redundancy: true,
            redundancy_threshold_q16: 10,
            ..ContextPackOptions::default()
        },
        "RETRIEVE CONTEXT FOR TASK \"solar budget\" IN BRAIN investment_projects;",
    );

    let explain = pack.explain_cell(CellId(2));
    assert_eq!(explain.outcome, ContextCellExplainOutcome::Excluded);
    assert_eq!(explain.first_excluding_stage.as_deref(), Some("redundancy"));
    assert!(explain
        .why_excluded
        .as_deref()
        .unwrap()
        .contains("reduce_redundancy"));
    assert_eq!(
        explain_snapshot(&explain),
        "cell_id=2 outcome=excluded first_excluding_stage=redundancy why_selected=null why_excluded=excluded because reduce_redundancy is enabled and similarity exceeds the configured threshold matched_terms= score_components="
    );
}

fn explain_snapshot(explain: &ContextCellExplain) -> String {
    let why_selected = explain.why_selected.as_deref().unwrap_or("null");
    let why_excluded = explain.why_excluded.as_deref().unwrap_or("null");
    let first_excluding_stage = explain.first_excluding_stage.as_deref().unwrap_or("null");
    let matched_terms = explain.matched_terms.join(",");
    let score_components = explain
        .score_components
        .iter()
        .map(|component| component.name.as_str())
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "cell_id={} outcome={} first_excluding_stage={} why_selected={} why_excluded={} matched_terms={} score_components={}",
        explain.cell_id.0,
        explain.outcome.as_str(),
        first_excluding_stage,
        why_selected,
        why_excluded,
        matched_terms,
        score_components
    )
}
