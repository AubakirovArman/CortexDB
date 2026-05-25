use cortex_core::{CellId, CommitSeq};
use cortex_engine::Database;
use cortex_storage::indexes::{BitmapIndex, LexicalIndex};
use cortex_storage::manifest::{ManifestSegment, StorageManifest};
use cortex_storage::segment::{SegmentCell, SegmentWriter};
use cortex_storage::vectors::VectorIndex;

#[test]
fn checkpoint_persists_segment_indexes_and_manifest() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nbudget alpha".to_vec(),
    )
    .unwrap();
    db.put_cell(CellId(2), b"scope=default\nstatus=draft\nbeta".to_vec())
        .unwrap();

    let stats = db.checkpoint().unwrap();
    assert_eq!(stats.segment_id, Some(1));
    assert_eq!(stats.cells_flushed, 2);
    assert_eq!(stats.checkpoint_seq, CommitSeq(2));
    assert_eq!(db.manifest().checkpoint_seq, 2);

    let segment_dir = dir.path().join("segments");
    assert!(segment_dir.join("segment-1.acs").exists());
    let (stored_bitmap, stored_lexical) = db.persisted_indexes().unwrap();
    assert!(!stored_bitmap.bitmaps.is_empty());
    assert!(stored_lexical.terms.contains_key("budget"));
    assert!(!BitmapIndex::read(segment_dir.join("segment-1.acb"))
        .unwrap()
        .bitmaps
        .is_empty());
    assert!(LexicalIndex::read(segment_dir.join("segment-1.aci"))
        .unwrap()
        .terms
        .contains_key("budget"));
}

#[test]
fn checkpoint_survives_restart_without_wal_tail() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(1), b"hello".to_vec()).unwrap();
        db.checkpoint().unwrap();
    }
    let db = Database::open(dir.path()).unwrap();
    assert_eq!(db.current_seq(), CommitSeq(1));
    assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"hello");
}

#[test]
fn wal_tail_after_checkpoint_replays_newer_records() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(1), b"v1".to_vec()).unwrap();
        db.checkpoint().unwrap();
        db.patch_cell(CellId(1), b"v2".to_vec()).unwrap();
    }
    let db = Database::open(dir.path()).unwrap();
    assert_eq!(db.current_seq(), CommitSeq(2));
    assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"v2");
}

#[test]
fn put_after_compact_survives_restart() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(1), b"one".to_vec()).unwrap();
        db.compact().unwrap();
        db.put_cell(CellId(2), b"two".to_vec()).unwrap();
    }
    let db = Database::open(dir.path()).unwrap();
    assert_eq!(db.current_seq(), CommitSeq(2));
    assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"one");
    assert_eq!(db.get_latest_cell(CellId(2)).unwrap(), b"two");
}

#[test]
fn checkpoint_truncates_wal_and_writer_restarts_with_header() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(1), b"one".to_vec()).unwrap();
        db.checkpoint().unwrap();
        db.put_cell(CellId(2), b"two".to_vec()).unwrap();
    }
    let db = Database::open(dir.path()).unwrap();
    assert_eq!(db.current_seq(), CommitSeq(2));
    assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"one");
    assert_eq!(db.get_latest_cell(CellId(2)).unwrap(), b"two");
}

#[test]
fn second_checkpoint_is_incremental() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(CellId(1), b"v1".to_vec()).unwrap();
    db.checkpoint().unwrap();
    db.patch_cell(CellId(1), b"v2".to_vec()).unwrap();
    let stats = db.checkpoint().unwrap();

    assert_eq!(stats.segment_id, Some(2));
    assert_eq!(stats.cells_flushed, 1);
    assert_eq!(db.manifest().live_segments.len(), 2);
    assert_eq!(db.manifest().retired_segments.len(), 0);
    assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"v2");
}

#[test]
fn compact_retires_old_segments_to_full_visible_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(CellId(1), b"v1".to_vec()).unwrap();
    db.checkpoint().unwrap();
    db.patch_cell(CellId(1), b"v2".to_vec()).unwrap();
    db.checkpoint().unwrap();
    let stats = db.compact().unwrap();

    assert_eq!(stats.segment_id, Some(3));
    assert_eq!(stats.cells_flushed, 1);
    assert_eq!(db.manifest().live_segments.len(), 1);
    assert_eq!(db.manifest().retired_segments.len(), 2);
    assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"v2");
}

