use std::collections::BTreeMap;
use std::fs;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use cortex_core::CellId;
use cortex_engine::search::{
    SearchDiversityDiagnostics, SearchMode, SearchQuery, SearchQueryIntent,
};
use cortex_engine::Database;
use serde_json::json;

use super::{
    bench_view, body_vector_from_payload, build_benchmark_aql, build_payload, doc_id_from_payload,
    doc_id_to_cell_id, inferred_source_types_from_query, load_document_vectors,
    load_external_prefilter_retrieval, load_id_vectors, merge_prefilter_candidates,
    parse_query_vector, parse_status_kib, payload_has_vector, quote_aql_string,
    reject_oracle_fields, select_diverse_prefilter_candidates, source_prefilter_payloads,
    throughput_per_sec, vector_dot_score, BenchmarkSearchIndex, DiversityRunMetrics,
    ExternalPrefilterRetrieval, PrefilterCandidate, RunLogger, SourcePayloadPrefilter,
    ENGINE_PREFILTER_STRONG_EVIDENCE_SCORE,
};

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
fn source_payload_prefilter_reads_source_docs_without_persisted_index() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("cortexdb-source-prefilter-{unique}"));
    let sources = root.join("sources");
    fs::create_dir_all(sources.join("slack")).expect("create source dir");
    fs::write(
        sources.join("slack/doc-a.json"),
        json!({
            "title_field_name": "title",
            "content_field_names": ["body"],
            "title": "Alpha rollout",
            "body": "The alpha rollout mentions cache warmup and deployment status."
        })
        .to_string(),
    )
    .expect("write doc-a");
    fs::write(
        sources.join("slack/doc-b.json"),
        json!({
            "title_field_name": "title",
            "content_field_names": ["body"],
            "title": "Beta rollout",
            "body": "The beta rollout mentions source-payload prefiltering."
        })
        .to_string(),
    )
    .expect("write doc-b");
    let uuid_index = BTreeMap::from([
        ("doc-a".to_owned(), "slack/doc-a.json".to_owned()),
        ("doc-b".to_owned(), "slack/doc-b.json".to_owned()),
    ]);
    let external = ExternalPrefilterRetrieval {
        by_question_id: BTreeMap::from([(
            "q1".to_owned(),
            vec!["doc-b".to_owned(), "doc-a".to_owned()],
        )]),
        rows: 1,
    };
    let mut source_payloads =
        SourcePayloadPrefilter::new(&uuid_index, sources).expect("source prefilter");
    let mut document_vectors = super::DocumentVectorLookup::empty();
    let output = source_prefilter_payloads(
        &mut source_payloads,
        &mut document_vectors,
        &external,
        "q1",
        SearchQuery {
            text: "Which rollout mentions prefiltering?",
            vector: None,
            limit: 2,
            mode: SearchMode::HybridRerank,
        },
        2,
        None,
    )
    .expect("source prefilter output");
    let ids = output
        .payloads
        .iter()
        .filter_map(|payload| doc_id_from_payload(payload))
        .collect::<Vec<_>>();

    assert_eq!(ids, vec!["doc-b".to_owned(), "doc-a".to_owned()]);
    assert!(output.diversity_diagnostics.is_some());
    fs::remove_dir_all(root).ok();
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

#[test]
fn parses_query_vector_from_string_or_array() {
    assert_eq!(
        parse_query_vector(&json!({"vector": "1,2,-3"})),
        Some(vec![1, 2, -3])
    );
    assert_eq!(
        parse_query_vector(&json!({"vector": [4, 5, 6]})),
        Some(vec![4, 5, 6])
    );
}

#[test]
fn parses_query_vector_from_float_embedding_array() {
    assert_eq!(
        parse_query_vector(&json!({"vector": [0.0, 0.5, -0.5, 1.2, -1.2]})),
        Some(vec![0, 16_384, -16_384, i16::MAX, i16::MIN])
    );
}

#[test]
fn rejects_query_vector_with_non_numeric_array_values() {
    assert_eq!(parse_query_vector(&json!({"vector": [0.1, null]})), None);
}

