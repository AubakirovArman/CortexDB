use std::path::Path;

use cortex_engine::verification::{VerificationReport, VerificationStatus};
use cortex_engine::Database;

use crate::context::{escape_json, view_for_scope};

pub fn handle_remember(root: &Path, query: &str, body: &[u8]) -> Result<String, String> {
    let scope = query_param(query, "scope")?;
    let mut view = view_for_scope(scope);
    view.allow_remember = true;
    view.writable_scopes = std::collections::BTreeSet::from([cortex_engine::scope_id(scope)]);
    let mut db = Database::open(root).map_err(|error| error.to_string())?;
    let aql = String::from_utf8_lossy(body);
    let result = db
        .remember_aql(&aql, &view)
        .map_err(|error| error.to_string())?;
    let ttl = result
        .ttl_seconds
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned());
    Ok(format!(
        r#"{{"seq":{},"cell_id":{},"ttl_seconds":{}}}"#,
        result.commit_seq.0, result.cell_id.0, ttl
    ))
}

pub fn handle_verify(root: &Path, query: &str, body: &[u8]) -> Result<String, String> {
    let scope = query_param(query, "scope")?;
    let mut view = view_for_scope(scope);
    view.allow_verify_fact = true;
    let db = Database::open(root).map_err(|error| error.to_string())?;
    let aql = String::from_utf8_lossy(body);
    let report = db
        .verify_fact_aql(&aql, &view)
        .map_err(|error| error.to_string())?;
    Ok(report_json(&report))
}

fn report_json(report: &VerificationReport) -> String {
    let evidence = report
        .evidence
        .iter()
        .map(|evidence| {
            format!(
                r#"{{"cell_id":{},"matched_terms":{},"source_trust_q16":{}}}"#,
                evidence.cell_id.0, evidence.matched_terms, evidence.source_trust_q16
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let contradicting_evidence = report
        .contradicting_evidence
        .iter()
        .map(|evidence| {
            format!(
                r#"{{"cell_id":{},"matched_terms":{},"source_trust_q16":{}}}"#,
                evidence.cell_id.0, evidence.matched_terms, evidence.source_trust_q16
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let guards = report
        .guards
        .iter()
        .map(|guard| {
            let cell_id = guard
                .cell_id
                .map(|cell_id| cell_id.0.to_string())
                .unwrap_or_else(|| "null".to_owned());
            format!(
                r#"{{"cell_id":{},"code":"{}","message":"{}"}}"#,
                cell_id,
                guard.code,
                escape_json(&guard.message)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        r#"{{"fact":"{}","status":"{}","evidence":[{}],"contradicting_evidence":[{}],"guards":[{}]}}"#,
        escape_json(&report.fact),
        verification_status(report.status),
        evidence,
        contradicting_evidence,
        guards
    )
}

fn verification_status(status: VerificationStatus) -> &'static str {
    match status {
        VerificationStatus::Supported => "supported",
        VerificationStatus::Insufficient => "insufficient",
        VerificationStatus::Contradicted => "contradicted",
        VerificationStatus::Mixed => "mixed",
    }
}

fn query_param<'a>(query: &'a str, key: &str) -> Result<&'a str, String> {
    let prefix = format!("{key}=");
    query
        .split('&')
        .find_map(|pair| pair.strip_prefix(&prefix))
        .ok_or_else(|| format!("missing {key}"))
}
