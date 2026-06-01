use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use crate::dashboard;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AuditAction {
    Admin,
    Aql,
    Context,
    Delete,
    Health,
    Ingest,
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
        "/v1/health" => AuditAction::Health,
        "/v1/stats" | "/v1/validate" | "/v1/flush" | "/v1/compact" => AuditAction::Admin,
        "/v1/metrics" | "/v1/ann/metrics" => AuditAction::Metrics,
        "/v1/aql" => AuditAction::Aql,
        "/v1/context" => AuditAction::Context,
        "/v1/verify" => AuditAction::Verify,
        "/v1/search" | "/v1/search/explain" | "/v1/search/ann-evaluate" => AuditAction::Search,
        "/v1/remember" | "/v1/forget" => AuditAction::Memory,
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
}

pub(crate) struct AuditSink {
    file: Mutex<File>,
}

impl AuditSink {
    pub(crate) fn open(path: &Path) -> io::Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(Self {
            file: Mutex::new(file),
        })
    }

    fn append(&self, record: &AuditRecord<'_>) -> io::Result<()> {
        let mut file = self
            .file
            .lock()
            .map_err(|e| io::Error::other(e.to_string()))?;
        serde_json::to_writer(&mut *file, record).map_err(io::Error::other)?;
        file.write_all(b"\n")?;
        file.flush()?;
        file.sync_data()?;
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
    let record = AuditRecord {
        schema_version: "cortexdb.audit.v1",
        audit_event: "http_response",
        audit_action: action,
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
        if let Err(error) = sink.append(&record) {
            tracing::error!(
                target: "cortexdb_audit",
                audit_event = "sink_error",
                error = %error,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{classify, emit_http_response, AuditAction, AuditSink, HttpResponseAudit};

    #[test]
    fn classify_core_api_actions() {
        assert_eq!(classify("GET", "/v1/cell"), AuditAction::Read);
        assert_eq!(classify("POST", "/v1/cell"), AuditAction::Write);
        assert_eq!(classify("DELETE", "/v1/cell"), AuditAction::Delete);
        assert_eq!(classify("POST", "/v1/aql"), AuditAction::Aql);
        assert_eq!(classify("POST", "/v1/context"), AuditAction::Context);
        assert_eq!(classify("POST", "/v1/verify"), AuditAction::Verify);
        assert_eq!(classify("POST", "/v1/search"), AuditAction::Search);
        assert_eq!(classify("POST", "/v1/ingest/text"), AuditAction::Ingest);
        assert_eq!(classify("POST", "/v1/remember"), AuditAction::Memory);
        assert_eq!(classify("POST", "/v1/compact"), AuditAction::Admin);
        assert_eq!(classify("GET", "/v1/metrics"), AuditAction::Metrics);
    }

    #[test]
    fn audit_sink_writes_jsonl_without_body_or_query() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit").join("http.jsonl");
        let sink = AuditSink::open(&path).unwrap();

        emit_http_response(
            HttpResponseAudit {
                method: "POST",
                path: "/v1/cell",
                tenant: "tenant-a",
                request_id: "req-123",
                principal_id: Some("principal-a"),
                auth_role: Some("data"),
                auth_agent_id: Some(7),
                status: 403,
                error_code: Some("permission_denied"),
                duration_ms: 12,
            },
            Some(&sink),
        );

        let line = std::fs::read_to_string(path).unwrap();
        let value = serde_json::from_str::<serde_json::Value>(line.trim()).unwrap();
        assert_eq!(value["schema_version"], "cortexdb.audit.v1");
        assert_eq!(value["audit_event"], "http_response");
        assert_eq!(value["audit_action"], "write");
        assert_eq!(value["principal_id"], "principal-a");
        assert_eq!(value["auth_role"], "data");
        assert_eq!(value["auth_agent_id"], 7);
        assert_eq!(value["method"], "POST");
        assert_eq!(value["path"], "/v1/cell");
        assert_eq!(value["tenant"], "tenant-a");
        assert_eq!(value["request_id"], "req-123");
        assert_eq!(value["status"], 403);
        assert_eq!(value["error_code"], "permission_denied");
        assert!(!line.contains("secret_payload"));
        assert!(!line.contains('?'));
    }
}
