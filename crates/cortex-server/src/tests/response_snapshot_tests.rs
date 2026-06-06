use crate::responses::{AnnMetricsResponse, AnnNoFallbackDecisionResponse};
use crate::responses::{
    CheckpointResponse, ContextPackAnomalyResponse, ContextPackCellResponse, ContextPackResponse,
    ErrorCode, ErrorResponse, EvidenceResponse, ExplainResponse, GuardResponse, HealthResponse,
    IngestResponse, LatencyHistogramResponse, MetricsResponse, NumericConflictResponse,
    PutCellResponse, ScoreComponentResponse, SearchResultResponse, SourceRefResponse,
    StatsResponse, ValidationResponse, VerificationReportResponse,
};
use cortex_engine::{IngestionSkippedItem, IngestionSourceRefReport, IngestionValidationReport};

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
        memtable_payload_bytes: 2048,
        estimated_memtable_bytes: 4096,
        estimated_index_bytes: 8192,
        estimated_context_pack_bytes: 16384,
        estimated_total_memory_bytes: 28672,
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
fn snapshot_metrics_response() {
    let resp = MetricsResponse {
        current_seq: 42,
        checkpoint_seq: 30,
        live_segments: 3,
        retired_segments: 1,
        memtable_cells: 12,
        memtable_versions: 15,
        memtable_payload_bytes: 2048,
        estimated_memtable_bytes: 4096,
        estimated_index_bytes: 8192,
        estimated_context_pack_bytes: 16384,
        estimated_total_memory_bytes: 28672,
        wal_size_bytes: 4096,
        wal_writer_records: 100,
        wal_writer_bytes: 8192,
        wal_writer_fsyncs: 10,
        wal_writer_batches: 5,
        backup_latest_age_seconds: 3600,
        ann_graph_nodes: 120,
        ann_total_edges: 540,
        ann_persisted_segments: 2,
        ann_has_checkpoint: true,
        ann_has_uncheckpointed_changes: false,
        ann_search_requests: 11,
        ann_fallbacks: 2,
        ann_no_fallback_requests: 4,
        ann_no_fallback_allowed: 3,
        ann_no_fallback_blocked: 1,
        ann_search_latency_ms: LatencyHistogramResponse {
            count: 11,
            sum_ms: 120,
            le_10_ms: 5,
            le_50_ms: 8,
            le_100_ms: 10,
            le_500_ms: 11,
            le_1000_ms: 11,
            gt_1000_ms: 0,
        },
        actor_queue_depth: 2,
        actor_queue_capacity: 16,
        request_count: 1000,
        request_rejected: 3,
        request_duration_ms_total: 120,
        request_id_client_provided: 700,
        request_id_generated: 300,
        validation_failures: 1,
        principal_quota_requests_allowed: 80,
        principal_quota_requests_rejected: 4,
        principal_quota_body_bytes_allowed: 4096,
        principal_quota_body_bytes_rejected: 512,
        principal_quota_queue_acquired: 70,
        principal_quota_queue_rejected: 2,
    };
    insta::assert_json_snapshot!(resp);
}

#[test]
fn snapshot_put_cell_response() {
    let resp = PutCellResponse { seq: 7, cell_id: 1 };
    insta::assert_json_snapshot!(resp);
}

