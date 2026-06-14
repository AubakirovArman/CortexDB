use super::audit::{classify, emit_http_response, AuditAction, AuditSink, HttpResponseAudit};
use super::audit_chain;

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
    assert_eq!(classify("POST", "/v1/feedback"), AuditAction::Memory);
    assert_eq!(classify("GET", "/v1/feedback/stats"), AuditAction::Memory);
    assert_eq!(classify("POST", "/v1/compact"), AuditAction::Admin);
    assert_eq!(
        classify("POST", "/v1/admin/auth/principal"),
        AuditAction::Admin
    );
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
    assert_eq!(value["chain_id"], "cortexdb.audit.chain.v1");
    assert_eq!(value["sequence"], 1);
    assert_eq!(value["prev_hash"], "0000000000000000");
    assert!(value["event_hash"]
        .as_str()
        .is_some_and(audit_chain::is_hex_hash));
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

#[test]
fn audit_sink_continues_chain_when_reopened() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("audit").join("http.jsonl");
    let first_hash = {
        let sink = AuditSink::open(&path).unwrap();
        emit_http_response(
            HttpResponseAudit {
                method: "GET",
                path: "/v1/health",
                tenant: "default",
                request_id: "req-1",
                principal_id: None,
                auth_role: None,
                auth_agent_id: None,
                status: 200,
                error_code: None,
                duration_ms: 1,
            },
            Some(&sink),
        );
        let raw = std::fs::read_to_string(&path).unwrap();
        let value = serde_json::from_str::<serde_json::Value>(raw.trim()).unwrap();
        value["event_hash"].as_str().unwrap().to_owned()
    };

    let sink = AuditSink::open(&path).unwrap();
    emit_http_response(
        HttpResponseAudit {
            method: "GET",
            path: "/v1/stats",
            tenant: "default",
            request_id: "req-2",
            principal_id: Some("admin-a"),
            auth_role: Some("admin"),
            auth_agent_id: None,
            status: 200,
            error_code: None,
            duration_ms: 2,
        },
        Some(&sink),
    );

    let raw = std::fs::read_to_string(path).unwrap();
    let values = raw
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(values[1]["sequence"], 2);
    assert_eq!(values[1]["prev_hash"], first_hash);
}

#[test]
fn audit_sink_rejects_corrupt_chain_tail() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("audit").join("http.jsonl");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(
        &path,
        r#"{"schema_version":"cortexdb.audit.v1","sequence":1,"event_hash":"not-hex"}
"#,
    )
    .unwrap();

    let error = match AuditSink::open(&path) {
        Ok(_) => panic!("corrupt audit chain tail should fail"),
        Err(error) => error,
    };
    assert!(
        error.to_string().contains("invalid event_hash"),
        "unexpected error: {error}"
    );
}
