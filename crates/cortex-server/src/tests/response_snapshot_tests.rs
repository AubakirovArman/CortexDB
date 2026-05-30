use crate::responses::{
    CheckpointResponse, ContextPackAnomalyResponse, ContextPackCellResponse, ContextPackResponse,
    ErrorCode, ErrorResponse, EvidenceResponse, ExplainResponse, GuardResponse, HealthResponse,
    IngestResponse, NumericConflictResponse, PutCellResponse, SearchResultResponse,
    SourceRefResponse, StatsResponse, ValidationResponse, VerificationReportResponse,
};

#[test]
fn snapshot_health_response() {
    let resp = HealthResponse {
        status: "ok".to_owned(),
        version: "v1".to_owned(),
        server_version: "0.1.0-core-alpha".to_owned(),
    };
    insta::assert_json_snapshot!(resp);
}

#[test]
fn snapshot_stats_response() {
    let resp = StatsResponse {
        current_seq: 42,
        checkpoint_seq: 30,
        live_segments: 3,
        retired_segments: 1,
        memtable_cells: 12,
        memtable_versions: 15,
        wal_size_bytes: 4096,
        wal_writer_records: 100,
        wal_writer_bytes: 8192,
        wal_writer_fsyncs: 10,
        wal_writer_batches: 5,
    };
    insta::assert_json_snapshot!(resp);
}

#[test]
fn snapshot_validation_response() {
    let resp = ValidationResponse {
        ok: true,
        manifest_ok: true,
        wal_ok: true,
        live_segments_checked: 3,
        bitmap_indexes_checked: 3,
        lexical_indexes_checked: 3,
        vector_indexes_checked: 0,
        hnsw_graphs_checked: 0,
        cells_checked: 12,
        wal_records_checked: 100,
        wal_safe_truncate_offset: 4096,
        errors: vec![],
    };
    insta::assert_json_snapshot!(resp);
}

#[test]
fn snapshot_put_cell_response() {
    let resp = PutCellResponse { seq: 7, cell_id: 1 };
    insta::assert_json_snapshot!(resp);
}

#[test]
fn snapshot_checkpoint_response() {
    let resp = CheckpointResponse {
        checkpoint_seq: 30,
        cells_flushed: 12,
    };
    insta::assert_json_snapshot!(resp);
}

#[test]
fn snapshot_context_pack_response() {
    let resp = ContextPackResponse {
        schema_version: "context_pack.v1",
        token_budget_tokens: 4000,
        estimated_tokens: 2500,
        truncated: false,
        citations_required: true,
        cells: vec![ContextPackCellResponse {
            cell_id: 1,
            estimated_tokens: 120,
            citation: Some("report_q1.pdf#page=3".to_owned()),
            payload_text: "Solar Plant budget is 1.2B KZT".to_owned(),
            explain: Some(ExplainResponse {
                score: 95,
                matched_terms: vec!["budget".to_owned(), "solar".to_owned()],
                why_selected: "high lexical match".to_owned(),
                base_bm25: 80,
                source_trust_bonus: 10,
                redundancy_penalty: 0,
            }),
            source_ref: Some(SourceRefResponse {
                source_id: "report_q1.pdf".to_owned(),
                document_id: Some("doc-1".to_owned()),
                page: Some(3),
                cell_range: None,
                json_path: None,
                confidence_q16: 65535,
            }),
        }],
        anomalies: vec![
            ContextPackAnomalyResponse {
                cell_id: Some(1),
                code: "token_overload".to_owned(),
                message: "Cell exceeds token budget".to_owned(),
            },
            ContextPackAnomalyResponse {
                cell_id: None,
                code: "scope_mismatch".to_owned(),
                message: "Anomaly without cell association".to_owned(),
            },
        ],
    };
    insta::assert_json_snapshot!(resp);
}

