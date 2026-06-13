use crate::cli_json_types::{
    CellResponse, CliAnnEvaluationResponse, CliAnnSearchReportResponse, CliAnnValidateResponse,
    CliStatsResponse, CliValidateResponse, CliVectorRebuildResponse, NumericConflictResponse,
    RememberResponse, SearchResponse, SearchResultResponse, SearchRoutingDecisionResponse,
    VerificationEvidenceResponse, VerificationResponse,
};
use cortex_engine::{
    ContextPack, Database, DatabaseSearchResult, RememberedCell, SearchRouteDecision, StorageStats,
    VectorRebuildReport, VerificationEvidence, VerificationReport, VerificationStatus,
};
use serde_json::to_string;

mod context;

use context::context_pack_response;

fn serialize_or_error<T: serde::Serialize>(value: &T) -> String {
    to_string(value).unwrap_or_else(|e| {
        to_string(&crate::cli_json_types::ErrorResponse {
            code: "internal".to_owned(),
            error: "internal_error".to_owned(),
            message: e.to_string(),
        })
        .unwrap_or_else(|_| {
            "{\"code\":\"internal\",\"error\":\"internal_error\",\"message\":\"serialization failed\"}".to_owned()
        })
    })
}

pub(crate) fn context_pack_to_json(pack: &ContextPack) -> String {
    serialize_or_error(&context_pack_response(pack))
}

pub(crate) fn verification_report_to_json(report: &VerificationReport, db: &Database) -> String {
    let numeric_conflicts = report
        .numeric_conflicts
        .iter()
        .map(|conflict| NumericConflictResponse {
            metric: conflict.metric.clone(),
            left: conflict.left.clone(),
            right: conflict.right.clone(),
        })
        .collect::<Vec<_>>();

    let response = VerificationResponse {
        verdict: verification_verdict(report.status).to_owned(),
        supporting: report
            .evidence
            .iter()
            .map(|e| evidence_response(e, db))
            .collect(),
        contradicting: report
            .contradicting_evidence
            .iter()
            .map(|e| evidence_response(e, db))
            .collect(),
        numeric_conflicts,
    };

    serialize_or_error(&response)
}

pub(crate) fn cell_to_json(cell_id: u64, seq: u64, payload: &[u8]) -> String {
    serialize_or_error(&CellResponse {
        cell_id,
        seq,
        payload: String::from_utf8_lossy(payload).into_owned(),
    })
}

pub(crate) fn search_to_json(
    results: &[DatabaseSearchResult],
    search_mode: &str,
    routing: Option<&SearchRouteDecision>,
) -> String {
    serialize_or_error(&SearchResponse {
        search_mode: search_mode.to_owned(),
        routing: routing.map(|decision| SearchRoutingDecisionResponse {
            requested_mode: decision.requested_mode.clone(),
            selected_strategy: decision.selected_strategy.as_str().to_owned(),
            reason: decision.reason.to_owned(),
            text_available: decision.text_available,
            vector_available: decision.vector_available,
        }),
        results: results
            .iter()
            .map(|r| SearchResultResponse {
                cell_id: r.cell_id.0,
                score: r.score,
                lexical_score: r.lexical_score,
                vector_score: r.vector_score,
                payload: String::from_utf8_lossy(&r.payload).into_owned(),
            })
            .collect(),
    })
}

pub(crate) fn remember_to_json(result: &RememberedCell) -> String {
    serialize_or_error(&RememberResponse {
        seq: result.commit_seq.0,
        cell_id: result.cell_id.0,
        ttl_seconds: result.ttl_seconds,
    })
}

pub(crate) fn stats_to_json(stats: &StorageStats) -> String {
    serialize_or_error(&CliStatsResponse {
        current_seq: stats.current_seq.0,
        checkpoint_seq: stats.checkpoint_seq.0,
        live_segments: stats.live_segments,
        retired_segments: stats.retired_segments,
        memtable_cells: stats.memtable.cell_count,
        memtable_versions: stats.memtable.version_count,
        memtable_payload_bytes: stats.memtable_payload_bytes,
        estimated_memtable_bytes: stats.estimated_memtable_bytes,
        estimated_index_bytes: stats.estimated_index_bytes,
        estimated_context_pack_bytes: stats.estimated_context_pack_bytes,
        estimated_total_memory_bytes: stats.estimated_total_memory_bytes,
        live_segment_bytes: stats.live_segment_bytes,
        retired_segment_bytes: stats.retired_segment_bytes,
        total_segment_bytes: stats.total_segment_bytes,
        durable_storage_bytes: stats.durable_storage_bytes,
        live_segment_payload_bytes: stats.live_segment_payload_bytes,
        logical_payload_bytes: stats.logical_payload_bytes,
        space_amplification_q16: stats.space_amplification_q16,
        write_amplification_q16: stats.write_amplification_q16,
        compaction_pressure_q16: stats.compaction_pressure_q16,
        wal_size_bytes: stats.wal_size_bytes,
        wal_writer_records: stats.wal_writer.records_written,
        wal_writer_bytes: stats.wal_writer.bytes_written,
        wal_writer_fsyncs: stats.wal_writer.fsync_count,
        wal_writer_batches: stats.wal_writer.batches_committed,
    })
}

