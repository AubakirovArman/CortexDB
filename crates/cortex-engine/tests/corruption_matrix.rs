use cortex_core::CellId;
use cortex_engine::Database;

#[test]
fn corrupt_live_segment_blocks_open() {
    let dir = tempfile::tempdir().unwrap();
    write_checkpoint(dir.path());
    corrupt_last_byte(&dir.path().join("segments").join("segment-1.acs"));

    assert!(Database::open(dir.path()).is_err());
}

#[test]
fn corrupt_manifest_blocks_open() {
    let dir = tempfile::tempdir().unwrap();
    write_checkpoint(dir.path());
    corrupt_last_byte(&dir.path().join("manifest.acm"));

    assert!(Database::open(dir.path()).is_err());
}

#[test]
fn corrupt_bitmap_index_fails_validation_report() {
    let dir = tempfile::tempdir().unwrap();
    write_checkpoint(dir.path());
    corrupt_last_byte(&dir.path().join("segments").join("segment-1.acb"));

    let db = Database::open(dir.path()).unwrap();
    let report = db.validate_storage_report();
    assert!(!report.errors.is_empty());
    assert!(report
        .errors
        .iter()
        .any(|error| error.contains("bitmap index 1")));
}

#[test]
fn corrupt_lexical_index_fails_validation_report() {
    let dir = tempfile::tempdir().unwrap();
    write_checkpoint(dir.path());
    corrupt_last_byte(&dir.path().join("segments").join("segment-1.aci"));

    let db = Database::open(dir.path()).unwrap();
    let report = db.validate_storage_report();
    assert!(!report.errors.is_empty());
    assert!(report
        .errors
        .iter()
        .any(|error| error.contains("lexical index 1")));
}

#[test]
fn corrupt_vector_index_fails_validation_report() {
    let dir = tempfile::tempdir().unwrap();
    write_checkpoint(dir.path());
    corrupt_last_byte(&dir.path().join("segments").join("segment-1.acv"));

    let db = Database::open(dir.path()).unwrap();
    let report = db.validate_storage_report();
    assert!(!report.errors.is_empty());
    assert!(report
        .errors
        .iter()
        .any(|error| error.contains("vector index 1")));
}

fn write_checkpoint(root: &std::path::Path) {
    let mut db = Database::open(root).unwrap();
    db.put_cell(CellId(1), b"scope=default\nstatus=ready\none".to_vec())
        .unwrap();
    db.checkpoint().unwrap();
    drop(db);
}

fn corrupt_last_byte(path: &std::path::Path) {
    let mut bytes = std::fs::read(path).unwrap();
    *bytes.last_mut().unwrap() ^= 0xff;
    std::fs::write(path, bytes).unwrap();
}
