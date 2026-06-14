use super::common::prelude::*;
use super::common::view;

#[test]
fn aql_segment_pruning_skips_non_matching_type_segments() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    for id in 1..=10 {
        let cell_type = if id == 3 || id == 8 {
            "fact"
        } else {
            "document_block"
        };
        db.put_cell(
            CellId(id),
            format!("scope=project:wide\nstatus=ready\ntype={cell_type}\n\nbudget {id}")
                .into_bytes(),
        )
        .unwrap();
        db.checkpoint().unwrap();
    }

    let access = view(scope_id("project:wide"));
    let report = db
        .explain_retrieve_aql(
            r#"EXPLAIN RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
WHERE space = project:wide AND type = "fact" LIMIT 10 CANDIDATES;"#,
            &access,
        )
        .unwrap();
    assert!(report.filters.iter().any(|filter| {
        filter.kind == "segment_pruning"
            && filter.expression == "segments_skipped=8 segments_opened=2 total_segments=10"
    }));
    assert_eq!(report.candidate_counts.after_bitmap, 2);

    let cells = db
        .retrieve_aql(
            r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
WHERE space = project:wide AND type = "fact" LIMIT 10 CANDIDATES;"#,
            &access,
        )
        .unwrap();
    assert_eq!(
        cells.iter().map(|cell| cell.cell_id).collect::<Vec<_>>(),
        vec![CellId(3), CellId(8)]
    );
}

#[test]
fn aql_segment_pruning_uses_created_range_for_freshness_requirement() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:wide\nstatus=ready\ntype=fact\ncreated_unix_seconds=1\n\nold budget"
            .to_vec(),
    )
    .unwrap();
    db.checkpoint().unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:wide\nstatus=ready\ntype=fact\ncreated_unix_seconds=4000000000\n\nnew budget"
            .to_vec(),
    )
    .unwrap();
    db.checkpoint().unwrap();

    let access = view(scope_id("project:wide"));
    let report = db
        .explain_retrieve_aql(
            r#"EXPLAIN RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
WHERE space = project:wide LIMIT 10 CANDIDATES
REQUIRE freshness <= 3600 SECONDS;"#,
            &access,
        )
        .unwrap();
    assert!(report.filters.iter().any(|filter| {
        filter.kind == "segment_pruning"
            && filter.expression == "segments_skipped=1 segments_opened=1 total_segments=2"
    }));
    assert_eq!(report.candidate_counts.after_quality, 1);

    let cells = db
        .retrieve_aql(
            r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
WHERE space = project:wide LIMIT 10 CANDIDATES
REQUIRE freshness <= 3600 SECONDS;"#,
            &access,
        )
        .unwrap();
    assert_eq!(
        cells.iter().map(|cell| cell.cell_id).collect::<Vec<_>>(),
        vec![CellId(2)]
    );
}
