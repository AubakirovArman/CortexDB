use cortex_engine::{mean_reciprocal_rank_q16, Bm25Index, TextAnalyzer};

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
