use cortex_aql::{AgentId, AgentView};
use cortex_core::CellId;
use cortex_engine::{
    feedback::{ContextFeedback, FeedbackStats},
    Database, VerificationEvidence, VerificationReport, VerificationReportExportFormat,
    VerificationStatus,
};

use crate::authz;
use crate::responses::{
    EvidenceResponse, FeedbackCellStatsResponse, FeedbackRecordResponse, FeedbackStatsResponse,
    GuardResponse, NumericConflictResponse, RememberResponse, RouterError,
    VerificationReportResponse,
};
use crate::router::{query_param, query_param_decoded, query_param_opt_decoded};

mod conflicts;

pub use conflicts::handle_conflicts_shared;

pub fn handle_remember_shared(
    db: &mut Database,
    query: &str,
    body: &[u8],
    authenticated_view: Option<&AgentView>,
) -> Result<String, RouterError> {
    let scope = query_param_decoded(query, "scope").map_err(RouterError::BadRequest)?;
    let view = authz::remember_view_for_scope(&scope, authenticated_view)?;
    let aql = String::from_utf8_lossy(body);
    let result = db.remember_aql(&aql, &view)?;
    let response = RememberResponse {
        seq: result.commit_seq.0,
        cell_id: result.cell_id.0,
        ttl_seconds: result.ttl_seconds,
    };
    Ok(serde_json::to_string(&response)?)
}

pub fn handle_verify_shared(
    db: &Database,
    query: &str,
    body: &[u8],
    authenticated_view: Option<&AgentView>,
) -> Result<String, RouterError> {
    let scope = query_param_decoded(query, "scope").map_err(RouterError::BadRequest)?;
    let view = authz::verify_view_for_scope(&scope, authenticated_view)?;
    let aql = String::from_utf8_lossy(body);
    let report = db.verify_fact_aql(&aql, &view)?;
    match verify_output_format(query).as_str() {
        "json" => {
            let response = map_verification_report(&report, db);
            Ok(serde_json::to_string(&response)?)
        }
        "markdown" => Ok(report.export(VerificationReportExportFormat::Markdown)),
        "audit" => Ok(report.export(VerificationReportExportFormat::Audit)),
        other => Err(RouterError::BadRequest(format!(
            "unsupported verify format '{other}' (expected json, markdown, or audit)"
        ))),
    }
}

pub fn handle_feedback_shared(
    db: &mut Database,
    query: &str,
    body: &[u8],
    authenticated_view: Option<&AgentView>,
) -> Result<String, RouterError> {
    let source_cell_id = query_param(query, "source_cell_id")
        .map_err(RouterError::BadRequest)?
        .parse::<u64>()
        .map(CellId)
        .map_err(|_| RouterError::BadRequest("source_cell_id must be u64".to_owned()))?;
    let useful = parse_feedback_bool(
        &query_param_decoded(query, "useful").map_err(RouterError::BadRequest)?,
    )?;
    let descriptor = db
        .get_latest_cell_descriptor(source_cell_id)
        .ok_or_else(|| RouterError::NotFound("source cell not found".to_owned()))?;
    authz::require_descriptor_read(authenticated_view, &descriptor)?;
    let agent_id = authenticated_view.map_or(AgentId(0), |view| view.agent_id);
    let note = (!body.is_empty()).then(|| String::from_utf8_lossy(body).trim().to_owned());
    let stored = db.record_context_feedback(
        agent_id,
        ContextFeedback {
            source_cell_id,
            useful,
            note: note.filter(|value| !value.is_empty()),
        },
    )?;
    Ok(serde_json::to_string(&FeedbackRecordResponse {
        seq: stored.commit_seq.0,
        cell_id: stored.cell_id.0,
        source_cell_id: source_cell_id.0,
        useful,
    })?)
}

pub fn handle_feedback_stats_shared(
    db: &Database,
    authenticated_view: Option<&AgentView>,
) -> Result<String, RouterError> {
    Ok(serde_json::to_string(&map_feedback_stats(
        db,
        db.feedback_stats(),
        authenticated_view,
    ))?)
}

