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

    std::thread::sleep(std::time::Duration::from_millis(100));

    {
        use std::io::{Read, Write};
        use std::net::TcpStream;
        let mut stream = TcpStream::connect(local_addr).unwrap();
        let huge_size = 2100 * 1024;
        let mut huge_request = Vec::with_capacity(huge_size + 100);
        huge_request.extend_from_slice(b"POST /put?cell_id=1 HTTP/1.1\r\nContent-Length: ");
        huge_request.extend_from_slice(huge_size.to_string().as_bytes());
        huge_request.extend_from_slice(b"\r\n\r\n");
        huge_request.resize(huge_request.len() + huge_size, b'A');

        let _ = stream.write_all(&huge_request);

        let mut response = [0u8; 1024];
        let read = stream.read(&mut response).unwrap();
        let resp_str = String::from_utf8_lossy(&response[..read]);
        assert!(resp_str.contains("413 Payload Too Large"));
    }

    {
        use std::io::{Read, Write};
        use std::net::TcpStream;

        let mut threads = vec![];
        for _ in 0..5 {
            threads.push(std::thread::spawn(move || {
                let mut stream = TcpStream::connect(local_addr).unwrap();
                stream
                    .write_all(b"GET /v1/health HTTP/1.1\r\n\r\n")
                    .unwrap();
                let mut resp = [0u8; 1024];
                let read = stream.read(&mut resp).unwrap();
                let resp_str = String::from_utf8_lossy(&resp[..read]);
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

    std::thread::sleep(std::time::Duration::from_millis(100));

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
        use std::io::{Read, Write};
        use std::net::TcpStream;
        let mut stream = TcpStream::connect(local_addr).unwrap();
        let req = format!("GET /v1/health?tenant={} HTTP/1.1\r\n\r\n", tenant);
        stream.write_all(req.as_bytes()).unwrap();

        let mut response = [0u8; 1024];
        let read = stream.read(&mut response).unwrap();
        let resp_str = String::from_utf8_lossy(&response[..read]);
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

    std::thread::sleep(std::time::Duration::from_millis(100));

    use std::io::{Read, Write};
    use std::net::TcpStream;
    let mut stream = TcpStream::connect(local_addr).unwrap();
    stream
        .write_all(
            b"OPTIONS /v1/health HTTP/1.1\r\n\
              Origin: https://app.example\r\n\
              Access-Control-Request-Method: GET\r\n\
              Access-Control-Request-Headers: authorization,content-type\r\n\r\n",
        )
        .unwrap();
    let mut response = [0u8; 2048];
    let read = stream.read(&mut response).unwrap();
    let resp_str = String::from_utf8_lossy(&response[..read]).to_ascii_lowercase();
    assert!(resp_str.contains("access-control-allow-origin: https://app.example"));
    assert!(resp_str.contains("access-control-allow-methods"));
    assert!(resp_str.contains("access-control-allow-headers"));
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
    use std::io::{Read, Write};
    use std::net::TcpStream;

    let mut stream = TcpStream::connect(addr).unwrap();
    stream.write_all(request.as_bytes()).unwrap();
    let mut response = [0u8; 4096];
    let read = stream.read(&mut response).unwrap();
    String::from_utf8_lossy(&response[..read]).to_string()
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
