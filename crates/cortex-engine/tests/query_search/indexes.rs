use super::common::prelude::*;

#[test]
fn bm25_and_vector_indexes_rank_candidates() {
    let mut bm25 = Bm25Index::default();
    bm25.add_document(1, "ready investment budget budget");
    bm25.add_document(2, "workflow note");
    assert_eq!(bm25.search("budget", 1)[0].cell_id, 1);

    let mut vector = VectorIndex::default();
    vector.add_vector(1, vec![3, 0, 1]);
    vector.add_vector(2, vec![0, 4, 0]);
    assert_eq!(vector.search_dot(&[2, 0, 1], 1)[0].cell_id, 1);

    let mut hnsw = HnswIndex::default();
    let _ = hnsw.add_vector(7, vec![1, 2, 3]);
    assert_eq!(hnsw.search(&[1, 1, 1], 1)[0].cell_id, 7);
}

#[test]
fn unicode_tokenizer_handles_ru_kz_en_terms() {
    let terms = tokenize("Бюджет және Project-2025, инвестиции");
    assert!(terms.contains(&"бюджет".to_owned()));
    assert!(terms.contains(&"project".to_owned()));
    assert!(terms.contains(&"2025".to_owned()));
    assert!(terms.contains(&"инвестиции".to_owned()));
    assert!(!terms.contains(&"және".to_owned()));
}

#[test]
fn field_weighting_prioritizes_important_fields() {
    let mut index = Bm25Index::default();
    index.add_document_fields(1, &[("budget", 6), ("workflow note", 1)]);
    index.add_document_fields(2, &[("budget budget", 1)]);

    let results = index.search("budget", 2);
    assert_eq!(results[0].cell_id, 1);
}

#[test]
fn field_aware_bm25_prioritizes_title_over_body_frequency() {
    let mut indexes = SearchIndexes::default();
    indexes.add_field_terms(
        1,
        BTreeMap::from([
            (
                "title".to_owned(),
                BTreeMap::from([("apollo".to_owned(), 1)]),
            ),
            (
                "body".to_owned(),
                BTreeMap::from([("status".to_owned(), 8)]),
            ),
        ]),
    );
    indexes.add_field_terms(
        2,
        BTreeMap::from([(
            "body".to_owned(),
            BTreeMap::from([("apollo".to_owned(), 3)]),
        )]),
    );

    let results = indexes.search(SearchQuery {
        text: "apollo",
        vector: None,
        limit: 2,
        mode: SearchMode::Keyword,
    });

    assert_eq!(results[0].cell_id, 1);
}

#[test]
fn replacing_document_removes_old_postings() {
    let mut index = Bm25Index::default();
    index.add_document(1, "obsolete");
    index.add_document(1, "current");

    assert!(index.search("obsolete", 10).is_empty());
    assert_eq!(index.search("current", 10)[0].cell_id, 1);
}

#[test]
fn hybrid_search_fuses_keyword_and_vector_rankings() {
    let mut indexes = SearchIndexes::default();
    indexes.add_document(1, "ready investment budget");
    indexes.add_document(2, "workflow note");
    indexes.add_vector(1, vec![1, 0, 0]);
    indexes.add_vector(2, vec![5, 0, 0]);

    let results = indexes.search(SearchQuery {
        text: "budget",
        vector: Some(&[5, 0, 0]),
        limit: 2,
        mode: SearchMode::Hybrid,
    });
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].cell_id, 1);
    assert!(results[0].lexical_score > 0);
    assert!(results[0].vector_score > 0);
}

#[test]
fn rrf_both_lists_boosts_overlap_document() {
    let mut indexes = SearchIndexes::default();
    // Cell 1: appears in both lexical (rank 0) and vector (rank 1)
    indexes.add_document(1, "budget investment");
    indexes.add_vector(1, vec![1, 0, 0]);
    // Cell 2: appears only in lexical (rank 1)
    indexes.add_document(2, "budget workflow");
    // Cell 3: appears only in vector (rank 0)
    indexes.add_vector(3, vec![5, 0, 0]);

    let results = indexes.search(SearchQuery {
        text: "budget",
        vector: Some(&[5, 0, 0]),
        limit: 3,
        mode: SearchMode::Hybrid,
    });
    assert_eq!(results.len(), 3);
    // Cell 1 is in both lists → highest fused score
    assert_eq!(results[0].cell_id, 1);
    assert!(results[0].lexical_score > 0);
    assert!(results[0].vector_score > 0);
}

#[test]
fn rrf_empty_lexical_falls_back_to_vector_only() {
    let mut indexes = SearchIndexes::default();
    indexes.add_document(1, "unrelated text");
    indexes.add_vector(1, vec![1, 0, 0]);
    indexes.add_vector(2, vec![5, 0, 0]);

    let results = indexes.search(SearchQuery {
        text: "nonexistent_term_xyz",
        vector: Some(&[5, 0, 0]),
        limit: 2,
        mode: SearchMode::Hybrid,
    });
    assert_eq!(results.len(), 2);
    // Pure vector ranking when lexical is empty
    assert_eq!(results[0].cell_id, 2);
    assert_eq!(results[1].cell_id, 1);
    assert_eq!(results[0].lexical_score, 0);
    assert!(results[0].vector_score > 0);
}

#[test]
fn rrf_empty_vector_falls_back_to_keyword_only() {
    let mut indexes = SearchIndexes::default();
    indexes.add_document(1, "alpha budget");
    indexes.add_document(2, "beta budget");

    let results = indexes.search(SearchQuery {
        text: "budget",
        vector: Some(&[0, 0, 0]), // no vector match
        limit: 2,
        mode: SearchMode::Hybrid,
    });
    assert_eq!(results.len(), 2);
    // Pure lexical ranking when vector is empty
    assert_eq!(results[0].cell_id, 1);
    assert!(results[0].lexical_score > 0);
    assert_eq!(results[0].vector_score, 0);
}

#[test]
fn rrf_truncate_respects_limit() {
    let mut indexes = SearchIndexes::default();
    for id in 1..=10 {
        indexes.add_document(id, &format!("term {id}"));
        indexes.add_vector(id, vec![id as i16, 0]);
    }

    let results = indexes.search(SearchQuery {
        text: "term",
        vector: Some(&[5, 0]),
        limit: 3,
        mode: SearchMode::Hybrid,
    });
    assert_eq!(results.len(), 3);
}
