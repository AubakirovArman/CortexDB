use super::common::prelude::*;
use super::common::{query, view};

#[test]
fn context_pack_from_aql_respects_budget() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nsource=doc-a\nsmall".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\nsource=doc-b\nthis payload is deliberately much larger than the first selected cell".to_vec(),
    )
    .unwrap();

    let pack = db
        .context_pack_from_aql(
            query(),
            &view(false),
            ContextPackOptions {
                token_budget_tokens: 32,
                require_citations: false,
                ..ContextPackOptions::default()
            },
        )
        .unwrap();

    assert_eq!(pack.cells.len(), 1);
    assert_eq!(pack.cells[0].cell_id, CellId(1));
    assert!(pack.truncated);
    assert!(pack.estimated_tokens <= pack.token_budget_tokens);
    assert_eq!(
        pack.anomalies[0].code,
        ContextPackAnomalyCode::TokenOverload
    );
    assert!(pack.anomalies[0]
        .why_excluded
        .as_deref()
        .unwrap_or_default()
        .contains("token_budget_tokens"));
}

#[test]
fn context_pack_reports_missing_citations_when_required() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nalpha budget".to_vec(),
    )
    .unwrap();

    let pack = db
        .context_pack_from_aql(query(), &view(true), ContextPackOptions::default())
        .unwrap();

    assert!(pack.citations_required);
    assert_eq!(pack.anomalies.len(), 1);
    assert_eq!(
        pack.anomalies[0].code,
        ContextPackAnomalyCode::MissingCitation
    );
    assert_eq!(pack.anomalies[0].cell_id, Some(CellId(1)));
    assert_eq!(pack.anomalies[0].why_excluded, None);
}

#[test]
fn context_pack_uses_source_line_as_citation() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nsource=annual-report\nalpha budget".to_vec(),
    )
    .unwrap();

    let pack = db
        .context_pack_from_aql(query(), &view(true), ContextPackOptions::default())
        .unwrap();

    assert!(pack.anomalies.is_empty());
    assert_eq!(pack.cells[0].citation.as_deref(), Some("annual-report"));
}

#[test]
fn context_pack_accepts_source_ref_as_required_citation() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nsource_id=ifc:project-1\ndocument_id=doc-1\nconfidence_q16=60000\nalpha budget".to_vec(),
    )
    .unwrap();

    let pack = db
        .context_pack_from_aql(query(), &view(true), ContextPackOptions::default())
        .unwrap();

    assert!(pack.anomalies.is_empty());
    assert_eq!(pack.cells[0].citation.as_deref(), Some("ifc:project-1"));
}

#[test]
fn context_pack_filters_low_confidence_source_refs_from_aql_requirements() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nsource_id=ifc:high\nconfidence_q16=60000\nalpha budget".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\nsource_id=ifc:low\nconfidence_q16=30000\nalpha budget".to_vec(),
    )
    .unwrap();

    let pack = db
        .context_pack_from_aql(
            r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
WHERE space = project:investments AND status = "ready" LIMIT 10 CANDIDATES
REQUIRE citations, confidence >= 0.80;"#,
            &view(false),
            ContextPackOptions::default(),
        )
        .unwrap();

    assert_eq!(
        pack.cells
            .iter()
            .map(|cell| cell.cell_id)
            .collect::<Vec<_>>(),
        vec![CellId(1)]
    );
    assert!(pack.citations_required);
    assert_eq!(pack.cells[0].citation.as_deref(), Some("ifc:high"));
}
