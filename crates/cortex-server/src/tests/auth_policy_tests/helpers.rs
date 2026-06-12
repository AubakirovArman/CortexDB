#![allow(unused_imports)]

pub(crate) use std::collections::BTreeSet;

pub(crate) use cortex_aql::{
    AgentId, AgentView, BrainId, MemoryType, RetrievalMode, ScopeId, Q16_ZERO,
};
pub(crate) use cortex_engine::{scope_id, Database};
pub(crate) use serde_json::Value;

pub(crate) use crate::auth_policy_cells;
pub(crate) use crate::{handle_http_with_options, AuthRole, AuthTokenPolicy, ServerOptions};

pub(crate) fn admin_and_data_options() -> ServerOptions {
    ServerOptions {
        auth_tokens: vec![
            AuthTokenPolicy::new("data-secret", AuthRole::Data),
            AuthTokenPolicy::new("admin-secret", AuthRole::Admin),
        ],
        ..Default::default()
    }
}

pub(crate) fn body_json(response: &str) -> Value {
    let (_, body) = response.split_once("\r\n\r\n").unwrap();
    serde_json::from_str(body).unwrap()
}

pub(crate) fn post_with_body(path: &str, token: &str, body: &str) -> String {
    format!(
        "POST {path} HTTP/1.1\r\nAuthorization: Bearer {token}\r\ncontent-length: {}\r\n\r\n{body}",
        body.len()
    )
}

pub(crate) fn agent_view(agent_id: AgentId, scope: &str) -> AgentView {
    let scope_id = scope_id(scope);
    AgentView {
        agent_id,
        label: Some("http-token-policy-agent".to_owned()),
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
