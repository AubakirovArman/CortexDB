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
