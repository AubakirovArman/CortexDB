use cortex_core::CellId;
use cortex_engine::{
    scope_id, CsvIngestOptions, Database, JsonIngestOptions, PdfIngestOptions, TextIngestOptions,
};

#[test]
fn text_ingestion_writes_document_cell() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();

    let result = db
        .ingest_text(
            CellId(1),
            "ABC project budget",
            TextIngestOptions {
                scope: "project:investments".to_owned(),
                source: "note.md".to_owned(),
            },
        )
        .unwrap();

    let payload = db.get_latest_cell(result.cell_id).unwrap();
    let text = String::from_utf8_lossy(&payload);
    assert!(text.contains("type=document_block"));
    assert!(text.contains("source=note.md"));
    assert!(text.contains("ABC project budget"));
}

#[test]
fn json_ingestion_writes_fact_cells() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();

    let cells = db
        .ingest_json(
            CellId(10),
            r#"{"budget":"12000","status":"approved"}"#,
            JsonIngestOptions {
                scope: "project:investments".to_owned(),
                source: "api.json".to_owned(),
            },
        )
        .unwrap();

    assert_eq!(cells.len(), 2);
    let payload = db.get_latest_cell(CellId(10)).unwrap();
    assert!(String::from_utf8_lossy(&payload).contains("budget: 12000"));
}

#[test]
fn csv_ingestion_writes_one_cell_per_row() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();

    let cells = db
        .ingest_csv(
            CellId(20),
            "project,budget\nABC,12000\nXYZ,9000",
            CsvIngestOptions {
                scope: "project:investments".to_owned(),
                source: "budget.csv".to_owned(),
            },
        )
        .unwrap();

    assert_eq!(cells.len(), 2);
    let found = db
        .search_keyword("XYZ", &crate_view(), cortex_engine::SearchLimit(10))
        .unwrap();
    assert_eq!(found[0].cell_id, CellId(21));
}

#[test]
fn pdf_text_ingestion_marks_external_pdf_source() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();

    let result = db
        .ingest_pdf_text(
            CellId(30),
            "Extracted table text",
            PdfIngestOptions {
                scope: "project:investments".to_owned(),
                source: "report.pdf".to_owned(),
                page: Some(7),
            },
        )
        .unwrap();

    let payload = db.get_latest_cell(result.cell_id).unwrap();
    let text = String::from_utf8_lossy(&payload);
    assert!(text.contains("source=report.pdf"));
    assert!(text.contains("source_format=pdf"));
    assert!(text.contains("page=7"));
}

fn crate_view() -> cortex_aql::AgentView {
    use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
    use std::collections::BTreeSet;

    AgentView {
        agent_id: AgentId(1),
        label: None,
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
        allow_verify_fact: false,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: None,
    }
}
