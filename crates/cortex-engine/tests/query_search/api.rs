use super::common::prelude::*;
use super::common::view;

#[test]
fn search_api_supports_keyword_and_vector_modes() {
    let mut indexes = SearchIndexes::default();
    indexes.add_document(1, "alpha budget");
    indexes.add_vector(2, vec![0, 9]);

    let keyword = indexes.search(SearchQuery {
        text: "budget",
        vector: None,
        limit: 1,
        mode: SearchMode::Keyword,
    });
    assert_eq!(keyword[0].cell_id, 1);

    let vector = indexes.search(SearchQuery {
        text: "",
        vector: Some(&[0, 2]),
        limit: 1,
        mode: SearchMode::Vector,
    });
    assert_eq!(vector[0].cell_id, 2);
}

#[test]
fn database_search_lazy_payload_residency_reads_checkpoint_and_wal_tail_payloads() {
    let dir = tempfile::tempdir().unwrap();
    let checkpoint_payload =
        b"scope=project:search\nstatus=ready\ntype=fact\n\ncheckpoint alpha payload".to_vec();
    let tail_payload =
        b"scope=project:search\nstatus=ready\ntype=fact\n\nwal tail beta payload".to_vec();

    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(1), checkpoint_payload.clone()).unwrap();
        db.checkpoint().unwrap();
        db.put_cell(CellId(2), tail_payload.clone()).unwrap();
    }

    let db = Database::open_with_options(
        dir.path(),
        DatabaseOptions {
            payload_residency: PayloadResidency::Lazy,
            ..DatabaseOptions::default()
        },
    )
    .unwrap();
    assert_eq!(db.payload_residency(), PayloadResidency::Lazy);

    let view = view(scope_id("project:search"));
    let checkpoint_results = db
        .search_keyword("checkpoint alpha", &view, SearchLimit(10))
        .unwrap();
    assert_eq!(checkpoint_results[0].cell_id, CellId(1));
    assert_eq!(checkpoint_results[0].payload, checkpoint_payload);

    let tail_results = db
        .search_keyword("tail beta", &view, SearchLimit(10))
        .unwrap();
    assert_eq!(tail_results[0].cell_id, CellId(2));
    assert_eq!(tail_results[0].payload, tail_payload);
}
