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

#[test]
fn storage_stats_exposes_memory_accounting_estimates() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();

    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nsource=doc-a\nbudget payload one".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\nsource=doc-b\nbudget payload two".to_vec(),
    )
    .unwrap();

    let stats = db.storage_stats().unwrap();
    assert_eq!(stats.memtable_payload_bytes, stats.memtable.payload_bytes);
    assert!(stats.estimated_memtable_bytes > stats.memtable_payload_bytes);
    assert!(stats.estimated_index_bytes > 0);
    assert!(stats.estimated_context_pack_bytes > stats.memtable_payload_bytes);
    assert_eq!(
        stats.estimated_total_memory_bytes,
        stats
            .estimated_memtable_bytes
            .saturating_add(stats.estimated_index_bytes)
            .saturating_add(stats.estimated_context_pack_bytes)
    );

    db.checkpoint().unwrap();
    let checkpointed = db.storage_stats().unwrap();
    assert!(checkpointed.estimated_index_bytes > 0);
    assert!(checkpointed.estimated_total_memory_bytes >= checkpointed.estimated_index_bytes);
}
