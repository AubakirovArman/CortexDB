use super::support::*;

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
