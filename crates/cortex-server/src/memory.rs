use cortex_engine::verification::numeric::{extract_numeric_values, numeric_conflict};
use cortex_engine::verification::{VerificationReport, VerificationStatus};
use cortex_engine::Database;

use crate::context::view_for_scope;
use crate::responses::{
    EvidenceResponse, GuardResponse, NumericConflictResponse, RememberResponse,
    VerificationReportResponse,
};

pub fn handle_remember_shared(
    db: &mut Database,
    query: &str,
    body: &[u8],
) -> Result<String, String> {
    let scope = query_param(query, "scope")?;
    let mut view = view_for_scope(scope);
    view.allow_remember = true;
    view.writable_scopes = std::collections::BTreeSet::from([cortex_engine::scope_id(scope)]);
    let aql = String::from_utf8_lossy(body);
    let result = db
        .remember_aql(&aql, &view)
        .map_err(|error| error.to_string())?;
    let response = RememberResponse {
        seq: result.commit_seq.0,
        cell_id: result.cell_id.0,
        ttl_seconds: result.ttl_seconds,
    };
    serde_json::to_string(&response).map_err(|e| e.to_string())
}

pub fn handle_verify_shared(db: &Database, query: &str, body: &[u8]) -> Result<String, String> {
    let scope = query_param(query, "scope")?;
    let mut view = view_for_scope(scope);
    view.allow_verify_fact = true;
    let aql = String::from_utf8_lossy(body);
    let report = db
        .verify_fact_aql(&aql, &view)
        .map_err(|error| error.to_string())?;
    let response = map_verification_report(&report, db);
    serde_json::to_string(&response).map_err(|e| e.to_string())
}

fn extract_numeric_conflict(fact: &str, payload: &[u8]) -> Option<NumericConflictResponse> {
    let text = String::from_utf8_lossy(payload);
    let mut metric = "metric".to_owned();
    let mut currency: Option<String> = None;
    let mut value_str: Option<&str> = None;

    // First pass: collect metric, currency and raw value
    for line in text.lines() {
        if let Some(val) = line.strip_prefix("metric=") {
            metric = val.trim().to_owned();
        } else if let Some(val) = line.strip_prefix("currency=") {
            currency = Some(val.trim().to_ascii_uppercase().to_owned());
        } else if let Some(val) = line.strip_prefix("value=") {
            value_str = Some(val.trim());
        }
    }

    // Parse the numeric value after currency is known
    let payload_value = value_str.and_then(|val_str| {
        let candidate = format!("{val_str} {}", currency.as_deref().unwrap_or(""));
        extract_numeric_values(&candidate).into_iter().next()
    });

    let fact_values = extract_numeric_values(fact);
    if fact_values.is_empty() {
        return None;
    }

    let fact_value = &fact_values[0];
    let payload_value = payload_value.as_ref()?;

    if !numeric_conflict(fact_value, payload_value) {
        return None;
    }

    let left = format_display(
        &fact_value.raw,
        fact_value.currency.as_deref().or(currency.as_deref()),
    );
    let right = format_display(
        &payload_value.raw,
        payload_value.currency.as_deref().or(currency.as_deref()),
    );

    Some(NumericConflictResponse {
        metric,
        left,
        right,
    })
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

fn format_display(raw: &str, currency: Option<&str>) -> String {
    if let Some(c) = currency {
        if raw.ends_with(c) || raw.to_ascii_uppercase().ends_with(c) {
            return raw.to_owned();
        }
        return format!("{} {}", raw, c);
    }
    raw.to_owned()
}

fn query_param<'a>(query: &'a str, key: &str) -> Result<&'a str, String> {
    let prefix = format!("{key}=");
    query
        .split('&')
        .find_map(|pair| pair.strip_prefix(&prefix))
        .ok_or_else(|| format!("missing {key}"))
}
