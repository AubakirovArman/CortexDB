use cortex_core::CellId;
use cortex_engine::{extract_pdf_text, scope_id};
use cortex_engine::{CsvIngestOptions, Database, JsonIngestOptions, PdfIngestOptions};
use cortex_engine::{IngestionJobStatus, IngestionProgressTracker, TextIngestOptions};

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
fn empty_text_chunk_ingestion_returns_zero_cells() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();

    let cells = db
        .ingest_text_chunks(
            CellId(1),
            " \n\n\t ",
            TextIngestOptions {
                scope: "project:investments".to_owned(),
                source: "empty.md".to_owned(),
            },
        )
        .unwrap();

    assert!(cells.is_empty());
    assert!(db.get_latest_cell(CellId(1)).is_none());
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

#[test]
fn native_pdf_ingestion_extracts_simple_text_objects() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    let pdf = br#"%PDF-1.4
1 0 obj
<< /Length 64 >>
stream
BT /F1 12 Tf 72 720 Td (ABC budget) Tj <203132303030> Tj ET
endstream
endobj
%%EOF"#;

    let extracted = extract_pdf_text(pdf).unwrap();
    assert_eq!(extracted.literal_strings, 1);
    assert_eq!(extracted.hex_strings, 1);
    assert!(extracted.text.contains("ABC budget"));
    assert!(extracted.text.contains("12000"));

    db.ingest_pdf_bytes(
        CellId(40),
        pdf,
        PdfIngestOptions {
            scope: "project:investments".to_owned(),
            source: "native.pdf".to_owned(),
            page: None,
        },
    )
    .unwrap();
    let payload = db.get_latest_cell(CellId(40)).unwrap();
    assert!(String::from_utf8_lossy(&payload).contains("ABC budget"));
}

#[test]
fn native_pdf_ingestion_extracts_flate_decode_streams() {
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use std::io::Write;

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(b"BT (Compressed budget 12000) Tj ET")
        .unwrap();
    let compressed = encoder.finish().unwrap();
    let mut pdf = b"%PDF-1.4\n1 0 obj\n<< /Filter /FlateDecode >>\nstream\n".to_vec();
    pdf.extend_from_slice(&compressed);
    pdf.extend_from_slice(b"\nendstream\nendobj\n%%EOF");

    let extracted = extract_pdf_text(&pdf).unwrap();

    assert!(extracted.text.contains("Compressed budget 12000"));
}

#[test]
fn csv_ingestion_reports_progress() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    let mut tracker = IngestionProgressTracker::default();

    let (job_id, cells) = db
        .ingest_csv_with_progress(
            CellId(50),
            "project,budget\nABC,12000\nXYZ,9000",
            CsvIngestOptions {
                scope: "project:investments".to_owned(),
                source: "budget.csv".to_owned(),
            },
            &mut tracker,
            "budget import",
        )
        .unwrap();

    let progress = tracker.get(job_id).unwrap();
    assert_eq!(cells.len(), 2);
    assert_eq!(progress.status, IngestionJobStatus::Completed);
    assert_eq!(progress.total_items, Some(2));
    assert_eq!(progress.completed_items, 2);
    assert_eq!(progress.last_cell_id, Some(CellId(51)));
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
