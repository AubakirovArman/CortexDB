use crate::helpers::*;

#[test]
fn database_keyword_search_uses_body_terms_not_header_terms() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\n\nbody budget".to_vec(),
    )
    .unwrap();

    assert!(db
        .search_keyword("project", &view("project:investments"), SearchLimit(10))
        .unwrap()
        .is_empty());
    assert_eq!(
        db.search_keyword("budget", &view("project:investments"), SearchLimit(10))
            .unwrap()[0]
            .cell_id,
        CellId(1)
    );
}

#[test]
fn checkpoint_lexical_index_persists_doc_lengths() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\n\nalpha budget budget".to_vec(),
    )
    .unwrap();
    db.checkpoint().unwrap();

    let index = LexicalIndex::read(dir.path().join("segments").join("segment-1.aci")).unwrap();
    assert_eq!(index.doc_lengths.get(&1), Some(&3));
    assert_eq!(
        index
            .term_frequencies
            .get("budget")
            .and_then(|values| values.get(&1)),
        Some(&2)
    );
}

#[test]
fn database_keyword_search_uses_persisted_title_weighting() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\ntitle=budget\n\nworkflow note".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\n\nbudget budget".to_vec(),
    )
    .unwrap();
    db.checkpoint().unwrap();

    let results = db
        .search_keyword("budget", &view("project:investments"), SearchLimit(2))
        .unwrap();

    assert_eq!(results[0].cell_id, CellId(1));
    assert!(results[0].lexical_score > results[1].lexical_score);
}

#[test]
fn database_keyword_search_consumes_persisted_corpus_synonyms() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\n\nzephyr quartz rollout".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\n\nzephyr quartz incident".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(3),
        b"scope=project:investments\nstatus=ready\n\nquartz migration note".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(4),
        b"scope=project:investments\nstatus=ready\n\nzephyr rollout status".to_vec(),
    )
    .unwrap();

    let before = db
        .search_keyword("zephyr", &view("project:investments"), SearchLimit(3))
        .unwrap();
    assert!(!before.iter().any(|result| result.cell_id == CellId(3)));

    db.persist_corpus_synonym_dictionary(CorpusSynonymOptions::default())
        .unwrap();

    let snapshot_results = db
        .search_keyword("zephyr", &view("project:investments"), SearchLimit(3))
        .unwrap();
    assert!(snapshot_results
        .iter()
        .any(|result| result.cell_id == CellId(3)));

    db.checkpoint().unwrap();
    let persisted_results = db
        .search_keyword("zephyr", &view("project:investments"), SearchLimit(3))
        .unwrap();
    assert!(persisted_results
        .iter()
        .any(|result| result.cell_id == CellId(3)));
}

#[test]
fn checkpoint_publishes_corpus_synonyms_for_search() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\n\nzephyr quartz rollout".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\n\nzephyr quartz incident".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(3),
        b"scope=project:investments\nstatus=ready\n\nquartz migration note".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(4),
        b"scope=project:investments\nstatus=ready\n\nzephyr rollout status".to_vec(),
    )
    .unwrap();

    assert!(!db.corpus_synonym_dictionary_path().exists());
    db.checkpoint().unwrap();
    assert!(db.corpus_synonym_dictionary_path().exists());

    let results = db
        .search_keyword("zephyr", &view("project:investments"), SearchLimit(4))
        .unwrap();
    assert!(results.iter().any(|result| result.cell_id == CellId(3)));
}

#[test]
fn checkpoint_publishes_abbreviation_synonyms_for_search() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\n\nThe single sign on (SSO) rollout is blocked."
            .to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\n\nSingle sign on migration playbook.".to_vec(),
    )
    .unwrap();

    let before = db
        .search_keyword("SSO", &view("project:investments"), SearchLimit(2))
        .unwrap();
    assert!(!before.iter().any(|result| result.cell_id == CellId(2)));

    db.checkpoint().unwrap();

    let after = db
        .search_keyword("SSO", &view("project:investments"), SearchLimit(2))
        .unwrap();
    assert!(after.iter().any(|result| result.cell_id == CellId(2)));
}