#[test]
fn snapshot_verification_report_response() {
    let resp = VerificationReportResponse {
        fact: "Solar Plant budget is 1.2B KZT".to_owned(),
        status: "mixed_evidence".to_owned(),
        verdict: "mixed_evidence".to_owned(),
        evidence: vec![EvidenceResponse {
            cell_id: 1,
            matched_terms: 3,
            source_trust_q16: 65535,
            citation: Some("report_q1.pdf#page=3".to_owned()),
            payload_text: "Solar Plant budget is 1.2B KZT".to_owned(),
        }],
        contradicting_evidence: vec![EvidenceResponse {
            cell_id: 2,
            matched_terms: 3,
            source_trust_q16: 65535,
            citation: Some("report_q2.pdf#page=5".to_owned()),
            payload_text: "Solar Plant budget is 1.4B KZT".to_owned(),
        }],
        guards: vec![
            GuardResponse {
                cell_id: Some(1),
                code: "numeric_mismatch".to_owned(),
                message: "Value mismatch detected".to_owned(),
            },
            GuardResponse {
                cell_id: None,
                code: "missing_citation".to_owned(),
                message: "Fact lacks source citation".to_owned(),
            },
        ],
        supporting: vec![EvidenceResponse {
            cell_id: 1,
            matched_terms: 3,
            source_trust_q16: 65535,
            citation: Some("report_q1.pdf#page=3".to_owned()),
            payload_text: "Solar Plant budget is 1.2B KZT".to_owned(),
        }],
        contradicting: vec![EvidenceResponse {
            cell_id: 2,
            matched_terms: 3,
            source_trust_q16: 65535,
            citation: Some("report_q2.pdf#page=5".to_owned()),
            payload_text: "Solar Plant budget is 1.4B KZT".to_owned(),
        }],
        numeric_conflicts: vec![NumericConflictResponse {
            metric: "budget".to_owned(),
            left: "1.2B KZT".to_owned(),
            right: "1.4B KZT".to_owned(),
        }],
    };
    insta::assert_json_snapshot!(resp);
}

#[test]
fn snapshot_search_result_response() {
    let resp = crate::responses::SearchResponse {
        search_mode: "keyword".to_owned(),
        ann_report: None,
        results: vec![SearchResultResponse {
            cell_id: 1,
            score: 150,
            lexical_score: 100,
            vector_score: 50,
            payload: "Solar Plant budget is 1.2B KZT".to_owned(),
        }],
    };
    insta::assert_json_snapshot!(resp);
}

#[test]
fn snapshot_ann_search_report_response() {
    let resp = crate::responses::AnnSearchReportResponse {
        path: "exact_fallback".to_owned(),
        fallback_reason: Some("visit_budget_exceeded".to_owned()),
        fallback_performed: true,
        requested_limit: 10,
        allowed_candidates: 100,
        graph_nodes: 100,
        returned_candidates: 10,
        visited_candidates: 64,
        max_visited_candidates: Some(64),
        recall_q16: Some(49_000),
        min_recall_q16: Some(49_151),
        require_slo: true,
        production_safe: false,
        slo_violations: vec!["visit_budget_exceeded".to_owned()],
    };
    insta::assert_json_snapshot!(resp);
}

#[test]
fn snapshot_ingest_response_empty() {
    let resp = IngestResponse {
        rows_ingested: 0,
        chunks_ingested: 0,
        facts_ingested: 0,
        first_cell_id: None,
        job_id: Some(1),
    };
    insta::assert_json_snapshot!(resp);
}

#[test]
fn snapshot_ingest_response_with_cells() {
    let resp = IngestResponse {
        rows_ingested: 3,
        chunks_ingested: 3,
        facts_ingested: 3,
        first_cell_id: Some(1),
        job_id: Some(1),
    };
    insta::assert_json_snapshot!(resp);
}

#[test]
fn snapshot_error_response() {
    let resp = ErrorResponse {
        code: ErrorCode::InvalidTenant,
        error: "invalid_tenant".to_owned(),
        message: "invalid tenant ID structure".to_owned(),
    };
    insta::assert_json_snapshot!(resp);
}