#[test]
fn tombstone_after_checkpoint_does_not_resurrect_after_flush() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(1), b"hello".to_vec()).unwrap();
        db.checkpoint().unwrap();
        db.tombstone_cell(CellId(1)).unwrap();
        db.checkpoint().unwrap();
    }
    let db = Database::open(dir.path()).unwrap();
    assert_eq!(db.current_seq(), CommitSeq(2));
    assert_eq!(db.get_latest_cell(CellId(1)), None);
}

#[test]
fn tombstone_only_checkpoint_does_not_resurrect_cell() {
    let dir = tempfile::tempdir().unwrap();
    let segments = dir.path().join("segments");
    std::fs::create_dir_all(&segments).unwrap();
    SegmentWriter::write(
        segments.join("segment-1.acs"),
        &[SegmentCell {
            candidate_id: 1,
            cell_id: 42,
            created_seq: 0,
            deleted_seq: Some(7),
            payload: Vec::new(),
        }],
    )
    .unwrap();
    BitmapIndex::default()
        .write(segments.join("segment-1.acb"))
        .unwrap();
    LexicalIndex::default()
        .write(segments.join("segment-1.aci"))
        .unwrap();
    VectorIndex::default()
        .write(segments.join("segment-1.acv"))
        .unwrap();
    StorageManifest {
        generation: 1,
        checkpoint_seq: 7,
        live_segments: vec![ManifestSegment {
            id: 1,
            generation: 1,
            checkpoint_seq: 7,
            cell_count: 1,
        }],
        retired_segments: Vec::new(),
    }
    .store(dir.path().join("manifest.acm"))
    .unwrap();

    let db = Database::open(dir.path()).unwrap();
    assert_eq!(db.current_seq(), CommitSeq(7));
    assert_eq!(db.get_latest_cell(CellId(42)), None);
    assert_eq!(db.validate_storage().unwrap().cells_checked, 1);
}

#[test]
fn storage_stats_and_validate_cover_manifest_segments_and_wal() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(CellId(1), b"hello".to_vec()).unwrap();
    db.checkpoint().unwrap();

    let stats = db.storage_stats().unwrap();
    assert_eq!(stats.current_seq, CommitSeq(1));
    assert_eq!(stats.checkpoint_seq, CommitSeq(1));
    assert_eq!(stats.live_segments, 1);
    assert_eq!(stats.memtable.cell_count, 1);

    let validation = db.validate_storage().unwrap();
    assert_eq!(validation.live_segments_checked, 1);
    assert_eq!(validation.cells_checked, 1);
}

#[test]
fn validate_storage_rejects_segment_count_mismatch() {
    let dir = tempfile::tempdir().unwrap();
    let segments = dir.path().join("segments");
    std::fs::create_dir_all(&segments).unwrap();
    SegmentWriter::write(
        segments.join("segment-1.acs"),
        &[SegmentCell {
            candidate_id: 1,
            cell_id: 1,
            created_seq: 1,
            deleted_seq: None,
            payload: b"hello".to_vec(),
        }],
    )
    .unwrap();
    BitmapIndex::default()
        .write(segments.join("segment-1.acb"))
        .unwrap();
    LexicalIndex::default()
        .write(segments.join("segment-1.aci"))
        .unwrap();
    VectorIndex::default()
        .write(segments.join("segment-1.acv"))
        .unwrap();
    StorageManifest {
        generation: 1,
        checkpoint_seq: 1,
        live_segments: vec![ManifestSegment {
            id: 1,
            generation: 1,
            checkpoint_seq: 1,
            cell_count: 2,
        }],
        retired_segments: Vec::new(),
    }
    .store(dir.path().join("manifest.acm"))
    .unwrap();

    let db = Database::open(dir.path()).unwrap();
    let error = db.validate_storage().unwrap_err().to_string();
    assert!(error.contains("cell_count mismatch"));
}
