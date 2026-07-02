use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::audit_chain::{self, AUDIT_CHAIN_ID, AUDIT_CHAIN_ZERO_HASH};
use crate::config::AuditLogFsyncPolicy;
use crate::dashboard;

mod llm;
mod sink;
pub(crate) use llm::{emit_llm_inference_decision, LlmInferenceDecisionAudit};
pub(crate) use sink::{AuditSink, AuditSinkOptions};

pub(super) const AUDIT_SCHEMA_VERSION_V1: &str = "cortexdb.audit.v1";
pub(super) const AUDIT_SCHEMA_VERSION_V2: &str = "cortexdb.audit.v2";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AuditAction {
    Admin,
    Aql,
    Context,
    Delete,
    Health,
    Ingest,
    Inference,
    Memory,
    Metrics,
    Read,
    Search,
    Verify,
    Write,
    Other,
}

impl AuditAction {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Admin => "admin",
            Self::Aql => "aql",
            Self::Context => "context",
            Self::Delete => "delete",
            Self::Health => "health",
            Self::Ingest => "ingest",
            Self::Inference => "inference",
            Self::Memory => "memory",
            Self::Metrics => "metrics",
            Self::Read => "read",
            Self::Search => "search",
            Self::Verify => "verify",
            Self::Write => "write",
            Self::Other => "other",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ScopeDecision {
    Allowed,
    Denied,
    NotApplicable,
}

impl ScopeDecision {
    fn as_str(self) -> &'static str {
        match self {
            Self::Allowed => "allowed",
            Self::Denied => "denied",
            Self::NotApplicable => "not_applicable",
        }
    }
}

pub(crate) fn classify(method: &str, path: &str) -> AuditAction {
    match path {
        _ if dashboard::is_page(path) => AuditAction::Admin,
        _ if path.starts_with("/v1/admin/") => AuditAction::Admin,
        _ if path == "/v1/agents" || path.starts_with("/v1/agents/") => AuditAction::Admin,
        "/v1/health" => AuditAction::Health,
        "/v1/stats" | "/v1/validate" | "/v1/flush" | "/v1/compact" => AuditAction::Admin,
        "/v1/metrics" | "/v1/ann/metrics" => AuditAction::Metrics,
        "/v1/aql" => AuditAction::Aql,
        "/v1/context" => AuditAction::Context,
        "/v1/inference" => AuditAction::Inference,
        "/v1/verify" => AuditAction::Verify,
        "/v1/search" | "/v1/search/explain" | "/v1/search/ann-evaluate" => AuditAction::Search,
        "/v1/remember" | "/v1/forget" | "/v1/feedback" | "/v1/feedback/stats" => {
            AuditAction::Memory
        }
        _ if path.starts_with("/v1/ingest/") => AuditAction::Ingest,
        "/v1/cell" => match method {
            "GET" => AuditAction::Read,
            "POST" => AuditAction::Write,
            "DELETE" => AuditAction::Delete,
            _ => AuditAction::Other,
        },
        _ => AuditAction::Other,
    }
}

#[derive(Serialize)]
pub(super) struct AuditRecord<'a> {
    schema_version: &'static str,
    audit_event: &'static str,
    audit_action: &'static str,
    chain_id: &'static str,
    sequence: u64,
    prev_hash: String,
    event_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    mac_key_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    event_mac: Option<String>,
    principal_id: &'a str,
    auth_role: &'a str,
    auth_agent_id: Option<u64>,
    scope_decision: &'static str,
    method: &'a str,
    path: &'a str,
    tenant: &'a str,
    request_id: &'a str,
    status: u16,
    error_code: &'a str,
    duration_ms: u64,
    unix_time_ms: u128,
    #[serde(skip_serializing_if = "Option::is_none")]
    accountability_receipt_hash: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    llm: Option<llm::LlmInferenceAuditFields<'a>>,
}

pub(crate) struct HttpResponseAudit<'a> {
    pub(crate) method: &'a str,
    pub(crate) path: &'a str,
    pub(crate) tenant: &'a str,
    pub(crate) request_id: &'a str,
    pub(crate) principal_id: Option<&'a str>,
    pub(crate) auth_role: Option<&'a str>,
    pub(crate) auth_agent_id: Option<u64>,
    pub(crate) status: u16,
    pub(crate) error_code: Option<&'a str>,
    pub(crate) duration_ms: u64,
    pub(crate) accountability_receipt_hash: Option<&'a str>,
}

