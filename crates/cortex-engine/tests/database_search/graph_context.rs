use crate::helpers::*;

#[test]
fn database_search_expands_child_hit_with_parent_context_live_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    seed_parent_child_context_cells(&mut db);

    let results = search_child_anchor(&db);

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].cell_id, CellId(2));
    assert_eq!(results[1].cell_id, CellId(1));
    assert!(String::from_utf8_lossy(&results[1].payload).contains("Parent context"));
}

#[test]
fn database_search_expands_child_hit_with_parent_context_from_persisted_index() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        seed_parent_child_context_cells(&mut db);
        db.checkpoint().unwrap();
    }
    let db = Database::open(dir.path()).unwrap();

    let results = search_child_anchor(&db);

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].cell_id, CellId(2));
    assert_eq!(results[1].cell_id, CellId(1));
    assert!(String::from_utf8_lossy(&results[1].payload).contains("Parent context"));
}

#[test]
fn database_search_expands_child_hit_with_parent_context_lazy_checkpoint_reopen() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        seed_parent_child_context_cells(&mut db);
        db.checkpoint().unwrap();
    }
    let db = Database::open_with_options(
        dir.path(),
        DatabaseOptions {
            payload_residency: PayloadResidency::Lazy,
            ..DatabaseOptions::default()
        },
    )
    .unwrap();

    assert_eq!(db.storage_stats().unwrap().memtable_payload_bytes, 0);
    let results = search_child_anchor(&db);

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].cell_id, CellId(2));
    assert_eq!(results[1].cell_id, CellId(1));
    assert!(String::from_utf8_lossy(&results[1].payload).contains("Parent context"));
}

#[test]
fn database_search_project_query_adds_same_project_artifacts_live_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    seed_project_artifact_cells(&mut db);

    let results = search_project_launch_owner(&db);

    assert_eq!(results.len(), 3);
    assert_eq!(results[0].cell_id, CellId(1));
    assert!(results.iter().any(|result| result.cell_id == CellId(2)));
    assert!(results.iter().any(|result| result.cell_id == CellId(3)));
    assert!(!results.iter().any(|result| result.cell_id == CellId(4)));
}

#[test]
fn database_search_project_query_adds_same_project_artifacts_from_persisted_index() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        seed_project_artifact_cells(&mut db);
        db.checkpoint().unwrap();
    }
    let db = Database::open(dir.path()).unwrap();

    let results = search_project_launch_owner(&db);

    assert_eq!(results.len(), 3);
    assert_eq!(results[0].cell_id, CellId(1));
    assert!(results.iter().any(|result| result.cell_id == CellId(2)));
    assert!(results.iter().any(|result| result.cell_id == CellId(3)));
    assert!(!results.iter().any(|result| result.cell_id == CellId(4)));
}

#[test]
fn database_search_project_query_adds_same_project_artifacts_lazy_checkpoint_reopen() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        seed_project_artifact_cells(&mut db);
        db.checkpoint().unwrap();
    }
    let db = Database::open_with_options(
        dir.path(),
        DatabaseOptions {
            payload_residency: PayloadResidency::Lazy,
            ..DatabaseOptions::default()
        },
    )
    .unwrap();

    assert_eq!(db.storage_stats().unwrap().memtable_payload_bytes, 0);
    let results = search_project_launch_owner(&db);

    assert_eq!(results.len(), 3);
    assert_eq!(results[0].cell_id, CellId(1));
    assert!(results.iter().any(|result| result.cell_id == CellId(2)));
    assert!(results.iter().any(|result| result.cell_id == CellId(3)));
    assert!(!results.iter().any(|result| result.cell_id == CellId(4)));
}

#[test]
fn search_context_store_tracks_patch_tombstone_checkpoint_and_reopen() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        seed_parent_child_context_cells(&mut db);
        db.put_cell(
            CellId(10),
            b"scope=project:investments\nstatus=ready\nproject=Apollo\nowner=Maya\ntitle=Launch owner\n\nlaunch owner Maya"
                .to_vec(),
        )
        .unwrap();
        db.put_cell(
            CellId(11),
            b"scope=project:investments\nstatus=ready\nproject=Apollo\nstatus_tag=blocked\ntitle=Blocked launch\n\nblocked launch artifact"
                .to_vec(),
        )
        .unwrap();

        assert!(search_child_anchor(&db)
            .iter()
            .any(|result| result.cell_id == CellId(1)));
        assert!(search_project_launch_owner(&db)
            .iter()
            .any(|result| result.cell_id == CellId(11)));

        db.patch_cell(
            CellId(1),
            b"scope=project:investments\nstatus=ready\ndocument_id=doc-alpha\nchunk_id=parent-alpha\nchunk_role=note\ntitle=Alpha archive\n\nArchived context no longer expands."
                .to_vec(),
        )
        .unwrap();
        db.patch_cell(
            CellId(11),
            b"scope=project:investments\nstatus=ready\nproject=Zeus\nstatus_tag=blocked\ntitle=Blocked Zeus\n\nzeus artifact"
                .to_vec(),
        )
        .unwrap();

        assert!(!search_child_anchor(&db)
            .iter()
            .any(|result| result.cell_id == CellId(1)));
        assert!(!search_project_launch_owner(&db)
            .iter()
            .any(|result| result.cell_id == CellId(11)));
        db.checkpoint().unwrap();
    }

    let db = Database::open(dir.path()).unwrap();
    assert!(!search_child_anchor(&db)
        .iter()
        .any(|result| result.cell_id == CellId(1)));
    assert!(!search_project_launch_owner(&db)
        .iter()
        .any(|result| result.cell_id == CellId(11)));
}
