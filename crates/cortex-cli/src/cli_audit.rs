use std::collections::BTreeMap;
use std::fs;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::cli_audit_chain;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AuditRecord {
    pub schema_version: String,
    pub audit_event: String,
    pub audit_action: String,
    #[serde(default)]
    pub chain_id: Option<String>,
    #[serde(default)]
    pub sequence: Option<u64>,
    #[serde(default)]
    pub prev_hash: Option<String>,
    #[serde(default)]
    pub event_hash: Option<String>,
    #[serde(default)]
    pub principal_id: Option<String>,
    #[serde(default)]
    pub auth_role: Option<String>,
    #[serde(default)]
    pub auth_agent_id: Option<u64>,
    pub method: String,
    pub path: String,
    pub tenant: String,
    #[serde(default)]
    pub request_id: Option<String>,
    pub status: u16,
    pub error_code: String,
    pub duration_ms: u64,
    pub unix_time_ms: u128,
    #[serde(default)]
    pub llm: Option<LlmAuditFields>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct LlmAuditFields {
    pub outcome: String,
    pub reason: String,
    pub provider: String,
    pub model: String,
    pub context_cell_count: u64,
    pub citation_count: u64,
    pub request_api_key_present: bool,
    pub prompt_body_logged: bool,
    pub secrets_logged: bool,
}

#[derive(Debug, Serialize)]
struct AuditReviewResponse {
    total_records: usize,
    matched_records: usize,
    redaction_ok: bool,
    redaction_violations: usize,
    chain_ok: bool,
    chain_violations: usize,
    filters: AuditReviewFilters,
    summary: AuditReviewSummary,
    records: Vec<AuditRecord>,
}

#[derive(Debug, Serialize)]
struct AuditReviewFilters {
    route: Option<String>,
    status: Option<u16>,
    action: Option<String>,
    tenant: Option<String>,
}

#[derive(Default, Debug, Serialize)]
struct AuditReviewSummary {
    by_action: BTreeMap<String, usize>,
    by_status: BTreeMap<u16, usize>,
    by_tenant: BTreeMap<String, usize>,
    by_route: BTreeMap<String, usize>,
}

#[derive(Debug)]
pub struct AuditReviewOptions<'a> {
    pub path: &'a str,
    pub route: Option<&'a str>,
    pub status: Option<u16>,
    pub action: Option<&'a str>,
    pub tenant: Option<&'a str>,
    pub summary_only: bool,
    pub redaction_check: bool,
    pub verify_chain: bool,
    pub json: bool,
}

pub fn review(options: AuditReviewOptions<'_>) -> Result<String, String> {
    let input = fs::read_to_string(options.path)
        .map_err(|error| format!("audit log could not be read: {error}"))?;
    let mut records = Vec::new();
    let mut total_records = 0usize;
    let mut redaction_violations = 0usize;
    let mut chain_violations = 0usize;
    let mut previous_hash = cli_audit_chain::AUDIT_CHAIN_ZERO_HASH.to_owned();
    let mut expected_sequence = 1u64;

    for (index, line) in input.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let value = serde_json::from_str::<Value>(trimmed)
            .map_err(|error| format!("audit log line {} is not valid JSON: {error}", index + 1))?;
        total_records += 1;
        if has_redaction_violation(&value) {
            redaction_violations += 1;
        }
        let record = serde_json::from_value::<AuditRecord>(value).map_err(|error| {
            format!(
                "audit log line {} does not match cortexdb.audit.v1: {error}",
                index + 1
            )
        })?;
        if !cli_audit_chain::verify_record(&record, expected_sequence, &previous_hash) {
            chain_violations += 1;
        }
        if let Some(hash) = &record.event_hash {
            previous_hash = hash.clone();
        }
        expected_sequence = expected_sequence
            .checked_add(1)
            .ok_or_else(|| "audit chain sequence overflow".to_owned())?;
        records.push(record);
    }

    let matched = records
        .into_iter()
        .filter(|record| matches_filters(record, &options))
        .collect::<Vec<_>>();
    let summary = summarize(&matched);
    let redaction_ok = redaction_violations == 0;

    if options.redaction_check && !redaction_ok {
        return Err(format!(
            "audit redaction check failed: redaction_violations={redaction_violations}"
        ));
    }
    let chain_ok = chain_violations == 0;
    if options.verify_chain && !chain_ok {
        return Err(format!(
            "audit chain verification failed: chain_violations={chain_violations}"
        ));
    }

    let response = AuditReviewResponse {
        total_records,
        matched_records: matched.len(),
        redaction_ok,
        redaction_violations,
        chain_ok,
        chain_violations,
        filters: AuditReviewFilters {
            route: options.route.map(str::to_owned),
            status: options.status,
            action: options.action.map(str::to_owned),
            tenant: options.tenant.map(str::to_owned),
        },
        summary,
        records: if options.summary_only {
            Vec::new()
        } else {
            matched
        },
    };

    if options.json {
        return serde_json::to_string_pretty(&response).map_err(|error| error.to_string());
    }
    Ok(format_plain(&response, options.summary_only))
}

