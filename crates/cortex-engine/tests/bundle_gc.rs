use cortex_core::CellId;
use cortex_engine::{Database, SegmentBundle};

#[test]
fn segment_bundle_paths_are_stable() {
    let root = std::path::Path::new("segments");
    let bundle = SegmentBundle::new(root, 7);

    assert_eq!(bundle.segment_path, root.join("segment-7.acs"));
    assert_eq!(bundle.bitmap_path, root.join("segment-7.acb"));
    assert_eq!(bundle.lexical_path, root.join("segment-7.aci"));
    assert_eq!(bundle.vector_path, root.join("segment-7.acv"));
    assert_eq!(bundle.hnsw_path, root.join("segment-7.ach"));
}

#[test]
fn live_and_retired_segment_bundles_reflect_manifest() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(CellId(1), b"one".to_vec()).unwrap();
    db.checkpoint().unwrap();
    db.compact().unwrap();

    assert_eq!(db.live_segment_bundles().len(), 1);
    assert_eq!(db.retired_segment_bundles().len(), 1);
    assert!(db.live_segment_bundles()[0].exists_all());
    assert!(db.retired_segment_bundles()[0].exists_all());
}

#[test]
fn gc_retired_segments_removes_files_and_preserves_live_data() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(CellId(1), b"one".to_vec()).unwrap();
    db.checkpoint().unwrap();
    db.patch_cell(CellId(1), b"two".to_vec()).unwrap();
    db.compact().unwrap();

    let retired = db.retired_segment_bundles();
    assert_eq!(retired.len(), 1);
    assert!(retired[0].exists_all());

    let report = db.garbage_collect_retired_segments().unwrap();
    assert_eq!(report.retired_segments_removed, 1);
    assert_eq!(report.files_removed, 4);
    assert!(db.retired_segment_bundles().is_empty());
    assert!(!retired[0].segment_path.exists());
    assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"two");
    assert!(db.validate_storage().is_ok());

    drop(db);
    let db = Database::open(dir.path()).unwrap();
    assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"two");
    assert!(db.manifest().retired_segments.is_empty());
}

#[test]
fn gc_retired_segments_noops_without_retired_segments() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    let report = db.garbage_collect_retired_segments().unwrap();

    assert_eq!(report.retired_segments_removed, 0);
    assert_eq!(report.files_removed, 0);
}