#[test]
fn loads_id_vectors_from_jsonl() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("cortexdb-vector-loader-{unique}.jsonl"));
    fs::write(
        &path,
        r#"{"doc_id":"doc-a","vector":[1,2,3]}"#.to_owned() + "\n",
    )
    .expect("write vector jsonl");

    let vectors = load_id_vectors(Some(&path), "doc_id").expect("load vectors");

    assert_eq!(vectors.get("doc-a"), Some(&vec![1, 2, 3]));
    fs::remove_file(path).ok();
}

#[test]
fn loads_document_vectors_lazily_from_jsonl_offsets() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!("cortexdb-document-vector-loader-{unique}.jsonl"));
    fs::write(
        &path,
        [
            r#"{"doc_id":"doc-a","vector":[0.0,0.5]}"#,
            r#"{"doc_id":"doc-b","vector":"1,-2,3"}"#,
        ]
        .join("\n")
            + "\n",
    )
    .expect("write document vector jsonl");

    let mut vectors = load_document_vectors(Some(&path)).expect("load document vectors");

    assert_eq!(vectors.len(), 2);
    assert_eq!(vectors.get("doc-a").expect("doc-a"), Some(vec![0, 16_384]));
    assert_eq!(vectors.get("doc-b").expect("doc-b"), Some(vec![1, -2, 3]));
    assert_eq!(vectors.get("missing").expect("missing"), None);
    fs::remove_file(path).ok();
}

#[test]
fn extracts_doc_id_from_payload_metadata() {
    assert_eq!(
        doc_id_from_payload(b"scope=bench:enterprise_rag\ndoc_id=abc-123\n\nbody"),
        Some("abc-123".to_owned())
    );
}

#[test]
fn payload_vector_detector_requires_non_empty_vector_metadata() {
    assert!(payload_has_vector(b"doc_id=doc-1\nvector=1,-2,3\n\nbody"));
    assert!(!payload_has_vector(b"doc_id=doc-1\nvector=  \n\nbody"));
    assert!(!payload_has_vector(
        b"doc_id=doc-1\nembedding=1,-2,3\n\nbody"
    ));
}

#[test]
fn body_vector_helper_reads_body_vector_line() {
    assert_eq!(
        body_vector_from_payload(b"title_vector=1,0\nvector=0,1\n\nbody"),
        Some(vec![0, 1])
    );
    assert_eq!(body_vector_from_payload(b"title_vector=1,0\n\nbody"), None);
}

#[test]
fn vector_dot_score_clamps_negative_scores() {
    assert_eq!(vector_dot_score(&[2, -1], &[3, 4]), 2);
    assert_eq!(vector_dot_score(&[1], &[-10]), 0);
}

#[test]
fn prefilter_merge_preserves_lexical_head_then_promotes_reranked_candidates() {
    fn candidate(id: u64) -> PrefilterCandidate {
        PrefilterCandidate {
            cell_id: CellId(id),
            payload: format!("doc_id=doc-{id}\n\nbody").into_bytes(),
            score: id,
            lexical_score: id,
            vector_score: id,
            evidence_score: 0,
        }
    }
    let lexical = (1..=10).map(candidate).collect::<Vec<_>>();
    let reranked = [3, 20, 21, 22, 23, 24, 25, 26]
        .into_iter()
        .map(candidate)
        .collect::<Vec<_>>();

    let merged = merge_prefilter_candidates("Find invoice Q4", lexical, reranked, 10)
        .into_iter()
        .map(|candidate| candidate.cell_id.0)
        .collect::<Vec<_>>();

    assert_eq!(merged, vec![1, 2, 3, 4, 20, 21]);
}

#[test]
fn prefilter_merge_keeps_strong_evidence_tail_candidates() {
    fn candidate(id: u64, evidence_score: u32) -> PrefilterCandidate {
        PrefilterCandidate {
            cell_id: CellId(id),
            payload: format!("doc_id=doc-{id}\n\nbody").into_bytes(),
            score: id,
            lexical_score: id,
            vector_score: id,
            evidence_score,
        }
    }
    let lexical = (1..=10).map(|id| candidate(id, 0)).collect::<Vec<_>>();
    let reranked = [
        candidate(20, 0),
        candidate(21, 0),
        candidate(22, 0),
        candidate(23, 0),
        candidate(24, 0),
        candidate(25, 0),
        candidate(26, ENGINE_PREFILTER_STRONG_EVIDENCE_SCORE),
        candidate(27, ENGINE_PREFILTER_STRONG_EVIDENCE_SCORE + 1),
    ];

    let merged = merge_prefilter_candidates("Find invoice Q4", lexical, reranked.to_vec(), 10)
        .into_iter()
        .map(|candidate| candidate.cell_id.0)
        .collect::<Vec<_>>();

    assert_eq!(merged, vec![1, 2, 3, 4, 20, 21, 26, 27]);
}

