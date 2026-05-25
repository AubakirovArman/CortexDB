use cortex_core::CellId;
use cortex_engine::Database;

#[test]
fn storage_stats_exposes_live_wal_writer_metrics() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();

    db.put_cell(CellId(1), b"one".to_vec()).unwrap();
    db.put_cell(CellId(2), b"two".to_vec()).unwrap();

    let stats = db.storage_stats().unwrap();
    assert_eq!(stats.wal_writer.records_written, 2);
    assert!(stats.wal_writer.bytes_written > 0);
    assert_eq!(stats.wal_writer.fsync_count, 2);
    assert_eq!(stats.wal_writer.batches_committed, 2);
}
