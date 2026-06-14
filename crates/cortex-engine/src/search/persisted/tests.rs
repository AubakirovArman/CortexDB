use super::*;
use crate::search::Bm25Index;

#[test]
fn persisted_lexical_search_filters_allowed_candidates() {
    let terms = BTreeMap::from([("budget".to_owned(), BTreeSet::from([1, 2]))]);
    let doc_lengths = BTreeMap::from([(1, 3), (2, 3)]);
    let term_frequencies = BTreeMap::new();
    let field_doc_lengths = BTreeMap::new();
    let field_term_frequencies = BTreeMap::new();
    let results = search_persisted_lexical(
        PersistedLexicalSearchIndex {
            terms: &terms,
            doc_lengths: &doc_lengths,
            term_frequencies: &term_frequencies,
            field_doc_lengths: &field_doc_lengths,
            field_term_frequencies: &field_term_frequencies,
        },
        "budget",
        &BTreeSet::from([2]),
        10,
    );

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].cell_id, 2);
}

#[test]
fn persisted_lexical_search_uses_term_frequencies() {
    let terms = BTreeMap::from([("budget".to_owned(), BTreeSet::from([1, 2]))]);
    let doc_lengths = BTreeMap::from([(1, 1), (2, 3)]);
    let term_frequencies =
        BTreeMap::from([("budget".to_owned(), BTreeMap::from([(1, 1), (2, 3)]))]);
    let field_doc_lengths = BTreeMap::new();
    let field_term_frequencies = BTreeMap::new();
    let results = search_persisted_lexical(
        PersistedLexicalSearchIndex {
            terms: &terms,
            doc_lengths: &doc_lengths,
            term_frequencies: &term_frequencies,
            field_doc_lengths: &field_doc_lengths,
            field_term_frequencies: &field_term_frequencies,
        },
        "budget",
        &BTreeSet::from([1, 2]),
        2,
    );

    assert_eq!(results[0].cell_id, 2);
}

#[test]
fn persisted_lexical_search_uses_field_weights_when_available() {
    let terms = BTreeMap::from([("apollo".to_owned(), BTreeSet::from([1, 2]))]);
    let doc_lengths = BTreeMap::from([(1, 8), (2, 8)]);
    let term_frequencies =
        BTreeMap::from([("apollo".to_owned(), BTreeMap::from([(1, 1), (2, 3)]))]);
    let field_doc_lengths = BTreeMap::from([
        ("title".to_owned(), BTreeMap::from([(1, 1)])),
        ("body".to_owned(), BTreeMap::from([(2, 3)])),
    ]);
    let field_term_frequencies = BTreeMap::from([
        (
            "title".to_owned(),
            BTreeMap::from([("apollo".to_owned(), BTreeMap::from([(1, 1)]))]),
        ),
        (
            "body".to_owned(),
            BTreeMap::from([("apollo".to_owned(), BTreeMap::from([(2, 3)]))]),
        ),
    ]);
    let results = search_persisted_lexical(
        PersistedLexicalSearchIndex {
            terms: &terms,
            doc_lengths: &doc_lengths,
            term_frequencies: &term_frequencies,
            field_doc_lengths: &field_doc_lengths,
            field_term_frequencies: &field_term_frequencies,
        },
        "apollo",
        &BTreeSet::from([1, 2]),
        2,
    );

    assert_eq!(results[0].cell_id, 1);
}

