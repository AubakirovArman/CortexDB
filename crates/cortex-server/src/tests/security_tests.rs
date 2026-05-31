use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, ScopeId, Q16_ZERO};
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

    let audit = std::fs::read_to_string(audit_path).unwrap();
    let line = audit.lines().next().unwrap();
    let value = serde_json::from_str::<serde_json::Value>(line).unwrap();
    assert_eq!(value["audit_event"], "http_response");
    assert_eq!(value["audit_action"], "health");
    assert_eq!(value["method"], "GET");
    assert_eq!(value["path"], "/v1/health");
    assert_eq!(value["tenant"], "alpha");
    assert_eq!(value["status"], 200);
    assert!(!line.contains("tenant=alpha"));
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
