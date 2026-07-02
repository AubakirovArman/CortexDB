use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::CellId;
use cortex_engine::{
    scope_id, ContextPack, ContextPackAnomalyCode, ContextPackExportFormat, ContextPackOptions,
    Database, RetrievedCell, SourceFreshnessCategory,
};

#[test]
fn conflict_visibility_is_zero_without_conflicting_values() {
    let pack = pack_from_cells(vec![
        retrieved(
            1,
            "project=Solar\nmetric=budget\nvalue=1200000000\n\nSolar budget one",
        ),
        retrieved(2, "project=Solar\nmetric=risk\nvalue=low\n\nSolar risk one"),
    ]);

    assert_eq!(pack.visible_conflict_count, 0);
    assert_eq!(pack.conflict_visibility_q16, 0);
}

#[test]
fn conflict_visibility_reports_conflicting_project_metric_values() {
    let pack = pack_from_cells(vec![
        retrieved(
            1,
            "project=Solar\nmetric=budget\nvalue=1200000000\n\nSolar budget is 1.2B",
        ),
        retrieved(
            2,
            "project=Solar\nmetric=budget\nvalue=1400000000\n\nSolar budget is 1.4B",
        ),
    ]);

    assert_eq!(pack.cells.len(), 2);
    assert_eq!(pack.visible_conflict_count, 1);
    assert_eq!(pack.conflict_visibility_q16, 32_767);
    assert!(pack
        .anomalies
        .iter()
        .any(|anomaly| anomaly.code == ContextPackAnomalyCode::VisibleConflict));
}

#[test]
fn conflict_visibility_counts_distinct_conflict_groups() {
    let pack = pack_from_cells(vec![
        retrieved(
            1,
            "project=Solar\nmetric=budget\nvalue=1200000000\n\nSolar budget is 1.2B",
        ),
        retrieved(
            2,
            "project=Solar\nmetric=budget\nvalue=1400000000\n\nSolar budget is 1.4B",
        ),
        retrieved(
            3,
            "project=Road\nmetric=length\nvalue=20\n\nRoad length is 20 km",
        ),
        retrieved(
            4,
            "project=Road\nmetric=length\nvalue=25\n\nRoad length is 25 km",
        ),
    ]);

    assert_eq!(pack.visible_conflict_count, 2);
    assert_eq!(pack.conflict_visibility_q16, 43_690);
}

#[test]
fn normalized_equivalent_numeric_formats_have_zero_visible_conflicts() {
    let pack = pack_from_cells(vec![
        retrieved(
            1,
            "project=Solar\nmetric=budget\nvalue=$1.2M\n\nSolar budget is $1.2M.",
        ),
        retrieved(
            2,
            "project=Solar\nmetric=budget\nvalue=1,200,000\ncurrency=USD\n\nSolar budget is 1,200,000 USD.",
        ),
        retrieved(
            3,
            "project=Solar\nmetric=budget\nvalue=1.2 million\n\nSolar budget is 1.2 million.",
        ),
    ]);

    assert_eq!(pack.visible_conflict_count, 0);
    assert_eq!(pack.conflict_visibility_q16, 0);
    assert!(!pack
        .anomalies
        .iter()
        .any(|anomaly| anomaly.code == ContextPackAnomalyCode::VisibleConflict));
}

#[test]
fn context_pack_conflicts_match_verify_numeric_conflicts_on_shared_corpus() {
    let equal_corpus = vec![
        shared_corpus_cell(1, "$1.2M", "Solar budget is $1.2M."),
        shared_corpus_cell(2, "1,200,000 USD", "Solar budget is 1,200,000 USD."),
        shared_corpus_cell(3, "1.2 million", "Solar budget is 1.2 million."),
    ];
    assert_context_pack_and_verify_conflict_counts(&equal_corpus, 0);

    let conflicting_corpus = vec![
        shared_corpus_cell(11, "$1.2M", "Solar budget is $1.2M."),
        shared_corpus_cell(12, "1.4M USD", "Solar budget is 1.4M USD."),
    ];
    assert_context_pack_and_verify_conflict_counts(&conflicting_corpus, 1);
}

