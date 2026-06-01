use super::*;
use crate::http::{append_query_param, path};

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
fn tenant_query_param_is_appended_to_existing_queries() {
    let value = append_query_param("/v1/stats?limit=10", "tenant", "tenant:alpha");
    assert_eq!(value, "/v1/stats?limit=10&tenant=tenant%3Aalpha");
}

#[test]
fn client_with_tenant_scopes_requests() {
    let client = CortexDbClient::new("http://127.0.0.1:8181").with_tenant("tenant:alpha");
    assert_eq!(
        client.url("/v1/stats"),
        "http://127.0.0.1:8181/v1/stats?tenant=tenant%3Aalpha"
    );
    assert_eq!(
        client.url("/v1/search?scope=project%3Ainvestments"),
        "http://127.0.0.1:8181/v1/search?scope=project%3Ainvestments&tenant=tenant%3Aalpha"
    );
}

#[test]
fn vector_algorithm_is_wire_stable() {
    assert_eq!(VectorAlgorithm::Ann.as_str(), "ann");
    assert_eq!(VectorAlgorithm::Exact.as_str(), "exact");
}

#[test]
fn error_code_decodes_full_core_alpha_taxonomy() {
    let codes = [
        ("not_found", ErrorCode::NotFound),
        ("bad_request", ErrorCode::BadRequest),
        ("unauthorized", ErrorCode::Unauthorized),
        ("forbidden", ErrorCode::Forbidden),
        ("payload_too_large", ErrorCode::PayloadTooLarge),
        ("rate_limited", ErrorCode::RateLimited),
        ("service_unavailable", ErrorCode::ServiceUnavailable),
        ("internal", ErrorCode::Internal),
        ("invalid_aql", ErrorCode::InvalidAql),
        ("permission_denied", ErrorCode::PermissionDenied),
        ("database_busy", ErrorCode::DatabaseBusy),
        ("storage_corruption", ErrorCode::StorageCorruption),
        ("invalid_tenant", ErrorCode::InvalidTenant),
    ];

    for (wire, expected) in codes {
        let value = serde_json::json!({
            "code": wire,
            "error": wire,
            "message": "message"
        });
        let response: ErrorResponse =
            serde_json::from_value(value).expect("error response should decode");
        assert_eq!(response.code, expected);
        assert_eq!(response.error, wire);
    }
}

#[test]
fn ann_evaluation_path_matches_http_api_contract() {
    let value = path(
        "/v1/search/ann-evaluate",
        &[
            ("scope", "project:investments"),
            ("vector", "1,2,3"),
            ("limit", "20"),
        ],
    );
    assert_eq!(
        value,
        "/v1/search/ann-evaluate?scope=project%3Ainvestments&vector=1%2C2%2C3&limit=20"
    );
}

#[test]
fn typed_search_response_decodes_ann_report_contract() {
    let value = serde_json::json!({
        "search_mode": "vector_ann",
        "ann_report": {
            "path": "exact_fallback",
            "fallback_reason": "no_persisted_segments",
            "fallback_performed": true,
            "requested_limit": 20,
            "allowed_candidates": 1,
            "graph_nodes": 0,
            "returned_candidates": 1,
            "recall_q16": null,
            "min_recall_q16": null,
            "hnsw_ef_construction": 64,
            "require_slo": true,
            "production_safe": false,
            "slo_violations": ["no_persisted_segments"]
        },
        "no_fallback_decision": {
            "allowed": false,
            "reasons": ["not_hnsw_graph_path", "fallback_occurred"]
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
    let decision = response
        .no_fallback_decision
        .expect("no-fallback decision should decode");

    assert_eq!(response.search_mode, "vector_ann");
    assert_eq!(response.results[0].cell_id, 1);
    assert_eq!(
        report.fallback_reason.as_deref(),
        Some("no_persisted_segments")
    );
    assert_eq!(report.recall_q16, None);
    assert_eq!(report.min_recall_q16, None);
    assert_eq!(report.hnsw_ef_construction, 64);
    assert!(report.fallback_performed);
    assert!(report.require_slo);
    assert!(!report.production_safe);
    assert_eq!(report.slo_violations, vec!["no_persisted_segments"]);
    assert!(!decision.allowed);
    assert_eq!(decision.reasons[0], "not_hnsw_graph_path");
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

#[test]
fn typed_ann_evaluation_response_decodes_contract() {
    let value = serde_json::json!({
        "available": true,
        "reason": null,
        "ann_report": {
            "path": "hnsw_graph",
            "fallback_reason": null,
            "fallback_performed": false,
            "requested_limit": 20,
            "allowed_candidates": 2,
            "graph_nodes": 2,
            "returned_candidates": 2,
            "recall_q16": 65535,
            "min_recall_q16": 65535,
            "hnsw_ef_construction": 128,
            "require_slo": true,
            "production_safe": true,
            "slo_violations": []
        },
        "no_fallback_decision": {
            "allowed": true,
            "reasons": []
        },
        "exact_top_k": [2, 1],
        "ann_top_k": [2, 1],
        "overlap_count": 2,
        "recall_q16": 65535
    });

    let response: AnnEvaluationResponse =
        serde_json::from_value(value).expect("ann evaluation response should decode");

    assert!(response.available);
    assert_eq!(response.recall_q16, 65535);
    assert_eq!(response.exact_top_k, vec![2, 1]);
    let report = response.ann_report.expect("report");
    assert_eq!(report.path.as_str(), "hnsw_graph");
    assert_eq!(report.recall_q16, Some(65535));
    assert_eq!(report.min_recall_q16, Some(65535));
    assert_eq!(report.hnsw_ef_construction, 128);
    assert!(report.require_slo);
    assert!(report.production_safe);
    assert!(report.slo_violations.is_empty());
    assert!(response.no_fallback_decision.expect("decision").allowed);
}
