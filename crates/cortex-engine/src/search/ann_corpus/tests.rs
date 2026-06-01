use super::*;

fn vectors() -> &'static str {
    r#"
{"candidate":1,"vector":[100,0,0,0]}
{"candidate":2,"vector":[96,4,0,0]}
{"candidate":3,"vector":[0,100,0,0]}
{"candidate":4,"vector":[0,96,4,0]}
"#
}

fn queries() -> &'static str {
    r#"
{"name":"axis-a","vector":[100,0,0,0],"limit":2}
{"name":"axis-b","vector":[0,100,0,0],"limit":2}
"#
}

fn ground_truth() -> &'static str {
    r#"
{"name":"axis-a","candidates":[1,2]}
{"name":"axis-b","candidates":[3,4]}
"#
}

#[test]
fn corpus_evaluation_uses_external_ground_truth() {
    let report = evaluate_ann_corpus(
        vectors(),
        queries(),
        ground_truth(),
        AnnCorpusOptions::default(),
    )
    .unwrap();

    assert!(report.passed, "{:?}", report.failures);
    assert_eq!(report.vector_count, 4);
    assert_eq!(report.query_count, 2);
    assert_eq!(report.min_observed_recall_q16, 65_535);
    assert_eq!(report.mean_mrr_q16, 65_535);
    assert_eq!(report.mean_ndcg_q16, 65_535);
    assert_eq!(report.exact_parity_q16, 65_535);
    assert_eq!(report.exact_parity_count, 2);
    assert_eq!(report.required_min_recall_q16, 49_151);
    assert_eq!(report.allowed_p95_latency_nanos, 100_000_000);
    assert_eq!(report.allowed_p99_latency_nanos, 200_000_000);
    assert!(report.p99_latency_nanos >= report.p95_latency_nanos);
    assert!(report.require_production_safe);
    assert!(report.queries.iter().all(|query| query.exact_parity));
}

#[test]
fn corpus_evaluation_rejects_missing_ground_truth() {
    let error = evaluate_ann_corpus(
        vectors(),
        queries(),
        r#"{"name":"axis-a","candidates":[1,2]}"#,
        AnnCorpusOptions::default(),
    )
    .unwrap_err();

    assert!(error.to_string().contains("missing ground truth"));
}

#[test]
fn corpus_evaluation_fails_low_recall_threshold() {
    let options = AnnCorpusOptions {
        min_recall_q16: 65_535,
        min_mean_recall_q16: 65_535,
        ..AnnCorpusOptions::default()
    };
    let report = evaluate_ann_corpus(
        vectors(),
        queries(),
        r#"
{"name":"axis-a","candidates":[3,4]}
{"name":"axis-b","candidates":[1,2]}
"#,
        options,
    )
    .unwrap();

    assert!(!report.passed);
    assert!(report
        .failures
        .iter()
        .any(|failure| failure.contains("recall")));
}
