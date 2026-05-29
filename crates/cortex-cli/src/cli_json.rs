use cortex_engine::verification::{VerificationReport, VerificationStatus};
use cortex_engine::{ContextPack, Database};
use serde_json::to_string;

use crate::cli_json_types::{
    AqlCellResponse, AqlResponse, CellResponse, CliAnnEvaluationResponse,
    CliAnnSearchReportResponse, CliAnnValidateResponse, CliStatsResponse, CliValidateResponse,
    ContextPackAnomalyResponse, ContextPackCellResponse, ContextPackExplainResponse,
    ContextPackResponse, NumericConflictResponse, RememberResponse, SearchResponse,
    SearchResultResponse, SourceRefResponse, VerificationEvidenceResponse, VerificationResponse,
};

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
        .guards
        .iter()
        .filter(|guard| guard.code == "numeric_mismatch")
        .filter_map(|guard| guard.cell_id)
        .filter_map(|cell_id| db.get_latest_cell(cell_id))
        .filter_map(|payload| extract_numeric_conflict(&report.fact, &payload))
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

pub(crate) fn aql_to_json(cells: &[cortex_engine::RetrievedCell]) -> String {
    serialize_or_error(&AqlResponse {
        cells: cells
            .iter()
            .map(|c| AqlCellResponse {
                cell_id: c.cell_id.0,
                payload: String::from_utf8_lossy(&c.payload).into_owned(),
            })
            .collect(),
    })
}

