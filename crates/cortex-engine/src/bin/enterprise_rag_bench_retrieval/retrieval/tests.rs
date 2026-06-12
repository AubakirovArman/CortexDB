use std::collections::{BTreeMap, BTreeSet};

use cortex_storage::indexes::LexicalIndex;

use super::scoring::candidate_doc_maps;
use super::*;

#[test]
fn candidate_mapping_uses_one_based_candidate_ordinal() {
    let uuid_index = BTreeMap::from([
        (
            "doc-a".to_owned(),
            "confluence/product-docs/product-overview/a.json".to_owned(),
        ),
        (
            "doc-b".to_owned(),
            "confluence/sales-enablement/b.json".to_owned(),
        ),
    ]);
    let doc_lengths = BTreeMap::from([(1, 2), (2, 3), (0, 1)]);

    let (mapped, sources, paths) = candidate_doc_maps(&uuid_index, &doc_lengths);

    assert_eq!(mapped.get(&1), Some(&"doc-a".to_owned()));
    assert_eq!(mapped.get(&2), Some(&"doc-b".to_owned()));
    assert_eq!(sources.get(&1), Some(&"confluence".to_owned()));
    assert_eq!(sources.get(&2), Some(&"confluence".to_owned()));
    assert_eq!(
        paths.get(&1),
        Some(&"confluence/product-docs/product-overview/a.json".to_owned())
    );
    assert_eq!(
        paths.get(&2),
        Some(&"confluence/sales-enablement/b.json".to_owned())
    );
    assert!(!mapped.contains_key(&0));
}

#[test]
fn cached_lexical_search_returns_ranked_doc_ids() {
    let uuid_index = BTreeMap::from([
        ("doc-a".to_owned(), "a.json".to_owned()),
        ("doc-b".to_owned(), "b.json".to_owned()),
    ]);
    let lexical = LexicalIndex {
        terms: BTreeMap::from([("budget".to_owned(), BTreeSet::from([1, 2]))]),
        doc_lengths: BTreeMap::from([(1, 4), (2, 4)]),
        term_frequencies: BTreeMap::from([("budget".to_owned(), BTreeMap::from([(1, 1), (2, 4)]))]),
        ..LexicalIndex::default()
    };
    let index = BenchmarkRetrievalIndex::from_lexical(lexical, &uuid_index);

    assert_eq!(
        index.search_doc_ids("budget", &[], 2),
        vec!["doc-b", "doc-a"]
    );
}

#[test]
fn source_type_filter_is_used_before_global_fill() {
    let uuid_index = BTreeMap::from([
        ("doc-a".to_owned(), "slack/a.json".to_owned()),
        ("doc-b".to_owned(), "github/b.json".to_owned()),
    ]);
    let lexical = LexicalIndex {
        terms: BTreeMap::from([("budget".to_owned(), BTreeSet::from([1, 2]))]),
        doc_lengths: BTreeMap::from([(1, 4), (2, 4)]),
        term_frequencies: BTreeMap::from([("budget".to_owned(), BTreeMap::from([(1, 4), (2, 1)]))]),
        ..LexicalIndex::default()
    };
    let index = BenchmarkRetrievalIndex::from_lexical(lexical, &uuid_index);

    assert_eq!(
        index.search_doc_ids("budget", &["github".to_owned()], 2),
        vec!["doc-b", "doc-a"]
    );
}

#[test]
fn overview_queries_get_company_context_expansion() {
    let uuid_index = BTreeMap::from([
        (
            "doc-a".to_owned(),
            "confluence/company-overview.json".to_owned(),
        ),
        ("doc-b".to_owned(), "slack/runtime-incident.json".to_owned()),
    ]);
    let lexical = LexicalIndex {
        terms: BTreeMap::from([
            ("company".to_owned(), BTreeSet::from([1])),
            ("overview".to_owned(), BTreeSet::from([1])),
            ("platform".to_owned(), BTreeSet::from([1])),
            ("incident".to_owned(), BTreeSet::from([2])),
        ]),
        doc_lengths: BTreeMap::from([(1, 8), (2, 8)]),
        term_frequencies: BTreeMap::from([
            ("company".to_owned(), BTreeMap::from([(1, 1)])),
            ("overview".to_owned(), BTreeMap::from([(1, 1)])),
            ("platform".to_owned(), BTreeMap::from([(1, 1)])),
            ("incident".to_owned(), BTreeMap::from([(2, 1)])),
        ]),
        ..LexicalIndex::default()
    };
    let index = BenchmarkRetrievalIndex::from_lexical(lexical, &uuid_index);

    assert_eq!(
        index.search_doc_ids("What is Redwood Inference's mission statement?", &[], 2),
        vec!["doc-a"]
    );
}