pub(crate) fn emit_http_response(event: HttpResponseAudit<'_>, sink: Option<&AuditSink>) {
    let action = classify(event.method, event.path);
    let error_code = event.error_code.unwrap_or("");
    let scope_decision = scope_decision(action, event.status, error_code).as_str();
    let unix_time_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    let mut record = AuditRecord {
        schema_version: AUDIT_SCHEMA_VERSION_V1,
        audit_event: "http_response",
        audit_action: action.as_str(),
        chain_id: AUDIT_CHAIN_ID,
        sequence: 0,
        prev_hash: AUDIT_CHAIN_ZERO_HASH.to_owned(),
        event_hash: String::new(),
        mac_key_id: None,
        event_mac: None,
        principal_id: event.principal_id.unwrap_or(""),
        auth_role: event.auth_role.unwrap_or(""),
        auth_agent_id: event.auth_agent_id,
        scope_decision,
        method: event.method,
        path: event.path,
        tenant: event.tenant,
        request_id: event.request_id,
        status: event.status,
        error_code,
        duration_ms: event.duration_ms,
        unix_time_ms,
        accountability_receipt_hash: event.accountability_receipt_hash,
        llm: None,
    };
    tracing::info!(
        target: "cortexdb_audit",
        audit_event = "http_response",
        audit_action = action.as_str(),
        principal_id = event.principal_id.unwrap_or(""),
        auth_role = event.auth_role.unwrap_or(""),
        auth_agent_id = event.auth_agent_id,
        scope_decision = scope_decision,
        method = event.method,
        path = event.path,
        tenant = event.tenant,
        request_id = event.request_id,
        status = event.status,
        error_code = error_code,
        duration_ms = event.duration_ms,
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

pub(super) fn audit_event_hash(record: &AuditRecord<'_>) -> String {
    audit_chain::event_hash(&audit_event_fields(record))
}

pub(super) fn audit_event_mac(record: &AuditRecord<'_>, key: &cortex_crypto::MacKey) -> String {
    audit_chain::event_mac(key, &audit_event_fields(record))
}

fn audit_event_fields(record: &AuditRecord<'_>) -> Vec<(&'static str, String)> {
    vec![
        ("chain_id", record.chain_id.to_owned()),
        ("sequence", record.sequence.to_string()),
        ("prev_hash", record.prev_hash.clone()),
        ("schema_version", record.schema_version.to_owned()),
        ("audit_event", record.audit_event.to_owned()),
        ("audit_action", record.audit_action.to_owned()),
        (
            "mac_key_id",
            record.mac_key_id.as_deref().unwrap_or_default().to_owned(),
        ),
        ("principal_id", record.principal_id.to_owned()),
        ("auth_role", record.auth_role.to_owned()),
        (
            "auth_agent_id",
            record
                .auth_agent_id
                .map(|value| value.to_string())
                .unwrap_or_default(),
        ),
        ("scope_decision", record.scope_decision.to_owned()),
        ("method", record.method.to_owned()),
        ("path", record.path.to_owned()),
        ("tenant", record.tenant.to_owned()),
        ("request_id", record.request_id.to_owned()),
        ("status", record.status.to_string()),
        ("error_code", record.error_code.to_owned()),
        ("duration_ms", record.duration_ms.to_string()),
        ("unix_time_ms", record.unix_time_ms.to_string()),
        (
            "accountability_receipt_hash",
            record
                .accountability_receipt_hash
                .unwrap_or_default()
                .to_owned(),
        ),
        ("llm", record.llm.map(llm::hash_value).unwrap_or_default()),
    ]
}

fn scope_decision(action: AuditAction, status: u16, error_code: &str) -> ScopeDecision {
    if error_code == "permission_denied" || status == 403 {
        return ScopeDecision::Denied;
    }
    if status < 400 && action_has_scope_decision(action) {
        return ScopeDecision::Allowed;
    }
    ScopeDecision::NotApplicable
}

fn action_has_scope_decision(action: AuditAction) -> bool {
    use AuditAction::*;
    matches!(
        action,
        Aql | Context | Delete | Ingest | Memory | Read | Search | Verify | Write
    )
}

impl AuditSinkOptions {
    pub(crate) fn from_parts(
        rotate_bytes: Option<u64>,
        fsync_policy: AuditLogFsyncPolicy,
        mac_key: Option<crate::config::AuditMacKey>,
    ) -> Self {
        Self {
            rotate_bytes,
            fsync_policy,
            mac_key,
        }
    }

    #[cfg(test)]
    pub(crate) fn test_default() -> Self {
        let mac_key = crate::config::AuditMacKey::from_hex(
            "test-audit-key",
            "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
        )
        .expect("test audit MAC key is valid");
        Self::from_parts(None, AuditLogFsyncPolicy::Always, Some(mac_key))
    }
}
