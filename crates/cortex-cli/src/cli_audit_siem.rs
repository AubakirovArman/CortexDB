use std::fs;

use serde::Serialize;

use crate::cli_audit::{review, AuditRecord, AuditReviewOptions};

#[derive(Debug, Serialize)]
struct SiemAuditEvent<'a> {
    schema_version: &'static str,
    event_kind: &'static str,
    event_category: &'static str,
    event_action: &'a str,
    event_outcome: &'static str,
    service_name: &'static str,
    http_request_method: &'a str,
    url_path: &'a str,
    tenant: &'a str,
    principal_id: &'a str,
    auth_role: &'a str,
    auth_agent_id: Option<u64>,
    scope_decision: &'a str,
    request_id: &'a str,
    status: u16,
    error_code: &'a str,
    duration_ms: u64,
    unix_time_ms: u128,
    audit_chain_id: &'a str,
    audit_sequence: Option<u64>,
    audit_prev_hash: &'a str,
    audit_event_hash: &'a str,
    audit_mac_key_id: &'a str,
    audit_event_mac: &'a str,
}

#[derive(Debug, Serialize)]
pub(crate) struct SiemExportResponse {
    schema_version: &'static str,
    input_records: usize,
    exported_records: usize,
    output_path: String,
    redaction_checked: bool,
    chain_checked: bool,
}

pub(crate) fn export_jsonl(
    input_path: &str,
    output_path: &str,
    redaction_check: bool,
    verify_chain: bool,
    mac_key: Option<&cortex_crypto::MacKey>,
    json: bool,
) -> Result<String, String> {
    review(AuditReviewOptions {
        path: input_path,
        route: None,
        status: None,
        action: None,
        tenant: None,
        summary_only: true,
        redaction_check,
        verify_chain,
        mac_key,
        json: false,
    })?;

    let records = load_records(input_path)?;
    let mut output = String::new();
    for record in &records {
        output.push_str(
            &serde_json::to_string(&siem_event(record)).map_err(|error| error.to_string())?,
        );
        output.push('\n');
    }
    fs::write(output_path, output)
        .map_err(|error| format!("SIEM export could not be written: {error}"))?;

    let response = SiemExportResponse {
        schema_version: "cortexdb.siem_export.v1",
        input_records: records.len(),
        exported_records: records.len(),
        output_path: output_path.to_owned(),
        redaction_checked: redaction_check,
        chain_checked: verify_chain,
    };
    if json {
        serde_json::to_string_pretty(&response).map_err(|error| error.to_string())
    } else {
        Ok(format!(
            "siem_exported_records={} output_path={} redaction_checked={} chain_checked={}",
            response.exported_records,
            response.output_path,
            response.redaction_checked,
            response.chain_checked
        ))
    }
}

fn load_records(input_path: &str) -> Result<Vec<AuditRecord>, String> {
    let input = fs::read_to_string(input_path)
        .map_err(|error| format!("audit log could not be read: {error}"))?;
    input
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let trimmed = line.trim();
            (!trimmed.is_empty()).then_some((index, trimmed))
        })
        .map(|(index, line)| {
            serde_json::from_str::<AuditRecord>(line).map_err(|error| {
                format!(
                    "audit log line {} does not match cortexdb.audit.v1: {error}",
                    index + 1
                )
            })
        })
        .collect()
}

fn siem_event(record: &AuditRecord) -> SiemAuditEvent<'_> {
    SiemAuditEvent {
        schema_version: "cortexdb.siem.audit.v1",
        event_kind: "event",
        event_category: "web",
        event_action: &record.audit_action,
        event_outcome: outcome(record.status),
        service_name: "cortexdb",
        http_request_method: &record.method,
        url_path: &record.path,
        tenant: &record.tenant,
        principal_id: record.principal_id.as_deref().unwrap_or(""),
        auth_role: record.auth_role.as_deref().unwrap_or(""),
        auth_agent_id: record.auth_agent_id,
        scope_decision: record.scope_decision.as_deref().unwrap_or(""),
        request_id: record.request_id.as_deref().unwrap_or(""),
        status: record.status,
        error_code: &record.error_code,
        duration_ms: record.duration_ms,
        unix_time_ms: record.unix_time_ms,
        audit_chain_id: record.chain_id.as_deref().unwrap_or(""),
        audit_sequence: record.sequence,
        audit_prev_hash: record.prev_hash.as_deref().unwrap_or(""),
        audit_event_hash: record.event_hash.as_deref().unwrap_or(""),
        audit_mac_key_id: record.mac_key_id.as_deref().unwrap_or(""),
        audit_event_mac: record.event_mac.as_deref().unwrap_or(""),
    }
}

fn outcome(status: u16) -> &'static str {
    if status < 400 {
        "success"
    } else {
        "failure"
    }
}