fn parse_feedback_bool(value: &str) -> Result<bool, RouterError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => Err(RouterError::BadRequest("useful must be boolean".to_owned())),
    }
}

fn map_feedback_stats(
    db: &Database,
    stats: FeedbackStats,
    authenticated_view: Option<&AgentView>,
) -> FeedbackStatsResponse {
    let mut visible_total = 0;
    let mut visible_useful = 0;
    let mut visible_not_useful = 0;
    let mut by_source_cell = Vec::new();
    for (source_cell_id, stats) in stats.by_source_cell {
        let visible = db
            .get_latest_cell_descriptor(source_cell_id)
            .is_none_or(|descriptor| {
                authz::require_descriptor_read(authenticated_view, &descriptor).is_ok()
            });
        if !visible {
            continue;
        }
        visible_total += stats.useful + stats.not_useful;
        visible_useful += stats.useful;
        visible_not_useful += stats.not_useful;
        by_source_cell.push(FeedbackCellStatsResponse {
            source_cell_id: source_cell_id.0,
            useful: stats.useful,
            not_useful: stats.not_useful,
            score: stats.score,
        });
    }
    FeedbackStatsResponse {
        total: visible_total,
        useful: visible_useful,
        not_useful: visible_not_useful,
        by_source_cell,
    }
}

fn verify_output_format(query: &str) -> String {
    query_param_opt_decoded(query, "format")
        .unwrap_or_else(|| "json".to_owned())
        .trim()
        .to_ascii_lowercase()
}

pub(crate) fn map_verification_report(
    report: &VerificationReport,
    db: &Database,
) -> VerificationReportResponse {
    let status_str = match report.status {
        VerificationStatus::Supported => "supported",
        VerificationStatus::Insufficient => "insufficient",
        VerificationStatus::Contradicted => "contradicted",
        VerificationStatus::Mixed => "mixed",
    }
    .to_owned();

    let verdict = match report.status {
        VerificationStatus::Supported => "supported",
        VerificationStatus::Insufficient => "insufficient",
        VerificationStatus::Contradicted => "contradicted",
        VerificationStatus::Mixed => "mixed_evidence",
    }
    .to_owned();

    let map_evidence = |evs: &[VerificationEvidence]| {
        evs.iter()
            .map(|ev| {
                let payload_text = db
                    .get_latest_cell(ev.cell_id)
                    .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
                    .unwrap_or_else(|| "null".to_owned());
                EvidenceResponse {
                    cell_id: ev.cell_id.0,
                    matched_terms: ev.matched_terms,
                    match_score_q16: ev.match_score_q16,
                    match_kind: ev.match_kind.as_str().to_owned(),
                    source_trust_q16: ev.source_trust_q16,
                    source_trust_category: ev.source_trust_category.as_str().to_owned(),
                    citation: ev.citation.clone(),
                    payload_text,
                }
            })
            .collect::<Vec<_>>()
    };

    let evidence = map_evidence(&report.evidence);
    let contradicting_evidence = map_evidence(&report.contradicting_evidence);

    let guards = report
        .guards
        .iter()
        .map(|g| GuardResponse {
            cell_id: g.cell_id.map(|cid| cid.0),
            code: g.code.as_str().to_owned(),
            message: g.message.clone(),
        })
        .collect();

    let numeric_conflicts = report
        .numeric_conflicts
        .iter()
        .map(|conflict| NumericConflictResponse {
            metric: conflict.metric.clone(),
            left: conflict.left.clone(),
            right: conflict.right.clone(),
        })
        .collect();

    VerificationReportResponse {
        fact: report.fact.clone(),
        status: status_str,
        verdict,
        confidence_q16: report.confidence_q16,
        evidence: evidence.clone(),
        contradicting_evidence: contradicting_evidence.clone(),
        guards,
        supporting: evidence,
        contradicting: contradicting_evidence,
        numeric_conflicts,
    }
}