#[test]
fn non_overview_queries_do_not_get_company_context_expansion() {
    let uuid_index = BTreeMap::from([(
        "doc-a".to_owned(),
        "confluence/company-overview.json".to_owned(),
    )]);
    let lexical = LexicalIndex {
        terms: BTreeMap::from([("company".to_owned(), BTreeSet::from([1]))]),
        doc_lengths: BTreeMap::from([(1, 8)]),
        term_frequencies: BTreeMap::from([("company".to_owned(), BTreeMap::from([(1, 1)]))]),
        ..LexicalIndex::default()
    };
    let index = BenchmarkRetrievalIndex::from_lexical(lexical, &uuid_index);

    assert!(index
        .search_doc_ids("What timeout changed for SDK retries?", &[], 2)
        .is_empty());
}

#[test]
fn overview_queries_boost_corpus_metadata_paths() {
    let uuid_index = BTreeMap::from([
        (
            "doc-a".to_owned(),
            "confluence/product-docs/product-overview/platform-brief.json".to_owned(),
        ),
        (
            "doc-b".to_owned(),
            "confluence/eng-serving-runtime/runtime-architecture/runtime-notes.json".to_owned(),
        ),
        (
            "doc-c".to_owned(),
            "slack/support/random-customer-thread.json".to_owned(),
        ),
    ]);
    let lexical = LexicalIndex {
        terms: BTreeMap::from([("random".to_owned(), BTreeSet::from([3]))]),
        doc_lengths: BTreeMap::from([(1, 8), (2, 8), (3, 8)]),
        term_frequencies: BTreeMap::from([("random".to_owned(), BTreeMap::from([(3, 1)]))]),
        ..LexicalIndex::default()
    };
    let index = BenchmarkRetrievalIndex::from_lexical(lexical, &uuid_index);

    assert_eq!(
        index.search_doc_ids(
            "Which serving-runtime optimizations are part of Redwood's inference engine design?",
            &[],
            2,
        ),
        vec!["doc-b", "doc-a"]
    );
}

#[test]
fn non_overview_queries_do_not_boost_metadata_paths() {
    let uuid_index = BTreeMap::from([(
        "doc-a".to_owned(),
        "confluence/product-docs/product-overview/platform-brief.json".to_owned(),
    )]);
    let lexical = LexicalIndex {
        terms: BTreeMap::new(),
        doc_lengths: BTreeMap::from([(1, 8)]),
        term_frequencies: BTreeMap::new(),
        ..LexicalIndex::default()
    };
    let index = BenchmarkRetrievalIndex::from_lexical(lexical, &uuid_index);

    assert!(index
        .search_doc_ids("What timeout changed for SDK retries?", &[], 2)
        .is_empty());
}

#[test]
fn metadata_index_can_answer_overview_without_lexical_index() {
    let uuid_index = BTreeMap::from([
        (
            "doc-a".to_owned(),
            "confluence/product-docs/product-overview/platform-brief.json".to_owned(),
        ),
        (
            "doc-b".to_owned(),
            "slack/support/random-customer-thread.json".to_owned(),
        ),
    ]);
    let index = BenchmarkMetadataIndex::from_uuid_index(&uuid_index);

    assert_eq!(
        index.search_doc_ids("What is Redwood Inference's business model?", 2),
        vec!["doc-a"]
    );
    assert!(index
        .search_doc_ids("What timeout changed for SDK retries?", 2)
        .is_empty());
}
