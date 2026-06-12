use crate::helpers::*;

#[test]
fn database_vector_search_uses_named_view_vectors_in_live_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nvector=0,100\n\nbody vector only".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\ntitle_vector=100,0\nvector=0,1\n\nview vector"
            .to_vec(),
    )
    .unwrap();

    let results = db
        .search_vector(&[100, 0], &view("project:investments"), SearchLimit(2))
        .unwrap();

    assert_eq!(results[0].cell_id, CellId(2));
    assert!(results[0].vector_score > 0);
}

#[test]
fn database_vector_search_report_explains_winning_view_vector() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nvector=0,100\n\nbody vector only".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\ntitle_vector=100,0\nvector=0,1\n\nview vector"
            .to_vec(),
    )
    .unwrap();

    let outcome = db
        .search_vector_with_report(&[100, 0], &view("project:investments"), SearchLimit(2))
        .unwrap();

    assert_eq!(outcome.results[0].cell_id, CellId(2));
    assert_eq!(outcome.view_traces[0].cell_id, CellId(2));
    assert_eq!(outcome.view_traces[0].candidate_id, 2);
    assert_eq!(outcome.view_traces[0].vector_view.as_deref(), Some("title"));
    assert!(outcome.view_traces[0].vector_score > 0);
}

#[test]
fn database_hybrid_search_uses_named_view_vectors_in_live_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nvector=0,100\n\nbudget body".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\ntitle=budget view\ntitle_vector=100,0\nvector=0,1\n\ncommon".to_vec(),
    )
    .unwrap();

    let results = db
        .search_cells(
            SearchQuery {
                text: "budget",
                vector: Some(&[100, 0]),
                limit: 2,
                mode: SearchMode::Hybrid,
            },
            &view("project:investments"),
        )
        .unwrap();

    assert_eq!(results[0].cell_id, CellId(2));
    assert!(results[0].lexical_score > 0);
    assert!(results[0].vector_score > 0);
}
