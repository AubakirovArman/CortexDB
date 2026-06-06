use cortex_core::CellId;
use cortex_engine::{
    scope_id, split_text_chunks, stable_ingestion_hash_hex, CellMetadata, IngestionUpdatePolicy,
    TextChunkPolicy,
};
use cortex_engine::{CsvIngestOptions, Database, JsonIngestOptions};
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
fn text_chunk_policy_produces_stable_ids_and_long_paragraph_chunks() {
    let text = "A".repeat(1_200);
    let policy = TextChunkPolicy {
        max_chars: 500,
        overlap_chars: 50,
        min_chars: 1,
    };

    let chunks = split_text_chunks("Report 1.pdf", &text, policy).unwrap();
    let repeated = split_text_chunks("Report 1.pdf", &text, policy).unwrap();

    assert_eq!(chunks, repeated);
    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks[0].chunk_id, "Report-1.pdf#chunk-0001");
    assert_eq!(chunks[1].chunk_id, "Report-1.pdf#chunk-0002");
    assert!(chunks.iter().all(|chunk| chunk.text.chars().count() <= 500));
}

#[test]
fn text_chunk_policy_rejects_overlap_that_cannot_advance() {
    let policy = TextChunkPolicy {
        max_chars: 100,
        overlap_chars: 100,
        min_chars: 1,
    };

    assert!(split_text_chunks("doc", "body", policy).is_err());
}

#[test]
fn text_ingestion_writes_chunk_id_as_source_ref_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    let cells = db
        .ingest_text_chunks_with_policy(
            CellId(100),
            "First paragraph about budget.\n\nSecond paragraph about timeline.",
            TextIngestOptions {
                scope: "project:investments".to_owned(),
                source: "source/report.md".to_owned(),
            },
            TextChunkPolicy {
                max_chars: 40,
                overlap_chars: 0,
                min_chars: 1,
            },
        )
        .unwrap();

    assert_eq!(cells.len(), 2);
    assert_eq!(
        cells[0].chunk_id.as_deref(),
        Some("source-report.md#chunk-0001")
    );
    let payload = db.get_latest_cell(cells[0].cell_id).unwrap();
    let payload_text = String::from_utf8_lossy(&payload);
    assert!(payload_text.contains("source_id=source/report.md"));
    assert!(payload_text.contains("document_id=source/report.md"));
    assert!(payload_text.contains("chunk_id=source-report.md#chunk-0001"));

    let metadata = CellMetadata::from_payload(&payload);
    let source_ref = metadata.source_ref.unwrap();
    assert_eq!(source_ref.source_id, "source/report.md");
    assert_eq!(source_ref.document_id.as_deref(), Some("source/report.md"));
    assert_eq!(
        source_ref.cell_range.as_deref(),
        Some("source-report.md#chunk-0001")
    );
}

#[test]
fn text_ingestion_writes_content_and_source_hash_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    let cells = db
        .ingest_text_chunks(
            CellId(300),
            "Alpha budget duplicate guard",
            TextIngestOptions {
                scope: "project:investments".to_owned(),
                source: "dedup.md".to_owned(),
            },
        )
        .unwrap();

    let payload = db.get_latest_cell(cells[0].cell_id).unwrap();
    let metadata = CellMetadata::decode_payload(&payload).unwrap();

    assert_eq!(
        metadata.content_hash.as_deref(),
        Some(stable_ingestion_hash_hex(b"Alpha budget duplicate guard").as_str())
    );
    assert_eq!(
        metadata.source_hash.as_deref(),
        Some(stable_ingestion_hash_hex(b"dedup.md").as_str())
    );
}

#[test]
fn text_ingestion_skip_existing_policy_skips_duplicate_chunks() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    let options = TextIngestOptions {
        scope: "project:investments".to_owned(),
        source: "same-source.md".to_owned(),
    };
    let policy = TextChunkPolicy {
        max_chars: 100,
        overlap_chars: 0,
        min_chars: 1,
    };

    let first = db
        .ingest_text_chunks_with_update_policy(
            CellId(400),
            "same body",
            options.clone(),
            policy,
            IngestionUpdatePolicy::SkipExisting,
        )
        .unwrap();
    let skipped = db
        .ingest_text_chunks_with_update_policy(
            CellId(500),
            "same body",
            options.clone(),
            policy,
            IngestionUpdatePolicy::SkipExisting,
        )
        .unwrap();
    let inserted_again = db
        .ingest_text_chunks_with_update_policy(
            CellId(600),
            "same body",
            options,
            policy,
            IngestionUpdatePolicy::AlwaysInsert,
        )
        .unwrap();

    assert_eq!(first.len(), 1);
    assert!(skipped.is_empty());
    assert_eq!(inserted_again.len(), 1);
    assert!(db.get_latest_cell(CellId(500)).is_none());
    assert!(db.get_latest_cell(CellId(600)).is_some());
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
    let payload_text = String::from_utf8_lossy(&payload);
    assert!(payload_text.contains("budget: 12000"));
    assert!(payload_text.contains("source_id=api.json"));
    assert!(payload_text.contains("document_id=api.json"));
    assert!(payload_text.contains("json_path=budget"));
    let source_ref = CellMetadata::from_payload(&payload).source_ref.unwrap();
    assert_eq!(source_ref.document_id.as_deref(), Some("api.json"));
    assert_eq!(source_ref.json_path.as_deref(), Some("budget"));
    let metadata = CellMetadata::decode_payload(&payload).unwrap();
    assert!(metadata.content_hash.is_some());
    assert_eq!(
        metadata.source_hash.as_deref(),
        Some(stable_ingestion_hash_hex(b"api.json").as_str())
    );
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
    let payload = db.get_latest_cell(CellId(20)).unwrap();
    let payload_text = String::from_utf8_lossy(&payload);
    assert!(payload_text.contains("source_id=budget.csv"));
    assert!(payload_text.contains("document_id=budget.csv"));
    assert!(payload_text.contains("row=2"));
    assert!(payload_text.contains("cell_range=row-2"));
    let source_ref = CellMetadata::from_payload(&payload).source_ref.unwrap();
    assert_eq!(source_ref.row, Some(2));
    assert_eq!(source_ref.cell_range.as_deref(), Some("row-2"));
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
