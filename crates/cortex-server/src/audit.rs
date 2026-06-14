use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::audit_chain::{self, AUDIT_CHAIN_ID, AUDIT_CHAIN_ZERO_HASH};
use crate::dashboard;

mod llm;
pub(crate) use llm::{emit_llm_inference_decision, LlmInferenceDecisionAudit};

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
struct AuditRecord<'a> {
    schema_version: &'static str,
    audit_event: &'static str,
    audit_action: &'static str,
    chain_id: &'static str,
    sequence: u64,
    prev_hash: String,
    event_hash: String,
    principal_id: &'a str,
    auth_role: &'a str,
    auth_agent_id: Option<u64>,
    method: &'a str,
    path: &'a str,
    tenant: &'a str,
    request_id: &'a str,
    status: u16,
    error_code: &'a str,
    duration_ms: u64,
    unix_time_ms: u128,
    #[serde(skip_serializing_if = "Option::is_none")]
    llm: Option<llm::LlmInferenceAuditFields<'a>>,
}

pub(crate) struct AuditSink {
    state: Mutex<AuditSinkState>,
}

struct AuditSinkState {
    file: File,
    next_sequence: u64,
    prev_hash: String,
}

impl AuditSink {
    pub(crate) fn open(path: &Path) -> io::Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let (next_sequence, prev_hash) = audit_chain::tail(path)?;
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(Self {
            state: Mutex::new(AuditSinkState {
                file,
                next_sequence,
                prev_hash,
            }),
        })
    }

    fn append(&self, record: &mut AuditRecord<'_>) -> io::Result<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|e| io::Error::other(e.to_string()))?;
        record.sequence = state.next_sequence;
        record.prev_hash = state.prev_hash.clone();
        record.event_hash = audit_event_hash(record);
        serde_json::to_writer(&mut state.file, record).map_err(io::Error::other)?;
        state.file.write_all(b"\n")?;
        state.file.flush()?;
        state.file.sync_data()?;
        state.next_sequence = state
            .next_sequence
            .checked_add(1)
            .ok_or_else(|| io::Error::other("audit chain sequence overflow"))?;
        state.prev_hash = record.event_hash.clone();
        Ok(())
    }
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
}

pub(crate) fn emit_http_response(event: HttpResponseAudit<'_>, sink: Option<&AuditSink>) {
    let action = classify(event.method, event.path).as_str();
    let error_code = event.error_code.unwrap_or("");
    let unix_time_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    let mut record = AuditRecord {
        schema_version: "cortexdb.audit.v1",
        audit_event: "http_response",
        audit_action: action,
        chain_id: AUDIT_CHAIN_ID,
        sequence: 0,
        prev_hash: AUDIT_CHAIN_ZERO_HASH.to_owned(),
        event_hash: String::new(),
        principal_id: event.principal_id.unwrap_or(""),
        auth_role: event.auth_role.unwrap_or(""),
        auth_agent_id: event.auth_agent_id,
        method: event.method,
        path: event.path,
        tenant: event.tenant,
        request_id: event.request_id,
        status: event.status,
        error_code,
        duration_ms: event.duration_ms,
        unix_time_ms,
        llm: None,
    };
    tracing::info!(
        target: "cortexdb_audit",
        audit_event = "http_response",
        audit_action = action,
        principal_id = event.principal_id.unwrap_or(""),
        auth_role = event.auth_role.unwrap_or(""),
        auth_agent_id = event.auth_agent_id,
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

fn audit_event_hash(record: &AuditRecord<'_>) -> String {
    audit_chain::event_hash(&[
        ("chain_id", record.chain_id.to_owned()),
        ("sequence", record.sequence.to_string()),
        ("prev_hash", record.prev_hash.clone()),
        ("schema_version", record.schema_version.to_owned()),
        ("audit_event", record.audit_event.to_owned()),
        ("audit_action", record.audit_action.to_owned()),
        ("principal_id", record.principal_id.to_owned()),
        ("auth_role", record.auth_role.to_owned()),
        (
            "auth_agent_id",
            record
                .auth_agent_id
                .map(|value| value.to_string())
                .unwrap_or_default(),
        ),
        ("method", record.method.to_owned()),
        ("path", record.path.to_owned()),
        ("tenant", record.tenant.to_owned()),
        ("request_id", record.request_id.to_owned()),
        ("status", record.status.to_string()),
        ("error_code", record.error_code.to_owned()),
        ("duration_ms", record.duration_ms.to_string()),
        ("unix_time_ms", record.unix_time_ms.to_string()),
        ("llm", record.llm.map(llm::hash_value).unwrap_or_default()),
    ])
}
