use crate::helpers::*;

#[test]
fn database_keyword_search_returns_visible_cells() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nalpha budget approved".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=tenant:private\nstatus=ready\nalpha budget hidden".to_vec(),
    )
    .unwrap();

    let results = db
        .search_keyword("budget", &view("project:investments"), SearchLimit(10))
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].cell_id, CellId(1));
    assert!(String::from_utf8_lossy(&results[0].payload).contains("approved"));
}

#[test]
fn database_keyword_search_applies_acl_before_topk_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    seed_private_stronger_keyword_cell(&mut db);

    assert_keyword_limit_one_returns_public_cell(&db);
}

#[test]
fn database_keyword_search_uses_descriptor_scope_for_snapshot_acl() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(
        CellId(1),
        KnowledgeCell::new(
            KnowledgeCellMetadata {
                scope: "tenant:private".to_owned(),
                status: "ready".to_owned(),
                cell_type: KnowledgeCellType::Raw,
                ..Default::default()
            },
            b"scope=project:investments\nstatus=ready\n\nbudget hidden spoof".to_vec(),
        ),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\n\nbudget approved".to_vec(),
    )
    .unwrap();

    let results = db
        .search_keyword("budget", &view("project:investments"), SearchLimit(10))
        .unwrap();

    assert_eq!(
        results
            .iter()
            .map(|result| result.cell_id)
            .collect::<Vec<_>>(),
        vec![CellId(2)]
    );
}

#[test]
fn database_keyword_search_applies_acl_before_topk_persisted() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    seed_private_stronger_keyword_cell(&mut db);
    db.checkpoint().unwrap();

    assert_keyword_limit_one_returns_public_cell(&db);
}

#[test]
fn database_vector_search_applies_acl_before_topk_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    seed_private_stronger_vector_cell(&mut db);

    assert_vector_limit_one_returns_public_cell(&db);
}

#[test]
fn database_vector_search_applies_acl_before_topk_persisted() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    seed_private_stronger_vector_cell(&mut db);
    db.checkpoint().unwrap();

    assert_vector_limit_one_returns_public_cell(&db);
}

#[test]
fn database_hybrid_search_applies_acl_before_topk_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    seed_private_stronger_hybrid_cell(&mut db);

    assert_hybrid_limit_one_returns_public_cell(&db);
}

#[test]
fn database_hybrid_search_applies_acl_before_topk_persisted() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    seed_private_stronger_hybrid_cell(&mut db);
    db.checkpoint().unwrap();

    assert_hybrid_limit_one_returns_public_cell(&db);
}
