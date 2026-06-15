use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::audit_chain::{AUDIT_CHAIN_ID, AUDIT_CHAIN_ZERO_HASH};

use super::{AuditAction, AuditRecord, AuditSink};

#[derive(Clone, Copy, Serialize)]
pub(super) struct LlmInferenceAuditFields<'a> {
    outcome: &'a str,
    reason: &'a str,
    provider: &'a str,
    model: &'a str,
    context_cell_count: u64,
    citation_count: u64,
    request_api_key_present: bool,
    prompt_body_logged: bool,
    secrets_logged: bool,
}

pub(crate) struct LlmInferenceDecisionAudit<'a> {
    pub(crate) tenant: &'a str,
    pub(crate) request_id: &'a str,
    pub(crate) principal_id: Option<&'a str>,
    pub(crate) auth_role: Option<&'a str>,
    pub(crate) auth_agent_id: Option<u64>,
    pub(crate) status: u16,
    pub(crate) error_code: Option<&'a str>,
    pub(crate) duration_ms: u64,
    pub(crate) outcome: &'a str,
    pub(crate) reason: &'a str,
    pub(crate) provider: &'a str,
    pub(crate) model: &'a str,
    pub(crate) context_cell_count: u64,
    pub(crate) citation_count: u64,
    pub(crate) request_api_key_present: bool,
}

pub(crate) fn emit_llm_inference_decision(
    event: LlmInferenceDecisionAudit<'_>,
    sink: Option<&AuditSink>,
) {
    let llm = LlmInferenceAuditFields {
        outcome: event.outcome,
        reason: event.reason,
        provider: event.provider,
        model: event.model,
        context_cell_count: event.context_cell_count,
        citation_count: event.citation_count,
        request_api_key_present: event.request_api_key_present,
        prompt_body_logged: false,
        secrets_logged: false,
    };
    let error_code = event.error_code.unwrap_or("");
    let mut record = AuditRecord {
        schema_version: "cortexdb.audit.v1",
        audit_event: "llm_inference_decision",
        audit_action: AuditAction::Inference.as_str(),
        chain_id: AUDIT_CHAIN_ID,
        sequence: 0,
        prev_hash: AUDIT_CHAIN_ZERO_HASH.to_owned(),
        event_hash: String::new(),
        principal_id: event.principal_id.unwrap_or(""),
        auth_role: event.auth_role.unwrap_or(""),
        auth_agent_id: event.auth_agent_id,
        scope_decision: "not_applicable",
        method: "POST",
        path: "/v1/inference",
        tenant: event.tenant,
        request_id: event.request_id,
        status: event.status,
        error_code,
        duration_ms: event.duration_ms,
        unix_time_ms: unix_time_ms(),
        llm: Some(llm),
    };
    tracing::info!(
        target: "cortexdb_audit",
        audit_event = "llm_inference_decision",
        audit_action = "inference",
        principal_id = event.principal_id.unwrap_or(""),
        auth_role = event.auth_role.unwrap_or(""),
        auth_agent_id = event.auth_agent_id,
        scope_decision = "not_applicable",
        request_id = event.request_id,
        tenant = event.tenant,
        status = event.status,
        error_code = error_code,
        llm_outcome = event.outcome,
        llm_reason = event.reason,
        llm_provider = event.provider,
        llm_model = event.model,
        llm_context_cell_count = event.context_cell_count,
        llm_citation_count = event.citation_count,
        llm_request_api_key_present = event.request_api_key_present,
        llm_prompt_body_logged = false,
        llm_secrets_logged = false,
    );
    if let Some(sink) = sink {
        if let Err(error) = sink.append(&mut record) {
            tracing::error!(
                target: "cortexdb_audit",
                audit_event = "sink_error",
                error = %error,
            );
        }
    }
}

pub(super) fn hash_value(fields: LlmInferenceAuditFields<'_>) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}",
        fields.outcome,
        fields.reason,
        fields.provider,
        fields.model,
        fields.context_cell_count,
        fields.citation_count,
        fields.request_api_key_present,
        fields.prompt_body_logged,
        fields.secrets_logged,
    )
}

fn unix_time_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}
