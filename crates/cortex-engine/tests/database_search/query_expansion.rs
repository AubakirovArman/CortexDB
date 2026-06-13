use crate::helpers::*;

#[test]
fn search_query_understanding_extracts_anchors_without_oracle_metadata() {
    let analyzed = analyze_search_query(
        "Find the GitHub PR #77 for AUTH-456 in src/server/auth.rs before v2.1.0",
    );

    assert!(analyzed.source_hints.contains(&"github".to_owned()));
    assert!(analyzed
        .anchors
        .iter()
        .any(|anchor| anchor.kind == QueryAnchorKind::PullRequest && anchor.text == "#77"));
    assert!(analyzed
        .anchors
        .iter()
        .any(|anchor| anchor.kind == QueryAnchorKind::TicketId && anchor.text == "AUTH-456"));
    assert!(analyzed
        .anchors
        .iter()
        .any(|anchor| anchor.kind == QueryAnchorKind::FilePath));
}

#[test]
fn database_keyword_search_uses_query_expansion_from_question_text() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\n\nrelease dependency assigned to DRI".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\n\nlaunch celebration notes".to_vec(),
    )
    .unwrap();

    let results = db
        .search_keyword(
            "Who owns the blocker?",
            &view("project:investments"),
            SearchLimit(2),
        )
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].cell_id, CellId(1));
    assert!(results[0].lexical_score > 0);
}

#[test]
fn database_keyword_search_uses_bidirectional_query_expansion_from_question_text() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\n\nrelease owner is Maya; blocker is dependency on auth"
            .to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\n\nlaunch celebration notes".to_vec(),
    )
    .unwrap();

    let results = db
        .search_keyword(
            "Who is the DRI for the slipped rollout?",
            &view("project:investments"),
            SearchLimit(2),
        )
        .unwrap();

    assert_eq!(results[0].cell_id, CellId(1));
    assert!(results[0].lexical_score > 0);
}

#[test]
fn database_keyword_search_uses_high_level_phrase_expansion_from_question_text() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\ntitle=Company charter\n\nOur mission is to provide enterprise context infrastructure."
            .to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\n\nweekly sprint note".to_vec(),
    )
    .unwrap();

    let results = db
        .search_keyword(
            "Give me the high level company overview",
            &view("project:investments"),
            SearchLimit(2),
        )
        .unwrap();

    assert_eq!(results[0].cell_id, CellId(1));
    assert!(results[0].lexical_score > 0);
}

#[test]
fn database_search_high_level_query_fills_summary_anchor_live_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    seed_high_level_anchor_cells(&mut db);

    let results = search_high_level_anchor(&db);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].cell_id, CellId(1));
    assert!(String::from_utf8_lossy(&results[0].payload).contains("Northstar"));
}

#[test]
fn database_search_high_level_query_fills_summary_anchor_from_persisted_index() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        seed_high_level_anchor_cells(&mut db);
        db.checkpoint().unwrap();
    }
    let db = Database::open(dir.path()).unwrap();

    let results = search_high_level_anchor(&db);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].cell_id, CellId(1));
    assert!(String::from_utf8_lossy(&results[0].payload).contains("Northstar"));
}

#[test]
fn database_search_high_level_query_fills_summary_anchor_lazy_checkpoint_reopen() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        seed_high_level_anchor_cells(&mut db);
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
    let results = search_high_level_anchor(&db);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].cell_id, CellId(1));
    assert!(String::from_utf8_lossy(&results[0].payload).contains("Northstar"));
}