#[test]
fn persisted_lexical_scores_match_live_field_bm25() {
    let field_docs = BTreeMap::from([
        (
            1,
            BTreeMap::from([
                (
                    "title".to_owned(),
                    BTreeMap::from([("apollo".to_owned(), 1)]),
                ),
                (
                    "body".to_owned(),
                    BTreeMap::from([("budget".to_owned(), 1), ("status".to_owned(), 2)]),
                ),
            ]),
        ),
        (
            2,
            BTreeMap::from([(
                "body".to_owned(),
                BTreeMap::from([("apollo".to_owned(), 3), ("budget".to_owned(), 1)]),
            )]),
        ),
    ]);
    let mut live = Bm25Index::default();
    for (candidate, fields) in &field_docs {
        live.add_field_terms(*candidate, fields.clone());
    }

    let terms = BTreeMap::from([
        ("apollo".to_owned(), BTreeSet::from([1, 2])),
        ("budget".to_owned(), BTreeSet::from([1, 2])),
        ("status".to_owned(), BTreeSet::from([1])),
    ]);
    let doc_lengths = BTreeMap::from([(1, 12), (2, 4)]);
    let term_frequencies = BTreeMap::from([
        ("apollo".to_owned(), BTreeMap::from([(1, 8), (2, 3)])),
        ("budget".to_owned(), BTreeMap::from([(1, 1), (2, 1)])),
        ("status".to_owned(), BTreeMap::from([(1, 2)])),
    ]);
    let field_doc_lengths = BTreeMap::from([
        ("title".to_owned(), BTreeMap::from([(1, 1)])),
        ("body".to_owned(), BTreeMap::from([(1, 3), (2, 4)])),
    ]);
    let field_term_frequencies = BTreeMap::from([
        (
            "title".to_owned(),
            BTreeMap::from([("apollo".to_owned(), BTreeMap::from([(1, 1)]))]),
        ),
        (
            "body".to_owned(),
            BTreeMap::from([
                ("apollo".to_owned(), BTreeMap::from([(2, 3)])),
                ("budget".to_owned(), BTreeMap::from([(1, 1), (2, 1)])),
                ("status".to_owned(), BTreeMap::from([(1, 2)])),
            ]),
        ),
    ]);

    let persisted = search_persisted_lexical(
        PersistedLexicalSearchIndex {
            terms: &terms,
            doc_lengths: &doc_lengths,
            term_frequencies: &term_frequencies,
            field_doc_lengths: &field_doc_lengths,
            field_term_frequencies: &field_term_frequencies,
        },
        "apollo budget",
        &BTreeSet::from([1, 2]),
        2,
    );
    let live = live.search("apollo budget", 2);

    assert_eq!(persisted, live);
}

#[test]
fn persisted_doc_count_uses_allowed_doc_lengths() {
    let doc_lengths = BTreeMap::from([(1, 8), (2, 9)]);
    let allowed = BTreeSet::from([1, 2, 99]);

    assert_eq!(doc_count(&doc_lengths, &allowed), 2);
}

#[test]
fn persisted_lexical_search_filters_allowed_even_with_extra_allowed_ids() {
    let terms = BTreeMap::from([("budget".to_owned(), BTreeSet::from([1, 2]))]);
    let doc_lengths = BTreeMap::from([(1, 3)]);
    let term_frequencies = BTreeMap::new();
    let field_doc_lengths = BTreeMap::new();
    let field_term_frequencies = BTreeMap::new();
    let results = search_persisted_lexical(
        PersistedLexicalSearchIndex {
            terms: &terms,
            doc_lengths: &doc_lengths,
            term_frequencies: &term_frequencies,
            field_doc_lengths: &field_doc_lengths,
            field_term_frequencies: &field_term_frequencies,
        },
        "budget",
        &BTreeSet::from([1, 99]),
        10,
    );

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].cell_id, 1);
}

#[test]
fn persisted_query_term_selection_prefers_rare_terms() {
    let terms = BTreeMap::from([
        ("common".to_owned(), BTreeSet::from([1, 2, 3, 4])),
        ("rare".to_owned(), BTreeSet::from([4])),
    ]);
    let allowed = BTreeSet::from([1, 2, 3, 4]);
    let selected = selected_query_terms(&terms, "common rare common", &allowed, true);

    assert_eq!(selected.len(), 2);
    assert_eq!(selected[0].term, "rare");
    assert_eq!(selected[1].term, "common");
}

#[test]
fn persisted_vector_search_filters_allowed_candidates() {
    let results = search_persisted_vectors(
        &BTreeMap::from([(1, vec![9, 0]), (2, vec![0, 9])]),
        &[0, 2],
        &BTreeSet::from([2]),
        10,
        &DistanceMetric::default(),
    );

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].cell_id, 2);
}

#[test]
fn persisted_vector_search_skips_dimension_mismatches() {
    let results = search_persisted_vectors(
        &BTreeMap::from([(1, vec![9]), (2, vec![0, 9])]),
        &[0, 3],
        &BTreeSet::from([1, 2]),
        10,
        &DistanceMetric::default(),
    );

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].cell_id, 2);
    assert_eq!(results[0].score, 27);
}
