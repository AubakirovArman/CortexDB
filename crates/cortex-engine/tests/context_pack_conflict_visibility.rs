use cortex_core::CellId;
use cortex_engine::{
    ContextPack, ContextPackExportFormat, ContextPackOptions, RetrievedCell,
    SourceFreshnessCategory,
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
    assert_eq!(pack.conflict_visibility_q16, u16::MAX);
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
    assert_eq!(pack.conflict_visibility_q16, u16::MAX);
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

    assert!(json.contains(r#""conflict_visibility_q16":65535"#));
    assert!(json.contains(r#""visible_conflict_count":1"#));
    assert!(prompt
        .contains("Conflict visibility: conflict_visibility_q16=65535 visible_conflict_count=1"));
    assert!(markdown.contains("- conflict_visibility_q16: `65535`"));
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
    RetrievedCell {
        cell_id: CellId(cell_id),
        payload: payload.as_bytes().to_vec(),
    }
}
