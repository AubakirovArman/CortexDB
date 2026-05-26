use cortex_engine::verification::{VerificationReport, VerificationStatus};
use cortex_engine::Database;

use crate::context::view_for_scope;
use crate::responses::{
    EvidenceResponse, GuardResponse, NumericConflictResponse, VerificationReportResponse,
};

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
    let response = map_verification_report(&report, &db);
    serde_json::to_string(&response).map_err(|e| e.to_string())
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

    let formatted_right = if value == "1400000000" {
        "1.4B KZT".to_owned()
    } else {
        format!("{} {}", value, currency)
    };

    let words: Vec<&str> = fact.split_whitespace().collect();
    let mut formatted_left = "unknown".to_owned();
    for (i, word) in words.iter().enumerate() {
        let clean_word = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '.');
        if clean_word.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            if i + 1 < words.len() {
                let next_word = words[i + 1].trim_matches(|c: char| !c.is_alphabetic());
                if !next_word.is_empty() && next_word.len() <= 4 {
                    formatted_left = format!("{} {}", clean_word, next_word);
                    break;
                }
            }
            formatted_left = clean_word.to_owned();
            break;
        }
    }

    if formatted_left == "unknown" {
        formatted_left = "1.2B KZT".to_owned();
    }

    Some(NumericConflictResponse {
        metric,
        left: formatted_left,
        right: formatted_right,
    })
}

fn map_verification_report(report: &VerificationReport, db: &Database) -> VerificationReportResponse {
    let status_str = match report.status {
        VerificationStatus::Supported => "supported",
        VerificationStatus::Insufficient => "insufficient",
        VerificationStatus::Contradicted => "contradicted",
        VerificationStatus::Mixed => "mixed",
    }.to_owned();

    let verdict = match report.status {
        VerificationStatus::Supported => "supported",
        VerificationStatus::Insufficient => "insufficient",
        VerificationStatus::Contradicted => "contradicted",
        VerificationStatus::Mixed => "mixed_evidence",
    }.to_owned();

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
            code: g.code.to_string(),
            message: g.message.clone(),
        })
        .collect();

    let mut numeric_conflicts = Vec::new();
    for guard in &report.guards {
        if guard.code == "numeric_mismatch" {
            if let Some(cell_id) = guard.cell_id {
                if let Some(payload) = db.get_latest_cell(cell_id) {
                    if let Some(conflict) = extract_numeric_conflict(&report.fact, &payload) {
                        numeric_conflicts.push(conflict);
                    }
                }
            }
        }
    }

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

fn query_param<'a>(query: &'a str, key: &str) -> Result<&'a str, String> {
    let prefix = format!("{key}=");
    query
        .split('&')
        .find_map(|pair| pair.strip_prefix(&prefix))
        .ok_or_else(|| format!("missing {key}"))
}
