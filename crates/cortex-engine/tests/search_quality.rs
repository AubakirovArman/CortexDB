use cortex_engine::{mean_reciprocal_rank_q16, Bm25Index, Language, TextAnalyzer};

#[test]
fn text_analyzer_applies_field_weights_and_stopwords() {
    let analyzer = TextAnalyzer::default().with_stopwords(["draft".to_owned()]);
    let terms = analyzer.weighted_terms([("title", "Budget draft"), ("body", "budget workflow")]);

    assert_eq!(terms.get("budget"), Some(&7));
    assert_eq!(terms.get("workflow"), Some(&1));
    assert!(!terms.contains_key("draft"));
}

#[test]
fn bm25_quality_fixture_has_perfect_mrr_for_golden_queries() {
    let analyzer = TextAnalyzer::default();
    let mut index = Bm25Index::default();
    analyzer.add_document_fields(
        &mut index,
        1,
        [
            ("title", "investment budget"),
            ("body", "ABC project budget approved"),
        ],
    );
    analyzer.add_document_fields(
        &mut index,
        2,
        [
            ("title", "workflow incident"),
            ("body", "pipeline error log"),
        ],
    );

    let mrr =
        mean_reciprocal_rank_q16(&index, &[("budget approved", 1), ("pipeline error", 2)], 10);

    assert_eq!(mrr, 65_535);
}

#[test]
fn language_analyzer_packs_apply_stopwords_and_light_stemming() {
    let english = TextAnalyzer::for_language(Language::English);
    let terms = english.weighted_terms([("body", "the approved budgets approving")]);
    assert!(!terms.contains_key("the"));
    assert_eq!(terms.get("budget"), Some(&1));

    let russian = TextAnalyzer::for_language(Language::Russian);
    let terms = russian.weighted_terms([("body", "бюджеты и проект")]);
    assert!(terms.contains_key("бюджет"));
    assert!(!terms.contains_key("и"));

    let kazakh = TextAnalyzer::for_language(Language::Kazakh);
    let terms = kazakh.weighted_terms([("body", "жобалар және бюджет")]);
    assert!(terms.contains_key("жоба"));
    assert!(!terms.contains_key("және"));
}