pub(crate) fn search_to_json(
    results: &[cortex_engine::search::DatabaseSearchResult],
    search_mode: &str,
) -> String {
    serialize_or_error(&SearchResponse {
        search_mode: search_mode.to_owned(),
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

pub(crate) fn remember_to_json(result: &cortex_engine::ingestion::RememberedCell) -> String {
    serialize_or_error(&RememberResponse {
        seq: result.commit_seq.0,
        cell_id: result.cell_id.0,
        ttl_seconds: result.ttl_seconds,
    })
}

pub(crate) fn stats_to_json(stats: &cortex_engine::validation::StorageStats) -> String {
    serialize_or_error(&CliStatsResponse {
        current_seq: stats.current_seq.0,
        checkpoint_seq: stats.checkpoint_seq.0,
        live_segments: stats.live_segments,
        retired_segments: stats.retired_segments,
        memtable_cells: stats.memtable.cell_count,
        memtable_versions: stats.memtable.version_count,
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

pub(crate) fn ann_evaluation_to_json(
    available: bool,
    reason: Option<String>,
    report: Option<CliAnnSearchReportResponse>,
    exact_top_k: Vec<u32>,
    ann_top_k: Vec<u32>,
    overlap_count: usize,
    recall_q16: u16,
) -> String {
    serialize_or_error(&CliAnnEvaluationResponse {
        available,
        reason,
        ann_report: report,
        exact_top_k,
        ann_top_k,
        overlap_count,
        recall_q16,
    })
}

fn context_cell_json(cell: &cortex_engine::ContextPackCell) -> ContextPackCellResponse {
    let metadata = cortex_engine::query::CellMetadata::from_payload(&cell.payload);
    let explain = cell.explain.as_ref().map(|exp| ContextPackExplainResponse {
        score: exp.score,
        matched_terms: exp.matched_terms.clone(),
        why_selected: exp.why_selected.clone(),
        base_bm25: exp.base_bm25,
        source_trust_bonus: exp.source_trust_bonus,
        redundancy_penalty: exp.redundancy_penalty,
    });
    let source_ref = metadata.source_ref.as_ref().map(|sr| SourceRefResponse {
        source_id: sr.source_id.clone(),
        document_id: sr.document_id.clone(),
        page: sr.page,
        cell_range: sr.cell_range.clone(),
        json_path: sr.json_path.clone(),
        confidence_q16: sr.confidence_q16,
    });

    ContextPackCellResponse {
        cell_id: cell.cell_id.0,
        estimated_tokens: cell.estimated_tokens,
        citation: cell.citation.clone(),
        payload_text: String::from_utf8_lossy(&cell.payload).into_owned(),
        explain,
        source_ref,
    }
}

fn context_pack_response(pack: &ContextPack) -> ContextPackResponse {
    ContextPackResponse {
        token_budget_tokens: pack.token_budget_tokens,
        estimated_tokens: pack.estimated_tokens,
        truncated: pack.truncated,
        citations_required: pack.citations_required,
        cells: pack.cells.iter().map(context_cell_json).collect(),
        anomalies: pack
            .anomalies
            .iter()
            .map(|anomaly| ContextPackAnomalyResponse {
                cell_id: anomaly.cell_id.map(|id| id.0),
                code: anomaly.code.as_str().to_owned(),
                message: anomaly.message.clone(),
            })
            .collect(),
    }
}

fn evidence_response(
    evidence: &cortex_engine::verification::VerificationEvidence,
    db: &Database,
) -> VerificationEvidenceResponse {
    let payload_text = db
        .get_latest_cell(evidence.cell_id)
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .unwrap_or_else(|| "null".to_owned());
    VerificationEvidenceResponse {
        cell_id: evidence.cell_id.0,
        matched_terms: evidence.matched_terms,
        source_trust_q16: evidence.source_trust_q16,
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

/// Pure-integer display formatter for numeric magnitudes.
/// Avoids f64 to stay deterministic and fixed-point aligned.
///
/// Examples:
///   1_200_000_000 -> "1.2B KZT"
///   1_500_000_000 -> "1.5B KZT"
///   1_000_000     -> "1M KZT"
///   1_250_000     -> "1.25M KZT"
fn format_scale_currency(value_str: &str, currency: &str) -> String {
    if let Ok(val) = value_str.parse::<u64>() {
        if val >= 1_000_000_000 {
            let whole = val / 1_000_000_000;
            let rem = val % 1_000_000_000;
            if rem == 0 {
                return format!("{}B {}", whole, currency);
            }
            // Two decimal digits of precision without float
            let frac = rem / 10_000_000;
            if frac % 10 == 0 {
                return format!("{}.{:.1}B {}", whole, frac / 10, currency);
            }
            return format!("{}.{:.2}B {}", whole, frac / 10, currency);
        } else if val >= 1_000_000 {
            let whole = val / 1_000_000;
            let rem = val % 1_000_000;
            if rem == 0 {
                return format!("{}M {}", whole, currency);
            }
            let frac = rem / 10_000;
            if frac % 10 == 0 {
                return format!("{}.{:.1}M {}", whole, frac / 10, currency);
            }
            return format!("{}.{:.2}M {}", whole, frac / 10, currency);
        }
    }
    format!("{} {}", value_str, currency)
}

fn extract_numeric_conflict(fact: &str, payload: &[u8]) -> Option<NumericConflictResponse> {
    let text = String::from_utf8_lossy(payload);
    let mut metric = "metric".to_owned();
    let mut currency = "KZT".to_owned();
    let mut value = "unknown".to_owned();
    for line in text.lines() {
        if let Some(val) = line.strip_prefix("metric=") {
            metric = val.trim().to_owned();
        } else if let Some(val) = line.strip_prefix("currency=") {
            currency = val.trim().to_owned();
        } else if let Some(val) = line.strip_prefix("value=") {
            value = val.trim().to_owned();
        }
    }

    let formatted_right = format_scale_currency(&value, &currency);
    let formatted_left = fact_numeric_value(fact, &currency)?;
    Some(NumericConflictResponse {
        metric,
        left: formatted_left,
        right: formatted_right,
    })
}

fn fact_numeric_value(fact: &str, default_currency: &str) -> Option<String> {
    let words = fact.split_whitespace().collect::<Vec<_>>();
    for (i, word) in words.iter().enumerate() {
        let clean_word = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '.');
        if clean_word
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_digit())
        {
            if i + 1 < words.len() {
                let next_word = words[i + 1].trim_matches(|c: char| !c.is_alphabetic());
                if !next_word.is_empty() && next_word.len() <= 4 {
                    return Some(format_scale_currency(clean_word, next_word));
                }
            }
            return Some(format_scale_currency(clean_word, default_currency));
        }
    }
    None
}
