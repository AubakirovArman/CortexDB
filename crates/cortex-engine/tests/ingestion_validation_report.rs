use cortex_core::{CellId, CommitSeq};
use cortex_engine::{
    Database, IngestedCell, IngestionValidationReport, TextChunkPolicy, TextIngestOptions,
};

#[test]
fn ingestion_validation_report_captures_text_chunk_source_refs() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    let cells = db
        .ingest_text_chunks_with_policy(
            CellId(10),
            "Alpha budget\n\nBeta timeline",
            TextIngestOptions {
                scope: "project:investments".to_owned(),
                source: "memo.md".to_owned(),
            },
            TextChunkPolicy {
                max_chars: 20,
                overlap_chars: 0,
                min_chars: 1,
            },
        )
        .unwrap();

    let report = db.ingestion_validation_report(&cells);

    assert_eq!(report.cells_seen, 2);
    assert!(report.warnings.is_empty());
    assert_eq!(report.source_refs.len(), 2);
    assert_eq!(report.source_refs[0].cell_id, 10);
    assert!(report.source_refs[0].has_source_ref);
    assert_eq!(report.source_refs[0].source_id.as_deref(), Some("memo.md"));
    assert_eq!(
        report.source_refs[0].chunk_id.as_deref(),
        Some("memo.md#chunk-0001")
    );
    assert_eq!(
        report.source_refs[0].cell_range.as_deref(),
        Some("memo.md#chunk-0001")
    );
    assert_eq!(
        report.source_refs[0].document_id.as_deref(),
        Some("memo.md")
    );
    assert_eq!(report.source_refs[0].confidence_q16, Some(32768));
}

#[test]
fn ingestion_validation_report_captures_structured_json_and_csv_refs() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    let json_cells = db
        .ingest_json(
            CellId(20),
            r#"{"project":{"budget":"12000"}}"#,
            cortex_engine::JsonIngestOptions {
                scope: "project:investments".to_owned(),
                source: "api.json".to_owned(),
            },
        )
        .unwrap();
    let csv_cells = db
        .ingest_csv(
            CellId(30),
            "project,budget\nABC,12000",
            cortex_engine::CsvIngestOptions {
                scope: "project:investments".to_owned(),
                source: "budget.csv".to_owned(),
            },
        )
        .unwrap();

    let json_report = db.ingestion_validation_report(&json_cells);
    assert_eq!(
        json_report.source_refs[0].json_path.as_deref(),
        Some("project.budget")
    );
    assert_eq!(
        json_report.source_refs[0].document_id.as_deref(),
        Some("api.json")
    );

    let csv_report = db.ingestion_validation_report(&csv_cells);
    assert_eq!(csv_report.source_refs[0].row, Some(2));
    assert_eq!(
        csv_report.source_refs[0].cell_range.as_deref(),
        Some("row-2")
    );
    assert_eq!(
        csv_report.source_refs[0].document_id.as_deref(),
        Some("budget.csv")
    );
}

#[test]
fn ingestion_validation_report_warns_when_cell_is_missing() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).unwrap();
    let cells = vec![IngestedCell {
        cell_id: CellId(999),
        commit_seq: CommitSeq(1),
        chunk_id: Some("missing#chunk-0001".to_owned()),
    }];

    let report = db.ingestion_validation_report(&cells);

    assert_eq!(report.cells_seen, 1);
    assert_eq!(report.warnings.len(), 1);
    assert_eq!(report.warnings[0].code, "missing_payload");
    assert!(report.source_refs.is_empty());
}

#[test]
fn ingestion_validation_report_records_skipped_items() {
    let mut report = IngestionValidationReport::default();

    report.record_skipped("no_cells_emitted", Some("ingest_text".to_owned()));

    assert_eq!(report.skipped_items.len(), 1);
    assert_eq!(report.skipped_items[0].reason, "no_cells_emitted");
    assert_eq!(
        report.skipped_items[0].input_ref.as_deref(),
        Some("ingest_text")
    );
}