#[test]
fn prefilter_hybrid_rerank_diversifies_metadata_clusters() {
    fn candidate(id: u64, score: u64, doc_id: &str, body: &str) -> PrefilterCandidate {
        PrefilterCandidate {
            cell_id: CellId(id),
            payload: format!("doc_id={doc_id}\n\n{body}").into_bytes(),
            score,
            lexical_score: score,
            vector_score: 0,
            evidence_score: 0,
        }
    }
    let candidates = vec![
        candidate(1, 100, "doc-a", "billing blocker owner deadline"),
        candidate(2, 99, "doc-a", "billing blocker owner deadline duplicate"),
        candidate(3, 80, "doc-b", "security blocker status dependency"),
    ];

    let selection = select_diverse_prefilter_candidates(
        candidates.clone(),
        candidates,
        "List all support, security, and billing requirements",
        2,
    );
    let selected = selection
        .candidates
        .iter()
        .map(|candidate| candidate.cell_id.0)
        .collect::<Vec<_>>();

    assert!(selection.diagnostics.diversity_enabled);
    assert_eq!(selection.diagnostics.input_candidates, 3);
    assert_eq!(selection.diagnostics.output_candidates, 2);
    assert_eq!(selection.diagnostics.skipped_candidates, 1);
    assert_eq!(selected, vec![1, 3]);
    assert!(selection.diagnostics.max_cluster_similarity_q16 > 0);
}

#[test]
fn prefilter_diversity_gate_preserves_requested_top_k_for_lookup_rows() {
    fn candidate(id: u64) -> PrefilterCandidate {
        PrefilterCandidate {
            cell_id: CellId(id),
            payload: format!("doc_id=doc-{id}\n\nlookup evidence body").into_bytes(),
            score: 100 - id,
            lexical_score: 100 - id,
            vector_score: 0,
            evidence_score: 0,
        }
    }
    let candidates = (1..=10).map(candidate).collect::<Vec<_>>();

    let selection =
        select_diverse_prefilter_candidates(candidates.clone(), candidates, "Find invoice Q4", 10);

    assert!(!selection.diagnostics.diversity_enabled);
    assert_eq!(selection.candidates.len(), 10);
    assert_eq!(selection.diagnostics.output_candidates, 10);
}

#[test]
fn prefilter_tail_limit_preserves_multi_evidence_queries_without_oracle_labels() {
    assert_eq!(super::prefilter_default_doc_limit("Find invoice Q4", 10), 6);
    assert_eq!(
        super::prefilter_default_doc_limit(
            "What caused the 429 spike and how do we verify it is not burning SLOs?",
            10,
        ),
        10
    );
    assert_eq!(
        super::prefilter_default_doc_limit(
            "List all Fireflies calls mentioning data residency",
            10
        ),
        10
    );
    assert_eq!(
        super::prefilter_default_doc_limit("What is the temporary kill switch name?", 10),
        7
    );
}

#[test]
fn prefilter_lexical_head_preserves_clean_external_order_for_complex_queries() {
    assert_eq!(
        super::prefilter_lexical_head_count("Find invoice Q4", 10),
        4
    );
    assert_eq!(
        super::prefilter_lexical_head_count(
            "What caused the 429 spike and how do we verify it is not burning SLOs?",
            10,
        ),
        10
    );
    assert_eq!(
        super::prefilter_lexical_head_count(
            "List all Fireflies calls mentioning data residency",
            10
        ),
        10
    );
}

