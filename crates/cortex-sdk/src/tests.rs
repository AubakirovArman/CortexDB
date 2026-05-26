use super::*;
use crate::http::path;

#[test]
fn path_encodes_search_query_contract() {
    let value = path(
        "/v1/search",
        &[
            ("scope", "project:investments"),
            ("mode", "keyword"),
            ("q", "solar budget"),
            ("limit", "10"),
        ],
    );
    assert_eq!(
        value,
        "/v1/search?scope=project%3Ainvestments&mode=keyword&q=solar+budget&limit=10"
    );
}

#[test]
fn vector_algorithm_is_wire_stable() {
    assert_eq!(VectorAlgorithm::Ann.as_str(), "ann");
    assert_eq!(VectorAlgorithm::Exact.as_str(), "exact");
}

#[test]
fn typed_search_response_decodes_ann_report_contract() {
    let value = serde_json::json!({
        "search_mode": "vector_ann",
        "ann_report": {
            "path": "exact_fallback",
            "fallback_reason": "no_persisted_segments",
            "requested_limit": 20,
            "allowed_candidates": 1,
            "graph_nodes": 0,
            "returned_candidates": 1
        },
        "results": [{
            "cell_id": 1,
            "score": 42,
            "lexical_score": 0,
            "vector_score": 42,
            "payload": "scope=default\nstatus=ready\nhello"
        }]
    });

    let response: SearchResponse =
        serde_json::from_value(value).expect("search response should decode");
    let report = response.ann_report.expect("ann report should be present");

    assert_eq!(response.search_mode, "vector_ann");
    assert_eq!(response.results[0].cell_id, 1);
    assert_eq!(
        report.fallback_reason.as_deref(),
        Some("no_persisted_segments")
    );
}

#[test]
fn ingest_path_encodes_source_contract() {
    let value = path(
        "/v1/ingest/text",
        &[("scope", "project:investments"), ("source", "rust sdk")],
    );
    assert_eq!(
        value,
        "/v1/ingest/text?scope=project%3Ainvestments&source=rust+sdk"
    );
}
