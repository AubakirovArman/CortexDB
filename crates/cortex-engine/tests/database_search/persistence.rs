use crate::helpers::*;

#[test]
fn database_keyword_search_survives_checkpoint_restart() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(
            CellId(u64::MAX),
            b"scope=project:investments\nstatus=ready\nlarge budget".to_vec(),
        )
        .unwrap();
        db.checkpoint().unwrap();
    }

    let db = Database::open(dir.path()).unwrap();
    let results = db
        .search_keyword("budget", &view("project:investments"), SearchLimit(10))
        .unwrap();
    assert_eq!(results[0].cell_id, CellId(u64::MAX));
}

#[test]
fn database_keyword_search_reads_persisted_aci_without_wal_tail_changes() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\n\nalpha budget approved".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=tenant:private\nstatus=ready\n\nalpha budget hidden".to_vec(),
    )
    .unwrap();
    db.checkpoint().unwrap();

    let results = db
        .search_keyword("budget", &view("project:investments"), SearchLimit(10))
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].cell_id, CellId(1));
}

#[test]
fn database_keyword_search_falls_back_to_snapshot_for_uncheckpointed_changes() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\n\nold term".to_vec(),
    )
    .unwrap();
    db.checkpoint().unwrap();
    db.patch_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\n\nfresh budget".to_vec(),
    )
    .unwrap();

    let results = db
        .search_keyword("budget", &view("project:investments"), SearchLimit(10))
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].cell_id, CellId(1));
    assert!(String::from_utf8_lossy(&results[0].payload).contains("fresh budget"));
}

#[test]
fn database_vector_search_reads_persisted_acv_without_wal_tail_changes() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nvector=5,0\n\nalpha".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=tenant:private\nstatus=ready\nvector=9,0\n\nhidden".to_vec(),
    )
    .unwrap();
    db.checkpoint().unwrap();

    let results = db
        .search_vector(&[2, 0], &view("project:investments"), SearchLimit(10))
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].cell_id, CellId(1));
    assert!(results[0].vector_score > 0);
}

#[test]
fn database_vector_exact_reads_latest_disk_resident_acv_row() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nvector=100,0\n\nold".to_vec(),
    )
    .unwrap();
    db.checkpoint().unwrap();
    db.patch_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nvector=0,100\n\nfresh".to_vec(),
    )
    .unwrap();
    db.checkpoint().unwrap();

    let old_axis = db
        .search_vector_exact(&[100, 0], &view("project:investments"), SearchLimit(1))
        .unwrap();
    let fresh_axis = db
        .search_vector_exact(&[0, 100], &view("project:investments"), SearchLimit(1))
        .unwrap();

    assert_eq!(old_axis[0].cell_id, CellId(1));
    assert_eq!(old_axis[0].vector_score, 0);
    assert_eq!(fresh_axis[0].cell_id, CellId(1));
    assert!(fresh_axis[0].vector_score > 0);
    assert!(String::from_utf8_lossy(&fresh_axis[0].payload).contains("fresh"));
}

#[test]
fn database_hybrid_search_reads_persisted_aci_and_acv_without_snapshot_rebuild() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(
            CellId(1),
            b"scope=project:investments\nstatus=ready\nvector=1,0,0\n\nbudget investment".to_vec(),
        )
        .unwrap();
        db.put_cell(
            CellId(2),
            b"scope=project:investments\nstatus=ready\nvector=5,0,0\n\nbudget workflow".to_vec(),
        )
        .unwrap();
        db.put_cell(
            CellId(3),
            b"scope=project:investments\nstatus=ready\nvector=9,0,0\n\nunrelated".to_vec(),
        )
        .unwrap();
        db.checkpoint().unwrap();
    }

    let db = Database::open(dir.path()).unwrap();
    let results = db
        .search_cells(
            SearchQuery {
                text: "budget investment",
                vector: Some(&[9, 0, 0]),
                limit: 3,
                mode: SearchMode::Hybrid,
            },
            &view("project:investments"),
        )
        .unwrap();

    assert_eq!(results[0].cell_id, CellId(1));
    assert!(results[0].lexical_score > 0);
    assert!(results[0].vector_score > 0);
    assert!(results.iter().any(|result| result.vector_score > 0));
}

#[test]
fn database_vector_search_falls_back_to_snapshot_for_uncheckpointed_changes() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nvector=0,9\n\nold".to_vec(),
    )
    .unwrap();
    db.checkpoint().unwrap();
    db.patch_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nvector=9,0\n\nfresh".to_vec(),
    )
    .unwrap();

    let results = db
        .search_vector(&[3, 0], &view("project:investments"), SearchLimit(10))
        .unwrap();

    assert_eq!(results[0].cell_id, CellId(1));
    assert!(results[0].vector_score > 0);
    assert!(String::from_utf8_lossy(&results[0].payload).contains("fresh"));
}

#[test]
fn checkpoint_vector_index_persists_payload_vectors() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nvector=3,4\n\nalpha".to_vec(),
    )
    .unwrap();
    db.checkpoint().unwrap();

    let index = VectorIndex::read(dir.path().join("segments").join("segment-1.acv")).unwrap();
    assert_eq!(index.vectors.get(&1), Some(&vec![3, 4]));
    assert!(!dir.path().join("segments").join("segment-1.ach").exists());
}
