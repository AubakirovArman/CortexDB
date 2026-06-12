use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, ScopeId, Q16_ZERO};
use cortex_engine::scope_id;

pub(super) fn request_bytes(addr: std::net::SocketAddr, request: &[u8]) -> String {
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

pub(super) fn request(addr: std::net::SocketAddr, request: &str) -> String {
    request_bytes(addr, request.as_bytes())
}

pub(super) fn read_audit_event(
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

pub(super) fn agent_view(agent_id: AgentId, scope: &str, allow_write: bool) -> AgentView {
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
