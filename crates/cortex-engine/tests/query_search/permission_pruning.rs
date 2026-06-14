use super::common::prelude::*;
use super::common::view;

#[test]
fn aql_permission_pruning_skips_unreadable_checkpoint_index_segments() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:allowed\nstatus=ready\n\nallowed budget".to_vec(),
    )
    .unwrap();
    db.checkpoint().unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:denied\nstatus=ready\n\ndenied budget".to_vec(),
    )
    .unwrap();
    db.checkpoint().unwrap();

    let access = view(scope_id("project:allowed"));
    let report = db
        .explain_retrieve_aql(
            r#"EXPLAIN RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
LIMIT 10 CANDIDATES;"#,
            &access,
        )
        .unwrap();
    assert!(report.filters.iter().any(|filter| {
        filter.kind == "permission_pruning"
            && filter.expression == "segments_skipped=1 segments_opened=1 total_segments=2"
    }));
    assert_eq!(report.candidate_counts.after_bitmap, 1);

    let cells = db
        .retrieve_aql(
            r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
LIMIT 10 CANDIDATES;"#,
            &access,
        )
        .unwrap();
    assert_eq!(
        cells.iter().map(|cell| cell.cell_id).collect::<Vec<_>>(),
        vec![CellId(1)]
    );
}

#[test]
fn aql_permission_pruning_skipped_segment_removes_stale_readable_candidate() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:allowed\nstatus=ready\n\nold allowed budget".to_vec(),
    )
    .unwrap();
    db.checkpoint().unwrap();
    db.patch_cell(
        CellId(1),
        b"scope=project:denied\nstatus=ready\n\nmoved denied budget".to_vec(),
    )
    .unwrap();
    db.checkpoint().unwrap();

    let access = view(scope_id("project:allowed"));
    let report = db
        .explain_retrieve_aql(
            r#"EXPLAIN RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
LIMIT 10 CANDIDATES;"#,
            &access,
        )
        .unwrap();
    assert!(report.filters.iter().any(|filter| {
        filter.kind == "permission_pruning"
            && filter.expression == "segments_skipped=1 segments_opened=1 total_segments=2"
    }));
    assert_eq!(report.candidate_counts.after_bitmap, 0);

    let cells = db
        .retrieve_aql(
            r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
LIMIT 10 CANDIDATES;"#,
            &access,
        )
        .unwrap();
    assert!(cells.is_empty());
}
