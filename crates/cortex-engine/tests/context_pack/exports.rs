use super::common::prelude::*;
use super::common::{query, retrieved, view};

#[test]
fn context_pack_exports_stable_prompt_and_markdown() {
    let cells = vec![retrieved(
        7,
        "scope=project:investments\nstatus=ready\nsource=doc-a\nsource_id=doc-a\ndocument_id=doc-1\npage=3\n\nSolar budget evidence.",
    )];
    let pack = ContextPack::from_retrieved_with_options(
        cells,
        1_000,
        true,
        &ContextPackOptions::default(),
        "RETRIEVE CONTEXT FOR TASK \"budget\" IN BRAIN investment_projects;",
    );

    let prompt = pack.export(ContextPackExportFormat::Prompt);
    assert!(prompt.contains("CortexDB ContextPack v1"));
    assert!(prompt.contains("Use only the context cells below."));
    assert!(prompt.contains("[1] cell_id=7"));
    assert!(prompt.contains("source_ref=source_id=doc-a;document_id=doc-1;page=3"));
    assert!(prompt.contains("Solar budget evidence."));

    let markdown = pack.export(ContextPackExportFormat::Markdown);
    assert!(markdown.contains("# CortexDB ContextPack"));
    assert!(markdown.contains("### Cell 1"));
    assert!(markdown.contains("- cell_id: `7`"));
    assert!(markdown.contains("```text"));
    assert!(markdown.contains("Solar budget evidence."));
}

#[test]
fn context_pack_markdown_export_preserves_code_fences() {
    let cells = vec![retrieved(
        8,
        "scope=project:investments\nstatus=ready\nsource=doc-a\n\npayload with ``` fenced text",
    )];
    let pack = ContextPack::from_retrieved(cells, 1_000, false);

    let markdown = pack.export(ContextPackExportFormat::Markdown);
    assert!(markdown.contains("````text"));
    assert!(markdown.contains("payload with ``` fenced text"));
}

#[test]
fn context_pack_orders_cells_by_feedback_score() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nsource=doc-a\nalpha budget".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\nsource=doc-b\nbeta budget".to_vec(),
    )
    .unwrap();
    db.record_context_feedback(
        AgentId(1),
        ContextFeedback {
            source_cell_id: CellId(2),
            useful: true,
            note: None,
        },
    )
    .unwrap();

    let pack = db
        .context_pack_from_aql(query(), &view(false), ContextPackOptions::default())
        .unwrap();
    assert_eq!(pack.cells[0].cell_id, CellId(2));
    let explain = pack.cells[0].explain.as_ref().unwrap();
    assert!(explain
        .score_components
        .iter()
        .any(|component| { component.name == "feedback_bonus" && component.contribution > 0 }));
}
