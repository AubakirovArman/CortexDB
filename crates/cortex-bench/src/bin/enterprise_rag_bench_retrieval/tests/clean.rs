use super::support::*;

#[test]
fn official_clean_rejects_question_type_and_source_types() {
    let rows = vec![json!({
        "question_id": "q1",
        "question": "What happened?",
        "question_type": "basic",
        "source_types": ["gmail"]
    })];

    let error = reject_oracle_fields(&rows).expect_err("oracle fields should fail");

    assert!(error.contains("question_type"));
    assert!(error.contains("source_types"));
}

#[test]
fn official_clean_accepts_clean_question_rows() {
    let rows = vec![json!({
        "question_id": "q1",
        "question": "What happened?"
    })];

    reject_oracle_fields(&rows).expect("clean rows should pass");
}

#[test]
fn diversity_run_metrics_json_groups_cluster_diagnostics_by_intent() {
    let mut metrics = DiversityRunMetrics::default();
    metrics.record(&SearchDiversityDiagnostics {
        intent: SearchQueryIntent::Completeness,
        diversity_enabled: true,
        lambda_q16: 36_864,
        input_candidates: 8,
        output_candidates: 5,
        skipped_candidates: 3,
        max_payload_similarity_q16: 12_000,
        max_cluster_similarity_q16: 65_535,
        selected_with_payload_similarity: 2,
        selected_with_cluster_similarity: 3,
    });

    let value = metrics.to_json();

    assert_eq!(value["reports"], 1);
    assert_eq!(value["diversity_enabled_questions"], 1);
    assert_eq!(value["skipped_candidates"], 3);
    assert_eq!(value["max_cluster_similarity_q16"], 65_535);
    assert_eq!(
        value["by_intent"]["completeness"]["selected_with_cluster_similarity"],
        3
    );
}

#[test]
fn loads_external_prefilter_retrieval_clean_rows() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("cortexdb-prefilter-clean-{unique}.jsonl"));
    fs::write(
            &path,
            [
                r#"{"question_id":"q1","question":"What happened?","answer":"","document_ids":["doc-a","doc-b","doc-a"]}"#,
                r#"{"question_id":"q1","question":"What happened?","answer":"","document_ids":["doc-c"]}"#,
            ]
            .join("\n")
                + "\n",
        )
        .expect("write prefilter jsonl");

    let retrieval = load_external_prefilter_retrieval(Some(&path))
        .expect("load prefilter")
        .unwrap();

    assert_eq!(retrieval.rows, 2);
    assert_eq!(
        retrieval.doc_ids("q1"),
        Some(["doc-a".to_owned(), "doc-b".to_owned(), "doc-c".to_owned()].as_slice())
    );
    fs::remove_file(path).ok();
}

#[test]
fn external_prefilter_rejects_oracle_fields() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("cortexdb-prefilter-oracle-{unique}.jsonl"));
    fs::write(
            &path,
            r#"{"question_id":"q1","question":"What happened?","document_ids":["doc-a"],"source_types":["gmail"]}"#,
        )
        .expect("write prefilter jsonl");

    let error =
        load_external_prefilter_retrieval(Some(&path)).expect_err("oracle field should fail");

    assert!(error.contains("forbidden oracle fields"));
    assert!(error.contains("source_types"));
    fs::remove_file(path).ok();
}

#[test]
fn external_prefilter_rejects_unknown_fields() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("cortexdb-prefilter-extra-{unique}.jsonl"));
    fs::write(
        &path,
        r#"{"question_id":"q1","question":"What happened?","document_ids":["doc-a"],"score":1}"#,
    )
    .expect("write prefilter jsonl");

    let error =
        load_external_prefilter_retrieval(Some(&path)).expect_err("unknown field should fail");

    assert!(error.contains("unsupported fields"));
    assert!(error.contains("score"));
    fs::remove_file(path).ok();
}

#[test]
fn external_prefilter_allows_empty_document_ids_for_fallback() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("cortexdb-prefilter-empty-{unique}.jsonl"));
    fs::write(
        &path,
        r#"{"question_id":"q1","question":"No answer?","answer":"","document_ids":[]}"#,
    )
    .expect("write prefilter jsonl");

    let retrieval = load_external_prefilter_retrieval(Some(&path))
        .expect("empty document ids should load")
        .unwrap();

    assert_eq!(retrieval.rows, 1);
    assert_eq!(retrieval.doc_ids("q1"), None);
    fs::remove_file(path).ok();
}

#[test]
fn infers_source_types_from_question_text_without_oracle_fields() {
    assert_eq!(
        inferred_source_types_from_query(
            "Which Slack thread mentioned the GitHub PR for the Google Drive file?"
        ),
        vec![
            "slack".to_owned(),
            "github".to_owned(),
            "google_drive".to_owned()
        ]
    );
    assert_eq!(
        inferred_source_types_from_query("Which team owns the rollout decision?"),
        Vec::<String>::new()
    );
    assert_eq!(
        inferred_source_types_from_query(
            "What was the high-percentile latency concern reported after a smoke test?"
        ),
        Vec::<String>::new()
    );
}
