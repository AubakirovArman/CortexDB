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