#[test]
fn conflict_visibility_is_exported_in_json_prompt_and_markdown() {
    let pack = pack_from_cells(vec![
        retrieved(
            1,
            "project=Solar\nmetric=budget\nvalue=1200000000\n\nSolar budget is 1.2B",
        ),
        retrieved(
            2,
            "project=Solar\nmetric=budget\nvalue=1400000000\n\nSolar budget is 1.4B",
        ),
    ]);

    let json = pack.export(ContextPackExportFormat::Json);
    let prompt = pack.export(ContextPackExportFormat::Prompt);
    let markdown = pack.export(ContextPackExportFormat::Markdown);

    assert!(json.contains(r#""conflict_visibility_q16":32767"#));
    assert!(json.contains(r#""visible_conflict_count":1"#));
    assert!(prompt
        .contains("Conflict visibility: conflict_visibility_q16=32767 visible_conflict_count=1"));
    assert!(markdown.contains("- conflict_visibility_q16: `32767`"));
    assert!(markdown.contains("- visible_conflict_count: `1`"));
}

#[test]
fn conflicting_values_explain_source_freshness_for_current_source() {
    let pack = pack_from_cells(vec![
        retrieved(
            1,
            "created_unix_seconds=100\nsource_trust_class=internal\nproject=Solar\nmetric=budget\nvalue=1200000000\n\nSolar budget is 1.2B",
        ),
        retrieved(
            2,
            "created_unix_seconds=200\nsource_trust_class=official\nproject=Solar\nmetric=budget\nvalue=1400000000\n\nSolar budget is 1.4B",
        ),
    ]);

    assert_eq!(pack.visible_conflict_count, 1);
    let stale = pack.cells[0].explain.as_ref().unwrap();
    let current = pack.cells[1].explain.as_ref().unwrap();

    assert_eq!(
        stale.source_freshness_category,
        SourceFreshnessCategory::Stale
    );
    assert_eq!(
        current.source_freshness_category,
        SourceFreshnessCategory::Current
    );
    assert!(current.source_freshness_bonus > stale.source_freshness_bonus);
    assert!(current
        .score_components
        .iter()
        .any(|component| component.name == "source_freshness_bonus"
            && component.reason.contains("current source freshness")));

    let json = pack.export(ContextPackExportFormat::Json);
    assert!(json.contains(r#""source_freshness_category":"current""#));
    let prompt = pack.export(ContextPackExportFormat::Prompt);
    assert!(prompt.contains("source_freshness=current"));
    let markdown = pack.export(ContextPackExportFormat::Markdown);
    assert!(markdown.contains("- source_freshness: `current`"));
}

fn pack_from_cells(cells: Vec<RetrievedCell>) -> ContextPack {
    ContextPack::from_retrieved_with_options(
        cells,
        2_000,
        false,
        &ContextPackOptions::default(),
        "",
    )
}

fn retrieved(cell_id: u64, payload: &str) -> RetrievedCell {
    RetrievedCell::from_payload(CellId(cell_id), payload.as_bytes().to_vec())
}

fn shared_corpus_cell(cell_id: u64, value: &str, body: &str) -> (CellId, Vec<u8>) {
    (
        CellId(cell_id),
        format!(
            "scope=project:investments\nstatus=verified\ntype=fact\nsource=ledger\nproject=Solar\nmetric=budget\nvalue={value}\n\n{body}"
        )
        .into_bytes(),
    )
}

fn assert_context_pack_and_verify_conflict_counts(cells: &[(CellId, Vec<u8>)], expected: u32) {
    let pack = pack_from_cells(
        cells
            .iter()
            .map(|(cell_id, payload)| RetrievedCell::from_payload(*cell_id, payload.clone()))
            .collect(),
    );
    assert_eq!(pack.visible_conflict_count, expected);

    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    for (cell_id, payload) in cells {
        db.put_cell(*cell_id, payload.clone()).unwrap();
    }

    let report = db
        .verify_fact_aql(
            r#"VERIFY FACT "Solar budget is $1.2M" IN BRAIN investment_projects;"#,
            &verify_view(),
        )
        .unwrap();
    assert_eq!(
        pack.visible_conflict_count,
        report.numeric_conflicts.len() as u32
    );
}

fn verify_view() -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: Some("context-pack-conflict-visibility".to_owned()),
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope_id("project:investments")]),
        writable_scopes: BTreeSet::new(),
        allowed_modes: BTreeSet::from([RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::from([MemoryType::Decision]),
        max_context_budget_tokens: 1_000,
        default_context_budget_tokens: 400,
        max_candidate_limit: 100,
        default_candidate_limit: 20,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: Some(3_600),
        allow_remember: false,
        allow_verify_fact: true,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: None,
    }
}
