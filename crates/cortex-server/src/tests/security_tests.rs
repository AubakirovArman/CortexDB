use std::collections::BTreeSet;
use std::thread;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, ScopeId, Q16_ZERO};
use cortex_core::{CellId, KnowledgeCell, KnowledgeCellMetadata, KnowledgeCellType};
use cortex_engine::{scope_id, Database};

use crate::{handle_http_with_options, ServerOptions};

#[test]
fn v1_api_requires_bearer_token_when_configured() {
    let dir = tempfile::tempdir().unwrap();
    let options = ServerOptions {
        auth_token: Some("secret".to_owned()),
        ..Default::default()
    };
    let denied = handle_http_with_options(dir.path(), "GET /v1/health HTTP/1.1\r\n\r\n", &options);
    assert!(denied.contains("401 Unauthorized"));

    let allowed = handle_http_with_options(
        dir.path(),
        "GET /v1/health HTTP/1.1\r\nAuthorization: Bearer secret\r\n\r\n",
        &options,
    );
    assert!(allowed.contains(r#""status":"ok""#));
}

#[test]
fn v1_api_rejects_wrong_bearer_token_when_configured() {
    let dir = tempfile::tempdir().unwrap();
    let options = ServerOptions {
        auth_token: Some("secret".to_owned()),
        ..Default::default()
    };
    let denied = handle_http_with_options(
        dir.path(),
        "GET /v1/health HTTP/1.1\r\nAuthorization: Bearer wrong-secret\r\n\r\n",
        &options,
    );
    assert!(denied.contains("401 Unauthorized"));
    assert!(denied.contains("unauthorized"));
}

#[test]
fn auth_agent_view_blocks_unreadable_scope_over_http() {
    let dir = tempfile::tempdir().unwrap();
    {
        let db = Database::open(dir.path()).unwrap();
        db.save_agent_view(&agent_view(AgentId(7), "finance", true))
            .unwrap();
    }
    let options = ServerOptions {
        auth_token: Some("secret".to_owned()),
        auth_agent_id: Some(7),
        ..Default::default()
    };

    let denied = handle_http_with_options(
        dir.path(),
        "POST /v1/search?scope=secret&q=budget HTTP/1.1\r\nAuthorization: Bearer secret\r\n\r\n",
        &options,
    );
    assert!(
        denied.contains("403 Forbidden"),
        "unreadable scope should be denied: {denied}"
    );
    assert!(
        denied.contains("permission_denied"),
        "denial should use stable permission code: {denied}"
    );

    let allowed = handle_http_with_options(
        dir.path(),
        "POST /v1/search?scope=finance&q=budget HTTP/1.1\r\nAuthorization: Bearer secret\r\n\r\n",
        &options,
    );
    assert!(
        allowed.contains("200 OK"),
        "readable scope should be allowed: {allowed}"
    );
}

#[test]
fn auth_agent_view_blocks_unwritable_cell_scope_over_http() {
    let dir = tempfile::tempdir().unwrap();
    {
        let db = Database::open(dir.path()).unwrap();
        db.save_agent_view(&agent_view(AgentId(7), "finance", true))
            .unwrap();
    }
    let options = ServerOptions {
        auth_token: Some("secret".to_owned()),
        auth_agent_id: Some(7),
        ..Default::default()
    };

    let denied = handle_http_with_options(
        dir.path(),
        "POST /v1/cell?cell_id=1 HTTP/1.1\r\nAuthorization: Bearer secret\r\n\r\nscope=secret\n\nhidden",
        &options,
    );
    assert!(
        denied.contains("403 Forbidden"),
        "unwritable payload scope should be denied: {denied}"
    );
    assert!(
        denied.contains("permission_denied"),
        "denial should use stable permission code: {denied}"
    );

    let allowed = handle_http_with_options(
        dir.path(),
        "POST /v1/cell?cell_id=2 HTTP/1.1\r\nAuthorization: Bearer secret\r\n\r\nscope=finance\n\nvisible",
        &options,
    );
    assert!(
        allowed.contains("200 OK"),
        "writable payload scope should be allowed: {allowed}"
    );
}

#[test]
fn auth_agent_view_uses_descriptor_scope_for_cell_read_over_http() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.save_agent_view(&agent_view(AgentId(7), "project:investments", true))
            .unwrap();
        db.put_knowledge_cell(
            CellId(9),
            KnowledgeCell::new(
                KnowledgeCellMetadata {
                    scope: "tenant:private".to_owned(),
                    status: "ready".to_owned(),
                    cell_type: KnowledgeCellType::Raw,
                    ..Default::default()
                },
                b"scope=project:investments\nstatus=ready\n\nhidden spoof".to_vec(),
            ),
        )
        .unwrap();
    }
    let options = ServerOptions {
        auth_token: Some("secret".to_owned()),
        auth_agent_id: Some(7),
        ..Default::default()
    };

    let denied = handle_http_with_options(
        dir.path(),
        "GET /v1/cell?cell_id=9 HTTP/1.1\r\nAuthorization: Bearer secret\r\n\r\n",
        &options,
    );
    assert!(
        denied.contains("403 Forbidden"),
        "descriptor scope should deny spoofed payload reads: {denied}"
    );
    assert!(denied.contains("permission_denied"));
}

