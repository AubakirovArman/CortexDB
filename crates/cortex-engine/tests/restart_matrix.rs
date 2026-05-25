use cortex_core::{CellId, CommitSeq};
use cortex_engine::Database;

#[test]
fn checkpoint_then_patch_tail_survives_restart() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(1), b"base".to_vec()).unwrap();
        db.checkpoint().unwrap();
        db.patch_cell(CellId(1), b"patched-tail".to_vec()).unwrap();
    }

    let db = Database::open(dir.path()).unwrap();
    assert_eq!(db.current_seq(), CommitSeq(2));
    assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"patched-tail");
}

#[test]
fn checkpoint_then_tombstone_tail_survives_restart() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(1), b"base".to_vec()).unwrap();
        db.checkpoint().unwrap();
        db.tombstone_cell(CellId(1)).unwrap();
    }

    let db = Database::open(dir.path()).unwrap();
    assert_eq!(db.current_seq(), CommitSeq(2));
    assert_eq!(db.get_latest_cell(CellId(1)), None);
}

#[test]
fn compact_then_patch_tail_survives_restart() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(1), b"base".to_vec()).unwrap();
        db.compact().unwrap();
        db.patch_cell(CellId(1), b"patched-after-compact".to_vec())
            .unwrap();
    }

    let db = Database::open(dir.path()).unwrap();
    assert_eq!(db.current_seq(), CommitSeq(2));
    assert_eq!(
        db.get_latest_cell(CellId(1)).unwrap(),
        b"patched-after-compact"
    );
}

#[test]
fn compact_then_tombstone_tail_survives_restart() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(1), b"base".to_vec()).unwrap();
        db.compact().unwrap();
        db.tombstone_cell(CellId(1)).unwrap();
    }

    let db = Database::open(dir.path()).unwrap();
    assert_eq!(db.current_seq(), CommitSeq(2));
    assert_eq!(db.get_latest_cell(CellId(1)), None);
}
