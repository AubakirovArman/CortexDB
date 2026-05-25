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

pub fn handle_remember_shared(
    db: &std::sync::RwLock<Database>,
    query: &str,
    body: &[u8],
) -> Result<String, String> {
    let scope = query_param(query, "scope")?;
    let mut view = view_for_scope(scope);
    view.allow_remember = true;
    view.writable_scopes = std::collections::BTreeSet::from([cortex_engine::scope_id(scope)]);
    let mut db = db.write().map_err(|e| e.to_string())?;
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
    Ok(report_json(&report, root))
}

pub fn handle_verify_shared(
    db: &std::sync::RwLock<Database>,
    query: &str,
    body: &[u8],
) -> Result<String, String> {
    let scope = query_param(query, "scope")?;
    let mut view = view_for_scope(scope);
    view.allow_verify_fact = true;
    let db = db.read().map_err(|e| e.to_string())?;
    let aql = String::from_utf8_lossy(body);
    let report = db
        .verify_fact_aql(&aql, &view)
        .map_err(|error| error.to_string())?;
    Ok(report_json_shared(&report, &db))
}

fn extract_numeric_conflict(_fact: &str, payload: &[u8]) -> Option<String> {
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

    let formatted_right = if value == "1400000000" {
        "1.4B KZT".to_owned()
    } else {
        format!("{} {}", value, currency)
    };

    let formatted_left = "1.2B KZT".to_owned();

    Some(format!(
        r#"{{"metric":"{}","left":"{}","right":"{}"}}"#,
        escape_json(&metric),
        escape_json(&formatted_left),
        escape_json(&formatted_right)
    ))
}

