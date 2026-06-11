use cortex_core::CellId;
use cortex_engine::{
    ContextPack, ContextPackAnomalyCode, ContextPackExportFormat, ContextPackOptions, RetrievedCell,
};

#[test]
fn answerability_is_full_when_selected_cells_cover_query_terms() {
    let pack = pack_for_query(
        vec![retrieved(
            1,
            "scope=project:investments\nstatus=ready\nalpha budget",
        )],
        r#"RETRIEVE CONTEXT FOR TASK "alpha budget" IN BRAIN investment_projects;"#,
    );

    assert_eq!(pack.answerability_q16, u16::MAX);
    assert!(pack
        .anomalies
        .iter()
        .all(|anomaly| anomaly.code != ContextPackAnomalyCode::InsufficientContext));
}

#[test]
fn answerability_reports_missing_query_terms() {
    let pack = pack_for_query(
        vec![retrieved(
            1,
            "scope=project:investments\nstatus=ready\nalpha evidence",
        )],
        r#"RETRIEVE CONTEXT FOR TASK "alpha budget" IN BRAIN investment_projects;"#,
    );

    assert!(pack.answerability_q16 < u16::MAX);
    let anomaly = pack
        .anomalies
        .iter()
        .find(|anomaly| anomaly.code == ContextPackAnomalyCode::InsufficientContext)
        .expect("missing terms should create insufficient_context anomaly");
    assert_eq!(anomaly.cell_id, None);
    assert!(anomaly.message.contains("answerability score"));
    assert!(anomaly
        .why_excluded
        .as_deref()
        .unwrap_or_default()
        .contains("missing_terms=[budget]"));
}

#[test]
fn answerability_allows_partial_context_above_default_threshold() {
    let pack = pack_for_query(
        vec![retrieved(
            1,
            "scope=project:investments\nstatus=ready\nalpha beta gamma evidence",
        )],
        r#"RETRIEVE CONTEXT FOR TASK "alpha beta gamma delta" IN BRAIN investment_projects;"#,
    );

    assert!(pack.answerability_q16 > cortex_engine::context::DEFAULT_ANSWERABILITY_THRESHOLD_Q16);
    assert!(pack
        .anomalies
        .iter()
        .all(|anomaly| anomaly.code != ContextPackAnomalyCode::InsufficientContext));
}

#[test]
fn answerability_reports_empty_context() {
    let pack = pack_for_query(
        Vec::new(),
        r#"RETRIEVE CONTEXT FOR TASK "alpha budget" IN BRAIN investment_projects;"#,
    );

    assert_eq!(pack.answerability_q16, 0);
    assert_eq!(pack.anomalies.len(), 1);
    assert_eq!(
        pack.anomalies[0].code,
        ContextPackAnomalyCode::InsufficientContext
    );
}

#[test]
fn answerability_is_exported_in_json_prompt_and_markdown() {
    let pack = pack_for_query(
        vec![retrieved(
            1,
            "scope=project:investments\nstatus=ready\nalpha",
        )],
        r#"RETRIEVE CONTEXT FOR TASK "alpha budget" IN BRAIN investment_projects;"#,
    );

    let json = pack.export(ContextPackExportFormat::Json);
    let prompt = pack.export(ContextPackExportFormat::Prompt);
    let markdown = pack.export(ContextPackExportFormat::Markdown);

    assert!(json.contains(r#""answerability_q16":"#));
    assert!(json.contains(r#""code":"insufficient_context""#));
    assert!(prompt.contains("Answerability: answerability_q16="));
    assert!(prompt.contains("code=insufficient_context"));
    assert!(markdown.contains("- answerability_q16: `"));
    assert!(markdown.contains("`insufficient_context`"));
}

fn pack_for_query(cells: Vec<RetrievedCell>, query: &str) -> ContextPack {
    ContextPack::from_retrieved_with_options(
        cells,
        1_000,
        false,
        &ContextPackOptions::default(),
        query,
    )
}

fn retrieved(cell_id: u64, payload: &str) -> RetrievedCell {
    RetrievedCell {
        cell_id: CellId(cell_id),
        payload: payload.as_bytes().to_vec(),
    }
}
