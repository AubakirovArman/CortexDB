use cortex_core::{CellId, CommitSeq};
use cortex_engine::Database;

#[test]
fn pinned_snapshot_survives_compaction() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(CellId(1), b"v1".to_vec()).unwrap();

    let pin = db.pin_read_txn();
    let pinned_txn = pin.read_txn();

    db.patch_cell(CellId(1), b"v2".to_vec()).unwrap();
    db.compact().unwrap();

    assert_eq!(db.get_cell(pinned_txn, CellId(1)).unwrap(), b"v1");
    assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"v2");
    drop(pin);
}

#[test]
fn gc_horizon_protects_pinned_snapshot_from_checkpoint() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(CellId(1), b"v1".to_vec()).unwrap();

    let pin = db.pin_read_txn();
    let pinned_txn = pin.read_txn();
    let pinned_seq = pinned_txn.read_seq;

    db.patch_cell(CellId(1), b"v2".to_vec()).unwrap();
    let stats = db.checkpoint().unwrap();
    assert_eq!(stats.checkpoint_seq, CommitSeq(2));

    assert_eq!(db.gc_horizon(), pinned_seq);
    assert_eq!(db.get_cell(pinned_txn, CellId(1)).unwrap(), b"v1");
    drop(pin);

    assert_eq!(db.gc_horizon(), db.current_seq());
}

#[test]
fn unpinned_old_version_is_removed_by_compaction() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(CellId(1), b"v1".to_vec()).unwrap();
    let old_txn = db.read_txn();

    db.patch_cell(CellId(1), b"v2".to_vec()).unwrap();
    db.compact().unwrap();

    assert!(db.get_cell(old_txn, CellId(1)).is_none());
    assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"v2");
}
