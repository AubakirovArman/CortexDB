use cortex_aql::AgentView;
use cortex_engine::verification::{VerificationReport, VerificationStatus};
use cortex_engine::Database;

use crate::authz;
use crate::responses::{
    EvidenceResponse, GuardResponse, NumericConflictResponse, RememberResponse, RouterError,
    VerificationReportResponse,
};
use crate::router::query_param_decoded;

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
    let response = map_verification_report(&report, db);
    Ok(serde_json::to_string(&response)?)
}

fn map_verification_report(
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

    let map_evidence = |evs: &[cortex_engine::verification::VerificationEvidence]| {
        evs.iter()
            .map(|ev| {
                let payload_text = db
                    .get_latest_cell(ev.cell_id)
                    .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
                    .unwrap_or_else(|| "null".to_owned());
                EvidenceResponse {
                    cell_id: ev.cell_id.0,
                    matched_terms: ev.matched_terms,
                    source_trust_q16: ev.source_trust_q16,
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
        evidence: evidence.clone(),
        contradicting_evidence: contradicting_evidence.clone(),
        guards,
        supporting: evidence,
        contradicting: contradicting_evidence,
        numeric_conflicts,
    }
}
