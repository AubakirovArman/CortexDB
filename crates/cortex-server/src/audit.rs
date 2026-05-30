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
        "/" | "/dashboard" => AuditAction::Admin,
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

pub(crate) fn emit_http_response(
    method: &str,
    path: &str,
    tenant: &str,
    status: u16,
    error_code: Option<&str>,
    duration_ms: u64,
) {
    let action = classify(method, path).as_str();
    tracing::info!(
        target: "cortexdb_audit",
        audit_event = "http_response",
        audit_action = action,
        method = method,
        path = path,
        tenant = tenant,
        status = status,
        error_code = error_code.unwrap_or(""),
        duration_ms = duration_ms,
    );
}

#[cfg(test)]
mod tests {
    use super::{classify, AuditAction};

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
}