pub(crate) fn validation_to_json(
    live_segments_checked: usize,
    cells_checked: usize,
    wal_records_checked: u64,
    wal_safe_truncate_offset: u64,
    ok: bool,
) -> String {
    serialize_or_error(&CliValidateResponse {
        ok,
        live_segments_checked,
        cells_checked,
        wal_records_checked,
        wal_safe_truncate_offset,
    })
}

pub(crate) fn ann_validate_to_json(
    vector_indexes_checked: usize,
    hnsw_graphs_checked: usize,
    errors: Vec<String>,
) -> String {
    let ok = errors.is_empty();
    serialize_or_error(&CliAnnValidateResponse {
        ok,
        vector_indexes_checked,
        hnsw_graphs_checked,
        errors,
    })
}

pub(crate) fn vector_rebuild_to_json(report: &VectorRebuildReport) -> String {
    serialize_or_error(&CliVectorRebuildResponse {
        segments_checked: report.segments_checked,
        cells_scanned: report.cells_scanned,
        vector_candidates: report.vector_candidates,
        vector_indexes_rebuilt: report.vector_indexes_rebuilt,
        hnsw_graphs_rebuilt: report.hnsw_graphs_rebuilt,
        hnsw_enabled: report.hnsw_enabled,
    })
}

pub(crate) fn no_fallback_profile_to_json(
    policy: Option<cortex_engine::HnswNoFallbackRolloutPolicy>,
) -> String {
    let response = match policy {
        Some(policy) => crate::cli_json_types::CliNoFallbackProfileResponse {
            configured: true,
            rollout_enabled: Some(policy.rollout_enabled),
            min_recall_q16: Some(policy.min_recall_q16),
            require_upper_layers: Some(policy.require_upper_layers),
        },
        None => crate::cli_json_types::CliNoFallbackProfileResponse {
            configured: false,
            rollout_enabled: None,
            min_recall_q16: None,
            require_upper_layers: None,
        },
    };
    serialize_or_error(&response)
}

pub(crate) struct CliAnnEvaluationJsonInput {
    pub(crate) available: bool,
    pub(crate) reason: Option<String>,
    pub(crate) report: Option<CliAnnSearchReportResponse>,
    pub(crate) no_fallback_decision: Option<crate::cli_json_types::CliNoFallbackDecisionResponse>,
    pub(crate) exact_top_k: Vec<u32>,
    pub(crate) ann_top_k: Vec<u32>,
    pub(crate) overlap_count: usize,
    pub(crate) recall_q16: u16,
}

pub(crate) fn ann_evaluation_to_json(input: CliAnnEvaluationJsonInput) -> String {
    serialize_or_error(&CliAnnEvaluationResponse {
        available: input.available,
        reason: input.reason,
        ann_report: input.report,
        no_fallback_decision: input.no_fallback_decision,
        exact_top_k: input.exact_top_k,
        ann_top_k: input.ann_top_k,
        overlap_count: input.overlap_count,
        recall_q16: input.recall_q16,
    })
}

fn evidence_response(
    evidence: &VerificationEvidence,
    db: &Database,
) -> VerificationEvidenceResponse {
    let payload_text = db
        .get_latest_cell(evidence.cell_id)
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .unwrap_or_else(|| "null".to_owned());
    VerificationEvidenceResponse {
        cell_id: evidence.cell_id.0,
        matched_terms: evidence.matched_terms,
        match_score_q16: evidence.match_score_q16,
        match_kind: evidence.match_kind.as_str().to_owned(),
        source_trust_q16: evidence.source_trust_q16,
        source_trust_category: evidence.source_trust_category.as_str().to_owned(),
        citation: evidence.citation.clone(),
        payload_text,
    }
}

fn verification_verdict(status: VerificationStatus) -> &'static str {
    match status {
        VerificationStatus::Supported => "supported",
        VerificationStatus::Insufficient => "insufficient",
        VerificationStatus::Contradicted => "contradicted",
        VerificationStatus::Mixed => "mixed_evidence",
    }
}
