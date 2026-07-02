use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, ScopeId, Q16_ZERO};
use cortex_engine::{scope_id, Database};

use crate::ServerOptions;

#[test]
fn denied_ingestion_audit_event_does_not_leak_query_body_or_token() {
    let dir = tempfile::tempdir().unwrap();
    let root_path = dir.path().join("db");
    {
        let db = Database::open(&root_path).unwrap();
        db.save_agent_view(&agent_view(AgentId(7), "finance"))
            .unwrap();
    }

    let audit_path = dir.path().join("audit").join("http.jsonl");
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let local_addr = listener.local_addr().unwrap();
    drop(listener);

    let audit_path_for_server = audit_path.clone();
    std::thread::spawn(move || {
        let options = ServerOptions {
            auth_token: Some("super-secret-token".to_owned()),
            auth_agent_id: Some(7),
            audit_log_enabled: true,
            audit_log_path: Some(audit_path_for_server),
            audit_log_mac_key: Some(test_audit_mac_key()),
            ..Default::default()
        };
        let _ = crate::serve_with_options(&root_path, &local_addr.to_string(), options);
    });

    let request = "POST /v1/ingest/text?tenant=alpha&scope=..%2F..%2Fsecret&source=secret-source HTTP/1.1\r\n\
                   Authorization: Bearer super-secret-token\r\n\
                   content-length: 20\r\n\r\n\
                   super-secret-payload";
    let response = request_http(local_addr, request);
    assert!(
        response.contains("403 Forbidden"),
        "forbidden ingest should fail closed: {response}"
    );
    assert!(
        response.contains("permission_denied"),
        "forbidden ingest should use stable permission code: {response}"
    );
    assert!(
        !response.contains("super-secret-payload"),
        "error response must not echo request body: {response}"
    );

    let audit = std::fs::read_to_string(audit_path).unwrap();
    let line = audit.lines().next().unwrap();
    let value = serde_json::from_str::<serde_json::Value>(line).unwrap();
    assert_eq!(value["audit_action"], "ingest");
    assert_eq!(value["path"], "/v1/ingest/text");
    assert_eq!(value["tenant"], "alpha");
    assert_eq!(value["status"], 403);
    assert_eq!(value["scope_decision"], "denied");
    for leaked in [
        "secret-source",
        "super-secret-payload",
        "super-secret-token",
        "..%2F..%2Fsecret",
        "../../secret",
    ] {
        assert!(
            !line.contains(leaked),
            "audit event leaked sensitive request data {leaked:?}: {line}"
        );
    }
}

fn test_audit_mac_key() -> crate::AuditMacKey {
    crate::AuditMacKey::from_hex(
        "test-audit-key",
        "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
    )
    .unwrap()
}

fn request_http(addr: SocketAddr, request: &str) -> String {
    let mut last_err = None;
    for _ in 0..20 {
        match TcpStream::connect(addr) {
            Ok(mut stream) => {
                if let Err(err) = stream.write_all(request.as_bytes()) {
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

fn agent_view(agent_id: AgentId, scope: &str) -> AgentView {
    let scope_id = scope_id(scope);
    AgentView {
        agent_id,
        label: Some("http-security-redaction-agent".to_owned()),
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope_id]),
        writable_scopes: BTreeSet::from([scope_id]),
        allowed_modes: BTreeSet::from([RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::from([MemoryType::Decision]),
        max_context_budget_tokens: 4_000,
        default_context_budget_tokens: 1_000,
        max_candidate_limit: 100,
        default_candidate_limit: 20,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: Some(3_600),
        allow_remember: true,
        allow_verify_fact: true,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: Some(ScopeId(scope_id.0)),
    }
}