#[test]
fn tenant_realms_isolate_cell_data_over_http() {
    let dir = tempfile::tempdir().unwrap();

    let alpha_put = concat!(
        "POST /v1/cell?tenant=alpha&cell_id=1 HTTP/1.1\r\n\r\n",
        "scope=project:investments\nstatus=ready\nalpha-only-payload"
    );
    let beta_put = concat!(
        "POST /v1/cell?tenant=beta&cell_id=1 HTTP/1.1\r\n\r\n",
        "scope=project:investments\nstatus=ready\nbeta-only-payload"
    );
    assert!(
        handle_http_with_options(dir.path(), alpha_put, &ServerOptions::default())
            .contains(r#""seq":1"#)
    );
    assert!(
        handle_http_with_options(dir.path(), beta_put, &ServerOptions::default())
            .contains(r#""seq":1"#)
    );

    let alpha_get = handle_http_with_options(
        dir.path(),
        "GET /v1/cell?tenant=alpha&cell_id=1 HTTP/1.1\r\n\r\n",
        &ServerOptions::default(),
    );
    assert!(alpha_get.contains("alpha-only-payload"));
    assert!(!alpha_get.contains("beta-only-payload"));

    let beta_get = handle_http_with_options(
        dir.path(),
        "GET /v1/cell?tenant=beta&cell_id=1 HTTP/1.1\r\n\r\n",
        &ServerOptions::default(),
    );
    assert!(beta_get.contains("beta-only-payload"));
    assert!(!beta_get.contains("alpha-only-payload"));

    let default_get = handle_http_with_options(
        dir.path(),
        "GET /v1/cell?cell_id=1 HTTP/1.1\r\n\r\n",
        &ServerOptions::default(),
    );
    assert!(default_get.contains(r#""cell":null"#));

    assert!(dir.path().join("realms").join("alpha").is_dir());
    assert!(dir.path().join("realms").join("beta").is_dir());
}

#[test]
fn parallel_tenant_realms_do_not_share_state() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    let mut handles = Vec::new();

    for index in 0..8u64 {
        let root = root.clone();
        handles.push(thread::spawn(move || {
            let tenant = format!("tenant_{index}");
            let payload = format!("tenant-{index}-payload");
            let put = format!(
                "POST /v1/cell?tenant={tenant}&cell_id=1 HTTP/1.1\r\n\r\nscope=project:investments\nstatus=ready\n{payload}"
            );
            let put_response = handle_http_with_options(&root, &put, &ServerOptions::default());
            assert!(
                put_response.contains(r#""seq":1"#),
                "put failed for {tenant}: {put_response}"
            );

            let get = format!("GET /v1/cell?tenant={tenant}&cell_id=1 HTTP/1.1\r\n\r\n");
            let get_response = handle_http_with_options(&root, &get, &ServerOptions::default());
            assert!(
                get_response.contains(&payload),
                "get failed for {tenant}: {get_response}"
            );
            (tenant, payload)
        }));
    }

    let completed = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect::<Vec<_>>();

    for (tenant, payload) in completed {
        assert!(dir.path().join("realms").join(&tenant).is_dir());
        let response = handle_http_with_options(
            dir.path(),
            &format!("GET /v1/cell?tenant={tenant}&cell_id=1 HTTP/1.1\r\n\r\n"),
            &ServerOptions::default(),
        );
        assert!(response.contains(&payload));
    }
}

#[test]
fn auth_agent_id_requires_auth_token_in_server_options() {
    let dir = tempfile::tempdir().unwrap();
    let options = ServerOptions {
        auth_agent_id: Some(7),
        ..Default::default()
    };
    let error = crate::serve_with_options(dir.path(), "127.0.0.1:0", options).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
}

#[test]
fn audit_log_file_records_route_metadata_without_query() {
    let dir = tempfile::tempdir().unwrap();
    let audit_path = dir.path().join("audit").join("http.jsonl");
    let addr = "127.0.0.1:0";
    let listener = std::net::TcpListener::bind(addr).unwrap();
    let local_addr = listener.local_addr().unwrap();
    drop(listener);

    let root_path = dir.path().join("db");
    let audit_path_for_server = audit_path.clone();
    std::thread::spawn(move || {
        let options = ServerOptions {
            audit_log_enabled: true,
            audit_log_path: Some(audit_path_for_server),
            ..Default::default()
        };
        let _ = crate::serve_with_options(&root_path, &local_addr.to_string(), options);
    });

    std::thread::sleep(std::time::Duration::from_millis(100));
    let response = request(local_addr, "GET /v1/health?tenant=alpha HTTP/1.1\r\n\r\n");
    assert!(response.contains("200 OK"), "health failed: {response}");

    let (line, value) = read_audit_event(&audit_path, "health", "/v1/health");
    assert_eq!(value["audit_event"], "http_response");
    assert_eq!(value["audit_action"], "health");
    assert_eq!(value["method"], "GET");
    assert_eq!(value["path"], "/v1/health");
    assert_eq!(value["tenant"], "alpha");
    assert!(value["request_id"]
        .as_str()
        .is_some_and(|request_id| request_id.starts_with("cortexdb-")));
    assert_eq!(value["status"], 200);
    assert!(!line.contains("tenant=alpha"));
}

#[test]
fn audit_log_file_redacts_ingestion_query_and_body() {
    let dir = tempfile::tempdir().unwrap();
    let audit_path = dir.path().join("audit").join("http.jsonl");
    let addr = "127.0.0.1:0";
    let listener = std::net::TcpListener::bind(addr).unwrap();
    let local_addr = listener.local_addr().unwrap();
    drop(listener);

    let root_path = dir.path().join("db");
    let audit_path_for_server = audit_path.clone();
    std::thread::spawn(move || {
        let options = ServerOptions {
            audit_log_enabled: true,
            audit_log_path: Some(audit_path_for_server),
            ..Default::default()
        };
        let _ = crate::serve_with_options(&root_path, &local_addr.to_string(), options);
    });

    std::thread::sleep(std::time::Duration::from_millis(100));
    let ingest_request = "POST /v1/ingest/text?tenant=alpha&scope=project%3Ainvestments&source=secret-source HTTP/1.1\r\ncontent-length: 20\r\n\r\nsecret payload token";
    let response = request(local_addr, ingest_request);
    assert!(response.contains("200 OK"), "ingest failed: {response}");

    let (line, value) = read_audit_event(&audit_path, "ingest", "/v1/ingest/text");
    assert_eq!(value["audit_action"], "ingest");
    assert_eq!(value["path"], "/v1/ingest/text");
    assert_eq!(value["tenant"], "alpha");
    assert!(!line.contains("secret-source"));
    assert!(!line.contains("secret payload token"));
    assert!(!line.contains("project%3Ainvestments"));
}

#[test]
fn audit_log_file_records_policy_store_principal_without_token() {
    let dir = tempfile::tempdir().unwrap();
    let audit_path = dir.path().join("audit").join("http.jsonl");
    let policy_store = dir.path().join("auth-policy.json");
    std::fs::write(
        &policy_store,
        r#"{
          "schema_version": "cortexdb.auth_policy.v1",
          "principals": [
            {"principal_id":"finance-agent","token":"finance-token","role":"data","agent_id":11}
          ]
        }"#,
    )
    .unwrap();
    let addr = "127.0.0.1:0";
    let listener = std::net::TcpListener::bind(addr).unwrap();
    let local_addr = listener.local_addr().unwrap();
    drop(listener);

    let root_path = dir.path().join("db");
    let audit_path_for_server = audit_path.clone();
    std::thread::spawn(move || {
        let options = ServerOptions {
            auth_policy_store_file: Some(policy_store),
            audit_log_enabled: true,
            audit_log_path: Some(audit_path_for_server),
            ..Default::default()
        };
        let _ = crate::serve_with_options(&root_path, &local_addr.to_string(), options);
    });

    std::thread::sleep(std::time::Duration::from_millis(100));
    let response = request(
        local_addr,
        "GET /v1/health?tenant=alpha HTTP/1.1\r\nAuthorization: Bearer finance-token\r\n\r\n",
    );
    assert!(
        response.contains("200 OK"),
        "policy-store principal should access health: {response}"
    );

    let (line, value) = read_audit_event(&audit_path, "health", "/v1/health");
    assert_eq!(value["audit_event"], "http_response");
    assert_eq!(value["audit_action"], "health");
    assert_eq!(value["principal_id"], "finance-agent");
    assert_eq!(value["auth_role"], "data");
    assert_eq!(value["auth_agent_id"], 11);
    assert!(!line.contains("finance-token"));
}

#[test]
fn malicious_ingestion_scope_bypass_is_denied_by_agent_view() {
    let dir = tempfile::tempdir().unwrap();
    {
        let db = Database::open(dir.path()).unwrap();
        db.save_agent_view(&agent_view(AgentId(7), "finance", true))
            .unwrap();
    }
    let options = ServerOptions {
        auth_token: Some("secret".to_owned()),
        auth_agent_id: Some(7),
        ..Default::default()
    };

    let denied = handle_http_with_options(
        dir.path(),
        "POST /v1/ingest/text?scope=..%2F..%2Fsecret&source=attack HTTP/1.1\r\nAuthorization: Bearer secret\r\ncontent-length: 6\r\n\r\nbudget",
        &options,
    );
    assert!(
        denied.contains("403 Forbidden"),
        "malicious ingest scope must be denied: {denied}"
    );
    assert!(
        denied.contains("permission_denied"),
        "denial should use stable permission code: {denied}"
    );
    assert!(
        !denied.contains("budget"),
        "denial should not echo request body: {denied}"
    );
}

#[test]
fn x_request_id_is_propagated_or_generated() {
    let dir = tempfile::tempdir().unwrap();
    let addr = "127.0.0.1:0";
    let listener = std::net::TcpListener::bind(addr).unwrap();
    let local_addr = listener.local_addr().unwrap();
    drop(listener);

    let root_path = dir.path().to_owned();
    std::thread::spawn(move || {
        let _ = crate::serve(&root_path, &local_addr.to_string());
    });

    let propagated = request(
        local_addr,
        "GET /v1/health HTTP/1.1\r\nx-request-id: req-test-123\r\n\r\n",
    )
    .to_ascii_lowercase();
    assert!(propagated.contains("x-request-id: req-test-123"));

    let generated = request(local_addr, "GET /v1/health HTTP/1.1\r\n\r\n").to_ascii_lowercase();
    assert!(generated.contains("x-request-id: cortexdb-"));

    let metrics = request(local_addr, "GET /v1/metrics HTTP/1.1\r\n\r\n");
    assert!(
        metrics.contains(r#""request_id_client_provided":1"#),
        "client request-id counter missing from metrics: {metrics}"
    );
    assert!(
        metrics.contains(r#""request_id_generated":2"#),
        "generated request-id counter missing from metrics: {metrics}"
    );
}

#[test]
fn test_server_concurrency_and_size_limit() {
    let dir = tempfile::tempdir().unwrap();
    let addr = "127.0.0.1:0";
    let listener = std::net::TcpListener::bind(addr).unwrap();
    let local_addr = listener.local_addr().unwrap();
    drop(listener);

    let root_path = dir.path().to_owned();
    std::thread::spawn(move || {
        let _ = crate::serve(&root_path, &local_addr.to_string());
    });

    {
        let huge_size = 2100 * 1024;
        let mut huge_request = Vec::with_capacity(huge_size + 100);
        huge_request.extend_from_slice(b"POST /put?cell_id=1 HTTP/1.1\r\nContent-Length: ");
        huge_request.extend_from_slice(huge_size.to_string().as_bytes());
        huge_request.extend_from_slice(b"\r\n\r\n");
        huge_request.resize(huge_request.len() + huge_size, b'A');

        let resp_str = request_bytes(local_addr, &huge_request);
        assert!(resp_str.contains("413 Payload Too Large"));
    }

    {
        let mut threads = vec![];
        for _ in 0..5 {
            threads.push(std::thread::spawn(move || {
                let resp_str = request(local_addr, "GET /v1/health HTTP/1.1\r\n\r\n");
                assert!(resp_str.contains(r#""status":"ok""#));
            }));
        }

        for thread in threads {
            thread.join().unwrap();
        }
    }
}

#[test]
fn test_tenant_validation_unit_cases() {
    // Accepted tenants
    assert!(crate::validate_tenant_id("default"));
    assert!(crate::validate_tenant_id("tenant1"));
    assert!(crate::validate_tenant_id("tenant-1"));
    assert!(crate::validate_tenant_id("tenant_1"));
    assert!(crate::validate_tenant_id("project_1"));
    // tenant:1 and project:investments are now rejected because ':' is
    // disallowed for cross-platform safety (Windows reserves it in paths).
    assert!(!crate::validate_tenant_id("tenant:1"));
    assert!(!crate::validate_tenant_id("project:investments"));

    // Rejected — path traversal patterns
    assert!(!crate::validate_tenant_id("../../escape"));
    assert!(!crate::validate_tenant_id("..%2f..%2fescape"));
    assert!(!crate::validate_tenant_id("a/b"));
    assert!(!crate::validate_tenant_id("a%2Fb"));
    assert!(!crate::validate_tenant_id("."));
    assert!(!crate::validate_tenant_id(".."));
    assert!(!crate::validate_tenant_id("../x"));

    // Rejected — length and empty
    assert!(!crate::validate_tenant_id(""));
    assert!(!crate::validate_tenant_id(&"a".repeat(65)));

    // Rejected — special characters
    assert!(!crate::validate_tenant_id("tenant@home"));
    assert!(!crate::validate_tenant_id("tenant space"));
    assert!(!crate::validate_tenant_id("tenant\nline"));
    assert!(!crate::validate_tenant_id("tenant:alpha"));
}

#[test]
fn test_query_param_percent_decoding() {
    // Scope with colon
    assert_eq!(
        crate::router::query_param_decoded("scope=project%3Ainvestments", "scope").unwrap(),
        "project:investments"
    );
    // Search query with space
    assert_eq!(
        crate::router::query_param_decoded("q=Solar%20Plant", "q").unwrap(),
        "Solar Plant"
    );
    // Plus sign (common form encoding)
    assert_eq!(
        crate::router::query_param_decoded("q=Solar+Plant", "q").unwrap(),
        "Solar Plant"
    );
    // Unencoded value passes through
    assert_eq!(
        crate::router::query_param_decoded("scope=finance", "scope").unwrap(),
        "finance"
    );
}

#[test]
fn test_tenant_path_traversal_over_http() {
    let dir = tempfile::tempdir().unwrap();
    let addr = "127.0.0.1:0";
    let listener = std::net::TcpListener::bind(addr).unwrap();
    let local_addr = listener.local_addr().unwrap();
    drop(listener);

    let root_path = dir.path().to_owned();
    std::thread::spawn(move || {
        let _ = crate::serve(&root_path, &local_addr.to_string());
    });

    let bad_tenants = [
        "../../escape",
        "..%2f..%2fescape",
        "a/b",
        "a%2Fb",
        ".",
        "..",
        "../x",
    ];

    for tenant in &bad_tenants {
        let req = format!("GET /v1/health?tenant={} HTTP/1.1\r\n\r\n", tenant);
        let resp_str = request(local_addr, &req);
        assert!(
            resp_str.contains("400 Bad Request"),
            "tenant='{}' should be rejected with 400, got: {}",
            tenant,
            resp_str
        );
        assert!(
            resp_str.contains("invalid_tenant"),
            "tenant='{}' response should contain invalid_tenant, got: {}",
            tenant,
            resp_str
        );
    }
}

#[test]
fn cors_preflight_is_only_enabled_for_configured_origin() {
    let dir = tempfile::tempdir().unwrap();
    let addr = "127.0.0.1:0";
    let listener = std::net::TcpListener::bind(addr).unwrap();
    let local_addr = listener.local_addr().unwrap();
    drop(listener);

    let root_path = dir.path().to_owned();
    std::thread::spawn(move || {
        let options = ServerOptions {
            cors_allowed_origin: Some("https://app.example".to_owned()),
            ..Default::default()
        };
        let _ = crate::serve_with_options(&root_path, &local_addr.to_string(), options);
    });

    let response = request(
        local_addr,
        "OPTIONS /v1/health HTTP/1.1\r\n\
         Origin: https://app.example\r\n\
         Access-Control-Request-Method: GET\r\n\
         Access-Control-Request-Headers: authorization,content-type\r\n\r\n",
    );
    let resp_str = response.to_ascii_lowercase();
    assert!(resp_str.contains("access-control-allow-origin: https://app.example"));
    assert!(resp_str.contains("access-control-allow-methods"));
    assert!(resp_str.contains("access-control-allow-headers"));
}

fn request_bytes(addr: std::net::SocketAddr, request: &[u8]) -> String {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::time::Duration;

    let mut last_err = None;
    for _ in 0..20 {
        match TcpStream::connect(addr) {
            Ok(mut stream) => {
                if let Err(err) = stream.write_all(request) {
                    last_err = Some(err);
                    std::thread::sleep(Duration::from_millis(50));
                    continue;
                }
                let mut response = [0u8; 4096];
                let read = match stream.read(&mut response) {
                    Ok(read) => read,
                    Err(err) => {
                        last_err = Some(err);
                        std::thread::sleep(Duration::from_millis(50));
                        continue;
                    }
                };
                return String::from_utf8_lossy(&response[..read]).to_string();
            }
            Err(err) => {
                last_err = Some(err);
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }

    panic!("failed to perform request after retries: {:?}", last_err);
}

#[test]
fn rate_limit_returns_typed_429_when_enabled() {
    let dir = tempfile::tempdir().unwrap();
    let addr = "127.0.0.1:0";
    let listener = std::net::TcpListener::bind(addr).unwrap();
    let local_addr = listener.local_addr().unwrap();
    drop(listener);

    let root_path = dir.path().to_owned();
    std::thread::spawn(move || {
        let options = ServerOptions {
            request_rate_limit_per_minute: Some(1),
            ..Default::default()
        };
        let _ = crate::serve_with_options(&root_path, &local_addr.to_string(), options);
    });

    std::thread::sleep(std::time::Duration::from_millis(100));

    let first = request(local_addr, "GET /v1/health HTTP/1.1\r\n\r\n");
    assert!(first.contains("200 OK"), "first request failed: {first}");

    let second = request(local_addr, "GET /v1/health HTTP/1.1\r\n\r\n");
    assert!(
        second.contains("429 Too Many Requests"),
        "second request should be rate limited: {second}"
    );
    assert!(
        second.contains("rate_limited"),
        "rate limit response should use typed error code: {second}"
    );
}

fn request(addr: std::net::SocketAddr, request: &str) -> String {
    request_bytes(addr, request.as_bytes())
}

fn read_audit_event(
    path: &std::path::Path,
    action: &str,
    route_path: &str,
) -> (String, serde_json::Value) {
    use std::time::{Duration, Instant};

    let deadline = Instant::now() + Duration::from_secs(2);
    let mut last_audit = String::new();
    while Instant::now() < deadline {
        if let Ok(audit) = std::fs::read_to_string(path) {
            last_audit = audit.clone();
            for line in audit.lines() {
                let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
                    continue;
                };
                if value["audit_action"] == action && value["path"] == route_path {
                    return (line.to_owned(), value);
                }
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!(
        "audit event action={action:?} path={route_path:?} not found in audit log:\n{last_audit}"
    );
}

fn agent_view(agent_id: AgentId, scope: &str, allow_write: bool) -> AgentView {
    let scope_id = scope_id(scope);
    AgentView {
        agent_id,
        label: Some("http-test-agent".to_owned()),
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope_id]),
        writable_scopes: if allow_write {
            BTreeSet::from([scope_id])
        } else {
            BTreeSet::new()
        },
        allowed_modes: BTreeSet::from([RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::from([MemoryType::Decision]),
        max_context_budget_tokens: 4_000,
        default_context_budget_tokens: 1_000,
        max_candidate_limit: 100,
        default_candidate_limit: 20,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: Some(3_600),
        allow_remember: allow_write,
        allow_verify_fact: true,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: Some(ScopeId(scope_id.0)),
    }
}
