use super::support::*;

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