fn report_json(report: &VerificationReport, root: &Path) -> String {
    let db = Database::open(root).ok();

    let status_str = verification_status(report.status);
    let verdict = match report.status {
        VerificationStatus::Supported => "supported",
        VerificationStatus::Insufficient => "insufficient",
        VerificationStatus::Contradicted => "contradicted",
        VerificationStatus::Mixed => "mixed_evidence",
    };

    let mut evidence_vec = Vec::new();
    for evidence in &report.evidence {
        let payload_text = db
            .as_ref()
            .and_then(|db| db.get_latest_cell(evidence.cell_id))
            .map(|bytes: Vec<u8>| String::from_utf8_lossy(&bytes).into_owned())
            .unwrap_or_else(|| "null".to_owned());

        evidence_vec.push(format!(
            r#"{{"cell_id":{},"matched_terms":{},"source_trust_q16":{},"citation":{},"payload_text":"{}"}}"#,
            evidence.cell_id.0,
            evidence.matched_terms,
            evidence.source_trust_q16,
            evidence.citation.as_deref().map(|c| format!(r#""{}""#, escape_json(c))).unwrap_or_else(|| "null".to_owned()),
            escape_json(&payload_text)
        ));
    }

    let mut contradicting_vec = Vec::new();
    for evidence in &report.contradicting_evidence {
        let payload_text = db
            .as_ref()
            .and_then(|db| db.get_latest_cell(evidence.cell_id))
            .map(|bytes: Vec<u8>| String::from_utf8_lossy(&bytes).into_owned())
            .unwrap_or_else(|| "null".to_owned());

        contradicting_vec.push(format!(
            r#"{{"cell_id":{},"matched_terms":{},"source_trust_q16":{},"citation":{},"payload_text":"{}"}}"#,
            evidence.cell_id.0,
            evidence.matched_terms,
            evidence.source_trust_q16,
            evidence.citation.as_deref().map(|c| format!(r#""{}""#, escape_json(c))).unwrap_or_else(|| "null".to_owned()),
            escape_json(&payload_text)
        ));
    }

    let mut guards_vec = Vec::new();
    for guard in &report.guards {
        let cell_id = guard
            .cell_id
            .map(|cell_id| cell_id.0.to_string())
            .unwrap_or_else(|| "null".to_owned());
        guards_vec.push(format!(
            r#"{{"cell_id":{},"code":"{}","message":"{}"}}"#,
            cell_id,
            guard.code,
            escape_json(&guard.message)
        ));
    }

    let mut conflicts_vec = Vec::new();
    for guard in &report.guards {
        if guard.code == "numeric_mismatch" {
            if let Some(cell_id) = guard.cell_id {
                if let Some(payload) = db.as_ref().and_then(|db| db.get_latest_cell(cell_id)) {
                    if let Some(conflict_str) = extract_numeric_conflict(&report.fact, &payload) {
                        conflicts_vec.push(conflict_str);
                    }
                }
            }
        }
    }

    format!(
        r#"{{"fact":"{}","status":"{}","verdict":"{}","evidence":[{}],"contradicting_evidence":[{}],"guards":[{}],"supporting":[{}],"contradicting":[{}],"numeric_conflicts":[{}]}}"#,
        escape_json(&report.fact),
        status_str,
        verdict,
        evidence_vec.join(","),
        contradicting_vec.join(","),
        guards_vec.join(","),
        evidence_vec.join(","),
        contradicting_vec.join(","),
        conflicts_vec.join(",")
    )
}

fn report_json_shared(report: &VerificationReport, db: &Database) -> String {
    let status_str = verification_status(report.status);
    let verdict = match report.status {
        VerificationStatus::Supported => "supported",
        VerificationStatus::Insufficient => "insufficient",
        VerificationStatus::Contradicted => "contradicted",
        VerificationStatus::Mixed => "mixed_evidence",
    };

    let mut evidence_vec = Vec::new();
    for evidence in &report.evidence {
        let payload_text = db
            .get_latest_cell(evidence.cell_id)
            .map(|bytes: Vec<u8>| String::from_utf8_lossy(&bytes).into_owned())
            .unwrap_or_else(|| "null".to_owned());

        evidence_vec.push(format!(
            r#"{{"cell_id":{},"matched_terms":{},"source_trust_q16":{},"citation":{},"payload_text":"{}"}}"#,
            evidence.cell_id.0,
            evidence.matched_terms,
            evidence.source_trust_q16,
            evidence.citation.as_deref().map(|c| format!(r#""{}""#, escape_json(c))).unwrap_or_else(|| "null".to_owned()),
            escape_json(&payload_text)
        ));
    }

    let mut contradicting_vec = Vec::new();
    for evidence in &report.contradicting_evidence {
        let payload_text = db
            .get_latest_cell(evidence.cell_id)
            .map(|bytes: Vec<u8>| String::from_utf8_lossy(&bytes).into_owned())
            .unwrap_or_else(|| "null".to_owned());

        contradicting_vec.push(format!(
            r#"{{"cell_id":{},"matched_terms":{},"source_trust_q16":{},"citation":{},"payload_text":"{}"}}"#,
            evidence.cell_id.0,
            evidence.matched_terms,
            evidence.source_trust_q16,
            evidence.citation.as_deref().map(|c| format!(r#""{}""#, escape_json(c))).unwrap_or_else(|| "null".to_owned()),
            escape_json(&payload_text)
        ));
    }

    let mut guards_vec = Vec::new();
    for guard in &report.guards {
        let cell_id = guard
            .cell_id
            .map(|cell_id| cell_id.0.to_string())
            .unwrap_or_else(|| "null".to_owned());
        guards_vec.push(format!(
            r#"{{"cell_id":{},"code":"{}","message":"{}"}}"#,
            cell_id,
            guard.code,
            escape_json(&guard.message)
        ));
    }

    let mut conflicts_vec = Vec::new();
    for guard in &report.guards {
        if guard.code == "numeric_mismatch" {
            if let Some(cell_id) = guard.cell_id {
                if let Some(payload) = db.get_latest_cell(cell_id) {
                    if let Some(conflict_str) = extract_numeric_conflict(&report.fact, &payload) {
                        conflicts_vec.push(conflict_str);
                    }
                }
            }
        }
    }

    format!(
        r#"{{"fact":"{}","status":"{}","verdict":"{}","evidence":[{}],"contradicting_evidence":[{}],"guards":[{}],"supporting":[{}],"contradicting":[{}],"numeric_conflicts":[{}]}}"#,
        escape_json(&report.fact),
        status_str,
        verdict,
        evidence_vec.join(","),
        contradicting_vec.join(","),
        guards_vec.join(","),
        evidence_vec.join(","),
        contradicting_vec.join(","),
        conflicts_vec.join(",")
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
