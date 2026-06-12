use super::*;
use crate::http::{append_query_param, path};

#[test]
fn aql_retrieve_builder_outputs_stable_statement() {
    let statement = Aql::retrieve_context("budget \"audit\"\nline", "investment_projects")
        .mode(AqlRetrievalMode::Balanced)
        .budget_tokens(2048)
        .limit_candidates(10)
        .where_clause(r#"space = project:investments AND status = "ready""#)
        .require_citations()
        .require_confidence_at_least("0.80")
        .require_source_trust_at_least("0.90")
        .require_freshness_seconds(86400)
        .build()
        .expect("retrieve builder should generate valid AQL");

    assert_eq!(
        statement,
        concat!(
            "RETRIEVE CONTEXT FOR TASK \"budget \\\"audit\\\"\\nline\" ",
            "IN BRAIN investment_projects USING MODE balanced BUDGET 2048 TOKENS ",
            "LIMIT 10 CANDIDATES WHERE space = project:investments AND status = \"ready\" ",
            "REQUIRE citations REQUIRE confidence >= 0.80 REQUIRE source_trust >= 0.90 ",
            "REQUIRE freshness <= 86400 SECONDS;"
        )
    );
}

#[test]
fn aql_verify_and_remember_builders_output_stable_statements() {
    let verify = Aql::verify_fact("Solar Plant budget is 1.2B KZT", "investment_projects")
        .build()
        .expect("verify builder should generate valid AQL");
    let remember = Aql::remember(
        "Use conservative budget assumptions",
        "project:investments",
        "decision",
    )
    .ttl_seconds(3600)
    .build()
    .expect("remember builder should generate valid AQL");

    assert_eq!(
        verify,
        "VERIFY FACT \"Solar Plant budget is 1.2B KZT\" IN BRAIN investment_projects;"
    );
    assert_eq!(
        remember,
        concat!(
            "REMEMBER \"Use conservative budget assumptions\" ",
            "IN SCOPE project:investments AS TYPE decision TTL 3600 SECONDS;"
        )
    );
}

#[test]
fn aql_builders_reject_invalid_identifiers_and_empty_fragments() {
    assert!(Aql::verify_fact("x", "tenant alpha").build().is_err());
    assert!(Aql::remember("x", "project:investments", "bad type")
        .build()
        .is_err());
    assert!(Aql::retrieve_context("x", "brain")
        .where_clause("  ")
        .build()
        .is_err());
    assert!(Aql::retrieve_context("x", "brain")
        .require_confidence_at_least("0")
        .build()
        .is_err());
}

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
fn path_encodes_auto_search_routing_contract() {
    let value = path(
        "/v1/search",
        &[
            ("scope", "project:investments"),
            ("mode", "auto"),
            ("q", "solar budget"),
            ("vector", "1,2,3"),
            ("limit", "10"),
        ],
    );
    assert_eq!(
        value,
        "/v1/search?scope=project%3Ainvestments&mode=auto&q=solar+budget&vector=1%2C2%2C3&limit=10"
    );
}

#[test]
fn search_explain_path_encodes_hybrid_contract() {
    let value = path(
        "/v1/search/explain",
        &[
            ("scope", "project:investments"),
            ("mode", "hybrid"),
            ("q", "solar budget"),
            ("vector", "1,2,3"),
            ("limit", "10"),
        ],
    );
    assert_eq!(
        value,
        "/v1/search/explain?scope=project%3Ainvestments&mode=hybrid&q=solar+budget&vector=1%2C2%2C3&limit=10"
    );
}

#[test]
fn context_export_paths_are_wire_stable() {
    let prompt = path(
        "/v1/context",
        &[("scope", "project:investments"), ("format", "prompt")],
    );
    let markdown = path(
        "/v1/context",
        &[("scope", "project:investments"), ("format", "markdown")],
    );
    assert_eq!(
        prompt,
        "/v1/context?scope=project%3Ainvestments&format=prompt"
    );
    assert_eq!(
        markdown,
        "/v1/context?scope=project%3Ainvestments&format=markdown"
    );
}

#[test]
fn typed_context_response_decodes_source_ref_url() {
    let value = serde_json::json!({
        "schema_version": "context_pack.v1",
        "token_budget_tokens": 100,
        "estimated_tokens": 10,
        "truncated": false,
        "citations_required": true,
        "answerability_q16": 65535,
        "conflict_visibility_q16": 0,
        "visible_conflict_count": 0,
        "cells": [{
            "cell_id": 1,
            "estimated_tokens": 10,
            "citation": "ifc:project-1",
            "payload_text": "scope=project:investments\nstatus=ready\nsource_id=ifc:project-1\nbody",
            "explain": null,
            "source_ref": {
                "source_id": "ifc:project-1",
                "source_url": "https://example.test/projects/1",
                "document_id": "doc-1",
                "page": 3,
                "cell_range": "chunk-1",
                "json_path": null,
                "confidence_q16": 60000
            }
        }],
        "anomalies": []
    });

    let response: ContextPackResponse =
        serde_json::from_value(value).expect("context response should decode");
    assert_eq!(response.answerability_q16, 65535);
    assert_eq!(response.conflict_visibility_q16, 0);
    assert_eq!(response.visible_conflict_count, 0);
    let source_ref = response.cells[0]
        .source_ref
        .as_ref()
        .expect("source ref should decode");
    assert_eq!(
        source_ref.source_url.as_deref(),
        Some("https://example.test/projects/1")
    );
    assert_eq!(source_ref.confidence_q16, 60_000);
}

#[test]
fn typed_ingest_response_decodes_validation_report() {
    let value = serde_json::json!({
        "rows_ingested": 0,
        "chunks_ingested": 1,
        "facts_ingested": 0,
        "first_cell_id": 42,
        "job_id": 7,
        "validation_report": {
            "cells_seen": 1,
            "warnings": [],
            "skipped_items": [],
            "source_refs": [{
                "cell_id": 42,
                "chunk_id": "memo.md#chunk-0001",
                "has_source_ref": true,
                "source_id": "memo.md",
                "document_id": "memo.md",
                "confidence_q16": 32768
            }]
        }
    });

    let response: IngestResponse =
        serde_json::from_value(value).expect("ingest response should decode");

    assert_eq!(response.validation_report.cells_seen, 1);
    assert_eq!(
        response.validation_report.source_refs[0]
            .chunk_id
            .as_deref(),
        Some("memo.md#chunk-0001")
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
        ("unknown_field", ErrorCode::UnknownField),
        ("unsupported_operator", ErrorCode::UnsupportedOperator),
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
        "routing": {
            "requested_mode": "auto",
            "selected_strategy": "vector_ann",
            "reason": "auto_vector_available_without_text",
            "text_available": false,
            "vector_available": true
        },
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
    let routing = response.routing.expect("routing should decode");
    assert_eq!(routing.requested_mode, "auto");
    assert_eq!(routing.selected_strategy, "vector_ann");
    assert_eq!(routing.reason, "auto_vector_available_without_text");
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
fn typed_search_explain_response_decodes_contribution_contract() {
    let value = serde_json::json!({
        "query_terms": ["budget"],
        "search_mode": "hybrid",
        "routing": {
            "requested_mode": "hybrid",
            "selected_strategy": "hybrid",
            "reason": "explicit_hybrid_mode",
            "text_available": true,
            "vector_available": true
        },
        "results": [{
            "cell_id": 1,
            "rank": 1,
            "score": 32786,
            "lexical_score": 42,
            "vector_score": 100,
            "lexical_contribution_q16": 19383,
            "vector_contribution_q16": 46152,
            "fusion_rank_score": 32786,
            "matched_terms": ["budget"],
            "matched_fields": ["title", "body_text", "vector"],
            "term_contributions": [{
                "term": "budget",
                "term_frequency": 2,
                "score": 42
            }],
            "contribution_summary": "hybrid rrf_score=32786 lexical_score=42 vector_score=100",
            "payload_preview": "scope=default\nstatus=ready\nbudget"
        }]
    });

    let response: SearchExplainResponse =
        serde_json::from_value(value).expect("search explain response should decode");

    assert_eq!(response.search_mode, "hybrid");
    assert_eq!(
        response
            .routing
            .as_ref()
            .map(|route| route.selected_strategy.as_str()),
        Some("hybrid")
    );
    assert_eq!(response.query_terms, vec!["budget"]);
    let item = &response.results[0];
    assert_eq!(item.rank, 1);
    assert_eq!(item.matched_terms, vec!["budget"]);
    assert_eq!(item.matched_fields, vec!["title", "body_text", "vector"]);
    assert_eq!(item.term_contributions[0].term_frequency, 2);
    assert_eq!(item.fusion_rank_score, 32786);
    assert!(item.contribution_summary.contains("hybrid rrf_score="));
}

#[test]
fn typed_aql_explain_response_decodes_contract() {
    let value = serde_json::json!({
        "cells": [],
        "explain": {
            "task": "budget",
            "brain_id": 1,
            "selected_mode": "balanced",
            "logical_plan": {
                "nodes": [{
                    "id": 0,
                    "kind": "scan",
                    "detail": "brain_id=1 predicate=bitmap_candidates",
                    "permission_predicate": null
                }],
                "policy_complete": false
            },
            "policy_rewritten_plan": {
                "nodes": [{
                    "id": 0,
                    "kind": "scan",
                    "detail": "brain_id=1 predicate=bitmap_candidates",
                    "permission_predicate": "agent_allowed"
                }],
                "policy_complete": true
            },
            "bitmap_plan": "BitmapProgram(max_stack_depth=3)\n0000: PushAgentAllowed",
            "bitmap_ops": ["PushAgentAllowed", "PushLive", "And"],
            "filters": [{
                "kind": "where",
                "expression": "status = \"ready\""
            }],
            "candidate_counts": {
                "universe": 2,
                "agent_allowed": 2,
                "live": 2,
                "after_bitmap": 1,
                "after_quality": 1,
                "returned_limit": 1
            },
            "candidate_limit": 10,
            "budget_tokens": 2048,
            "citations_required": false
        }
    });

    let response: AqlResponse =
        serde_json::from_value(value).expect("aql explain response should decode");
    assert!(response.cells.is_empty());
    let explain = response.explain.expect("explain report should decode");
    assert_eq!(explain.selected_mode, "balanced");
    assert_eq!(explain.candidate_counts.after_bitmap, 1);
    assert_eq!(explain.filters[0].kind, "where");
    assert!(!explain.logical_plan.policy_complete);
    assert!(explain.policy_rewritten_plan.policy_complete);
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

#[test]
fn typed_hnsw_no_fallback_profile_response_decodes_contract() {
    let value = serde_json::json!({
        "configured": true,
        "rollout_enabled": true,
        "min_recall_q16": 65535,
        "require_upper_layers": true
    });

    let response: HnswNoFallbackProfileResponse =
        serde_json::from_value(value).expect("profile response should decode");

    assert!(response.configured);
    assert_eq!(response.min_recall_q16, Some(65535));
    assert_eq!(response.require_upper_layers, Some(true));
}
