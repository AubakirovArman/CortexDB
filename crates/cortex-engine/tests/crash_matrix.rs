use cortex_core::{CellId, CommitSeq};
use cortex_engine::Database;
use cortex_storage::hnsw::HnswGraphIndex;
use cortex_storage::indexes::{BitmapIndex, LexicalIndex};
use cortex_storage::segment::{SegmentCell, SegmentWriter};
use cortex_storage::vectors::VectorIndex;

#[test]
fn interrupted_checkpoint_orphan_bundle_is_ignored() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(1), b"scope=default\nstatus=ready\none".to_vec())
            .unwrap();
        db.checkpoint().unwrap();
    }
    write_orphan_bundle(dir.path(), 99, CellId(99));

    let db = Database::open(dir.path()).unwrap();
    assert_eq!(db.current_seq(), CommitSeq(1));
    assert_eq!(
        db.get_latest_cell(CellId(1)).unwrap(),
        b"scope=default\nstatus=ready\none"
    );
    assert_eq!(db.get_latest_cell(CellId(99)), None);
    assert_eq!(db.validate_storage().unwrap().live_segments_checked, 1);
}

#[test]
fn corrupt_manifest_tmp_after_checkpoint_is_removed_on_open() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(1), b"one".to_vec()).unwrap();
        db.checkpoint().unwrap();
    }
    std::fs::write(dir.path().join("manifest.acm.tmp"), b"interrupted").unwrap();

    let db = Database::open(dir.path()).unwrap();
    assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"one");
    assert!(!dir.path().join("manifest.acm.tmp").exists());
}

#[test]
fn missing_manifest_with_persisted_segments_fails_closed() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(1), b"one".to_vec()).unwrap();
        db.checkpoint().unwrap();
    }
    std::fs::remove_file(dir.path().join("manifest.acm")).unwrap();

    let error = Database::open(dir.path()).unwrap_err().to_string();
    assert!(error.contains("missing manifest.acm"));
}

#[test]
fn partial_manifest_fails_closed() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(1), b"one".to_vec()).unwrap();
        db.checkpoint().unwrap();
    }
    std::fs::write(dir.path().join("manifest.acm"), b"ACM0").unwrap();

    assert!(Database::open(dir.path()).is_err());
}

#[test]
fn interrupted_compact_before_segment_write_replays_wal_tail() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(1), b"v1".to_vec()).unwrap();
        db.checkpoint().unwrap();
        db.patch_cell(CellId(1), b"v2-tail".to_vec()).unwrap();
    }

    let db = Database::open(dir.path()).unwrap();
    assert_eq!(db.current_seq(), CommitSeq(2));
    assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"v2-tail");
    assert_eq!(db.manifest().live_segments.len(), 1);
}

#[test]
fn interrupted_compact_after_segment_write_without_manifest_is_ignored() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(1), b"v1".to_vec()).unwrap();
        db.checkpoint().unwrap();
        db.patch_cell(CellId(1), b"v2".to_vec()).unwrap();
        db.checkpoint().unwrap();
    }
    write_orphan_segment_file_only(dir.path(), 42, CellId(42));

    let db = Database::open(dir.path()).unwrap();
    assert_eq!(db.current_seq(), CommitSeq(2));
    assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"v2");
    assert_eq!(db.get_latest_cell(CellId(42)), None);
    assert_eq!(db.validate_storage().unwrap().live_segments_checked, 2);
}

#[test]
fn interrupted_compact_without_manifest_switch_keeps_old_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(1), b"v1".to_vec()).unwrap();
        db.checkpoint().unwrap();
        db.patch_cell(CellId(1), b"v2".to_vec()).unwrap();
        db.checkpoint().unwrap();
    }
    write_orphan_bundle(dir.path(), 42, CellId(42));

    let db = Database::open(dir.path()).unwrap();
    assert_eq!(db.current_seq(), CommitSeq(2));
    assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"v2");
    assert_eq!(db.manifest().live_segments.len(), 2);
}

#[test]
fn compact_after_manifest_update_preserves_retired_segments_until_gc() {
    let dir = tempfile::tempdir().unwrap();
    let retired_paths = {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(1), b"v1".to_vec()).unwrap();
        db.checkpoint().unwrap();
        db.patch_cell(CellId(1), b"v2".to_vec()).unwrap();
        db.checkpoint().unwrap();
        db.compact().unwrap();
        let retired = db.retired_segment_bundles();
        assert_eq!(retired.len(), 2);
        assert!(retired.iter().all(|bundle| bundle.exists_all()));
        retired
            .into_iter()
            .map(|bundle| bundle.segment_path)
            .collect::<Vec<_>>()
    };

    let mut db = Database::open(dir.path()).unwrap();
    assert_eq!(db.manifest().live_segments.len(), 1);
    assert_eq!(db.manifest().retired_segments.len(), 2);
    assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"v2");
    assert!(retired_paths.iter().all(|path| path.exists()));

    let report = db.garbage_collect_retired_segments().unwrap();
    assert_eq!(report.retired_segments_removed, 2);
    assert!(retired_paths.iter().all(|path| !path.exists()));
    assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"v2");
}

fn write_orphan_bundle(root: &std::path::Path, segment_id: u64, cell_id: CellId) {
    let segments = root.join("segments");
    std::fs::create_dir_all(&segments).unwrap();
    SegmentWriter::write(
        segments.join(format!("segment-{segment_id}.acs")),
        &[SegmentCell {
            candidate_id: 999,
            cell_id: cell_id.0,
            created_seq: 999,
            deleted_seq: None,
            payload: b"scope=default\nstatus=ready\norphan".to_vec(),
        }],
    )
    .unwrap();
    BitmapIndex::default()
        .write(segments.join(format!("segment-{segment_id}.acb")))
        .unwrap();
    LexicalIndex::default()
        .write(segments.join(format!("segment-{segment_id}.aci")))
        .unwrap();
    VectorIndex::default()
        .write(segments.join(format!("segment-{segment_id}.acv")))
        .unwrap();
    HnswGraphIndex::default()
        .write(segments.join(format!("segment-{segment_id}.ach")))
        .unwrap();
}

fn write_orphan_segment_file_only(root: &std::path::Path, segment_id: u64, cell_id: CellId) {
    let segments = root.join("segments");
    std::fs::create_dir_all(&segments).unwrap();
    SegmentWriter::write(
        segments.join(format!("segment-{segment_id}.acs")),
        &[SegmentCell {
            candidate_id: 999,
            cell_id: cell_id.0,
            created_seq: 999,
            deleted_seq: None,
            payload: b"scope=default\nstatus=ready\npartial-orphan".to_vec(),
        }],
    )
    .unwrap();
}