#[test]
fn snapshot_ann_metrics_response() {
    let resp = AnnMetricsResponse {
        graph_nodes: 100,
        total_edges: 420,
        persisted_segments: 2,
        has_checkpoint: true,
        has_uncheckpointed_changes: false,
        deleted_vectors: 4,
        rebuild_count: 1,
    };
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
        answerability_q16: 65535,
        conflict_visibility_q16: 65535,
        visible_conflict_count: 1,
        cells: vec![ContextPackCellResponse {
            cell_id: 1,
            estimated_tokens: 120,
            citation: Some("report_q1.pdf#page=3".to_owned()),
            payload_text: "Solar Plant budget is 1.2B KZT".to_owned(),
            explain: Some(ExplainResponse {
                score: 95,
                matched_terms: vec!["budget".to_owned(), "solar".to_owned()],
                why_selected: "high lexical match".to_owned(),
                score_components: vec![
                    ScoreComponentResponse {
                        name: "base_bm25".to_owned(),
                        value: 80,
                        contribution: 80,
                        reason: "lexical relevance before bonuses and penalties".to_owned(),
                    },
                    ScoreComponentResponse {
                        name: "source_trust_bonus".to_owned(),
                        value: 10,
                        contribution: 10,
                        reason: "trusted source metadata increased the score".to_owned(),
                    },
                    ScoreComponentResponse {
                        name: "redundancy_penalty".to_owned(),
                        value: 0,
                        contribution: 0,
                        reason: "no redundancy penalty was applied".to_owned(),
                    },
                ],
                base_bm25: 80,
                source_trust_q16: 60_000,
                source_trust_category: "official".to_owned(),
                source_trust_bonus: 10,
                redundancy_penalty: 0,
            }),
            source_ref: Some(SourceRefResponse {
                source_id: "report_q1.pdf".to_owned(),
                source_url: None,
                document_id: Some("doc-1".to_owned()),
                page: Some(3),
                row: None,
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
                why_excluded: Some(
                    "excluded because estimated_tokens would exceed token_budget_tokens".to_owned(),
                ),
            },
            ContextPackAnomalyResponse {
                cell_id: None,
                code: "scope_mismatch".to_owned(),
                message: "Anomaly without cell association".to_owned(),
                why_excluded: None,
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
            source_trust_category: "official".to_owned(),
            citation: Some("report_q1.pdf#page=3".to_owned()),
            payload_text: "Solar Plant budget is 1.2B KZT".to_owned(),
        }],
        contradicting_evidence: vec![EvidenceResponse {
            cell_id: 2,
            matched_terms: 3,
            source_trust_q16: 65535,
            source_trust_category: "official".to_owned(),
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
            source_trust_category: "official".to_owned(),
            citation: Some("report_q1.pdf#page=3".to_owned()),
            payload_text: "Solar Plant budget is 1.2B KZT".to_owned(),
        }],
        contradicting: vec![EvidenceResponse {
            cell_id: 2,
            matched_terms: 3,
            source_trust_q16: 65535,
            source_trust_category: "official".to_owned(),
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
        routing: None,
        ann_report: None,
        no_fallback_decision: None,
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
        hnsw_max_neighbors: 0,
        hnsw_ef_search: 0,
        hnsw_ef_construction: 0,
        hnsw_layer_count: 0,
        upper_graph_edges: 0,
        require_slo: true,
        production_safe: false,
        slo_violations: vec!["visit_budget_exceeded".to_owned()],
    };
    insta::assert_json_snapshot!(resp);
}

#[test]
fn snapshot_ann_no_fallback_decision_response() {
    let resp = AnnNoFallbackDecisionResponse {
        allowed: false,
        reasons: vec!["fallback_enabled".to_owned(), "slo_not_required".to_owned()],
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
        validation_report: IngestionValidationReport {
            cells_seen: 0,
            processed_records: 0,
            skipped_records: 1,
            invalid_metadata_records: 0,
            warnings: Vec::new(),
            skipped_items: vec![IngestionSkippedItem {
                reason: "no_cells_emitted".to_owned(),
                input_ref: Some("ingest_text".to_owned()),
            }],
            source_refs: Vec::new(),
        },
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
        validation_report: IngestionValidationReport {
            cells_seen: 1,
            processed_records: 1,
            skipped_records: 0,
            invalid_metadata_records: 0,
            warnings: Vec::new(),
            skipped_items: Vec::new(),
            source_refs: vec![IngestionSourceRefReport {
                cell_id: 1,
                chunk_id: Some("memo.md#chunk-0001".to_owned()),
                has_source_ref: true,
                source_id: Some("memo.md".to_owned()),
                source_url: Some("https://example.test/memo.md".to_owned()),
                document_id: Some("memo.md".to_owned()),
                page: Some(3),
                row: Some(7),
                cell_range: Some("memo.md#chunk-0001".to_owned()),
                json_path: Some("$.items[0]".to_owned()),
                confidence_q16: Some(32768),
            }],
        },
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