#[test]
fn prefilter_evidence_score_rewards_anchors_conditions_and_terms() {
    let score = super::prefilter_evidence_score(
        "Which PR #42 fixed AUTH-123 before 2026-05-01?",
        b"source=github\n\nAUTH-123 was fixed by PR #42 on 2026-04-30.",
    );

    assert!(score >= ENGINE_PREFILTER_STRONG_EVIDENCE_SCORE);
}

#[test]
fn reusable_index_bounded_hybrid_uses_vector_signal_inside_lexical_pool() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("cortexdb-bounded-hybrid-{unique}"));
    fs::create_dir_all(&dir).expect("create temp db dir");
    let mut db = Database::open(&dir).expect("open db");
    let uuid_index = BTreeMap::from([
        ("doc-a".to_owned(), "slack/a.json".to_owned()),
        ("doc-b".to_owned(), "slack/b.json".to_owned()),
    ]);
    db.put_cells(vec![
        (
            CellId(1),
            build_payload(
                "doc-a",
                "slack/a.json",
                "Shared",
                "shared topic alpha",
                Some(&[32_767, 0]),
            )
            .into_bytes(),
        ),
        (
            CellId(2),
            build_payload(
                "doc-b",
                "slack/b.json",
                "Shared",
                "shared topic beta",
                Some(&[0, 32_767]),
            )
            .into_bytes(),
        ),
    ])
    .expect("put cells");
    let logger = RunLogger::new(Instant::now(), None, None).expect("logger");
    let index =
        BenchmarkSearchIndex::load(&db, &uuid_index, &bench_view(), &logger).expect("index");
    let query_vector = [0, 32_767];
    let payloads = index.search_payloads(
        &db,
        SearchQuery {
            text: "shared topic",
            vector: Some(&query_vector),
            limit: 1,
            mode: SearchMode::Hybrid,
        },
        1,
        None,
    );

    assert_eq!(payloads.len(), 1);
    assert_eq!(doc_id_from_payload(&payloads[0]), Some("doc-b".to_owned()));
    drop(index);
    drop(db);
    fs::remove_dir_all(dir).ok();
}

#[test]
fn doc_id_to_cell_id_uses_ingest_order() {
    let uuid_index = BTreeMap::from([
        ("doc-a".to_owned(), "slack/a.json".to_owned()),
        ("doc-b".to_owned(), "gmail/b.json".to_owned()),
    ]);

    let mapped = doc_id_to_cell_id(&uuid_index).expect("mapping");

    assert_eq!(mapped.get("doc-a"), Some(&CellId(1)));
    assert_eq!(mapped.get("doc-b"), Some(&CellId(2)));
}

#[test]
fn benchmark_aql_uses_question_text_and_clean_filters() {
    let aql = build_benchmark_aql("What did \"Apollo\" ship?", None, 10);

    assert!(aql.contains("RETRIEVE CONTEXT FOR TASK"));
    assert!(aql.contains("\\\"Apollo\\\""));
    assert!(aql.contains("USING MODE balanced"));
    assert!(aql.contains("space = bench:enterprise_rag"));
    assert!(aql.contains("status = \"ready\""));
    assert!(aql.contains("type = \"document_block\""));
    assert!(aql.contains("LIMIT 10 CANDIDATES"));
    assert!(!aql.contains("question_type"));
    assert!(!aql.contains("source_types"));
    assert!(!aql.contains("expected_doc_ids"));
}

#[test]
fn benchmark_aql_can_carry_query_vector_without_gold_fields() {
    let aql = build_benchmark_aql("semantic lookup", Some(&vec![1, -2, 3]), 5);

    assert!(aql.contains("USING MODE semantic"));
    assert!(aql.contains("query_vector=1,-2,3"));
    assert!(aql.contains("LIMIT 5 CANDIDATES"));
}

#[test]
fn quote_aql_string_escapes_control_characters() {
    assert_eq!(
        quote_aql_string("a \"b\"\nnext\\tail"),
        r#""a \"b\"\nnext\\tail""#
    );
}

#[test]
fn reports_throughput_and_linux_status_memory_bytes() {
    assert_eq!(throughput_per_sec(10, Some(500.0)), 20.0);
    assert_eq!(throughput_per_sec(10, None), 0.0);
    assert_eq!(parse_status_kib(" 123 kB"), Some(125_952));
}
