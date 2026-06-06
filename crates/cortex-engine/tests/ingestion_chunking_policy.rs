use cortex_core::CellId;
use cortex_engine::{
    split_text_chunks, CellMetadata, CsvIngestOptions, Database, JsonChunkPolicy,
    JsonIngestOptions, TableChunkPolicy, TextChunkPolicy, TextOverlapPolicy,
};

#[test]
fn text_chunk_ids_and_overlap_are_deterministic() {
    let policy = TextChunkPolicy {
        max_chars: 10,
        overlap_chars: 3,
        min_chars: 1,
    };
    let text = "abcdefghijklmnopqrstuvwxyz";

    let chunks = split_text_chunks("reports/Alpha Plan.md", text, policy).unwrap();
    let repeated = split_text_chunks("reports/Alpha Plan.md", text, policy).unwrap();

    assert_eq!(
        policy.overlap_policy(),
        TextOverlapPolicy::FixedChars { chars: 3 }
    );
    assert_eq!(chunks, repeated);
    assert_eq!(chunks[0].chunk_id, "reports-Alpha-Plan.md#chunk-0001");
    assert_eq!(chunks[1].chunk_id, "reports-Alpha-Plan.md#chunk-0002");
    assert_eq!(chunks[0].text, "abcdefghij");
    assert_eq!(chunks[1].text, "hijklmnopq");
}

#[test]
fn json_ingestion_uses_sorted_leaf_paths_as_policy() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();

    let cells = db
        .ingest_json(
            CellId(100),
            r#"{"z":1,"a":{"b":2},"arr":[{"x":3}]}"#,
            JsonIngestOptions {
                scope: "project:investments".to_owned(),
                source: "api/projects.json".to_owned(),
            },
        )
        .unwrap();

    let policy = JsonChunkPolicy::default();
    assert_eq!(policy.path_separator, '.');
    assert!(policy.sort_paths);
    assert_eq!(cells.len(), 3);
    assert_source_ref_path(&db, CellId(100), "a.b");
    assert_source_ref_path(&db, CellId(101), "arr.0.x");
    assert_source_ref_path(&db, CellId(102), "z");
}

#[test]
fn table_policy_uses_one_based_source_rows_and_cell_ranges() {
    let policy = TableChunkPolicy::default();
    assert_eq!(policy.source_row_number(0).unwrap(), 2);
    assert_eq!(policy.source_row_number(2).unwrap(), 4);
    assert_eq!(policy.cell_range(4), "row-4");

    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    let cells = db
        .ingest_csv(
            CellId(200),
            "project,budget\nABC,12000\nXYZ,9000",
            CsvIngestOptions {
                scope: "project:investments".to_owned(),
                source: "budget.csv".to_owned(),
            },
        )
        .unwrap();

    assert_eq!(cells.len(), 2);
    assert_source_ref_row(&db, CellId(200), 2, "row-2");
    assert_source_ref_row(&db, CellId(201), 3, "row-3");
}

#[test]
fn invalid_chunking_policies_fail_closed() {
    assert!(JsonChunkPolicy {
        path_separator: '\n',
        sort_paths: true,
    }
    .validate()
    .is_err());
    assert!(TableChunkPolicy {
        first_data_row: 0,
        cell_range_prefix: "row-",
    }
    .validate()
    .is_err());
    assert!(TableChunkPolicy {
        first_data_row: 2,
        cell_range_prefix: "bad\n",
    }
    .validate()
    .is_err());
}

fn assert_source_ref_path(db: &Database, cell_id: CellId, path: &str) {
    let payload = db.get_latest_cell(cell_id).unwrap();
    let metadata = CellMetadata::from_payload(&payload);
    let source_ref = metadata.source_ref.unwrap();
    assert_eq!(source_ref.json_path.as_deref(), Some(path));
}

fn assert_source_ref_row(db: &Database, cell_id: CellId, row: u32, cell_range: &str) {
    let payload = db.get_latest_cell(cell_id).unwrap();
    let metadata = CellMetadata::from_payload(&payload);
    let source_ref = metadata.source_ref.unwrap();
    assert_eq!(source_ref.row, Some(row));
    assert_eq!(source_ref.cell_range.as_deref(), Some(cell_range));
}