fn matches_filters(record: &AuditRecord, options: &AuditReviewOptions<'_>) -> bool {
    options.route.is_none_or(|route| record.path == route)
        && options.status.is_none_or(|status| record.status == status)
        && options
            .action
            .is_none_or(|action| record.audit_action == action)
        && options.tenant.is_none_or(|tenant| record.tenant == tenant)
}

fn summarize(records: &[AuditRecord]) -> AuditReviewSummary {
    let mut summary = AuditReviewSummary::default();
    for record in records {
        *summary
            .by_action
            .entry(record.audit_action.clone())
            .or_default() += 1;
        *summary.by_status.entry(record.status).or_default() += 1;
        *summary.by_tenant.entry(record.tenant.clone()).or_default() += 1;
        *summary.by_route.entry(record.path.clone()).or_default() += 1;
    }
    summary
}

fn has_redaction_violation(value: &Value) -> bool {
    match value {
        Value::Object(map) => map.iter().any(|(key, value)| {
            let key = key.to_ascii_lowercase();
            matches!(
                key.as_str(),
                "authorization" | "auth_header" | "body" | "payload" | "query" | "query_string"
            ) || has_redaction_violation(value)
        }),
        Value::Array(values) => values.iter().any(has_redaction_violation),
        Value::String(value) => value.contains('?') && value.starts_with("/v1/"),
        _ => false,
    }
}

fn format_plain(response: &AuditReviewResponse, summary_only: bool) -> String {
    let mut lines = vec![format!(
        "audit_records={} matched_records={} redaction_ok={} redaction_violations={} chain_ok={} chain_violations={}",
        response.total_records,
        response.matched_records,
        response.redaction_ok,
        response.redaction_violations,
        response.chain_ok,
        response.chain_violations
    )];
    lines.push(format_counts("by_action", &response.summary.by_action));
    lines.push(format_counts("by_status", &response.summary.by_status));
    lines.push(format_counts("by_tenant", &response.summary.by_tenant));
    lines.push(format_counts("by_route", &response.summary.by_route));

    if !summary_only {
        for record in &response.records {
            lines.push(format!(
                "record method={} path={} tenant={} status={} action={} error_code={} duration_ms={}",
                record.method,
                record.path,
                record.tenant,
                record.status,
                record.audit_action,
                empty_as_dash(&record.error_code),
                record.duration_ms
            ));
        }
    }
    lines.join("\n")
}

fn format_counts<K>(label: &str, counts: &BTreeMap<K, usize>) -> String
where
    K: std::fmt::Display,
{
    if counts.is_empty() {
        return format!("{label}=none");
    }
    let values = counts
        .iter()
        .map(|(key, value)| format!("{key}:{value}"))
        .collect::<Vec<_>>()
        .join(",");
    format!("{label}={values}")
}

fn empty_as_dash(value: &str) -> &str {
    if value.is_empty() {
        "-"
    } else {
        value
    }
}
