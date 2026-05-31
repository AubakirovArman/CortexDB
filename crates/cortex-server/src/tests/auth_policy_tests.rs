use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, ScopeId, Q16_ZERO};
use cortex_engine::{scope_id, Database};

use crate::{handle_http_with_options, AuthRole, AuthTokenPolicy, ServerOptions};

#[test]
fn data_token_cannot_access_admin_routes() {
    let dir = tempfile::tempdir().unwrap();
    let options = admin_and_data_options();

    let denied = handle_http_with_options(
        dir.path(),
        "GET /v1/stats HTTP/1.1\r\nAuthorization: Bearer data-secret\r\n\r\n",
        &options,
    );
    assert!(denied.contains("403 Forbidden"), "data token got: {denied}");
    assert!(
        denied.contains("forbidden"),
        "data token denial should use forbidden code: {denied}"
    );

    let allowed = handle_http_with_options(
        dir.path(),
        "GET /v1/stats HTTP/1.1\r\nAuthorization: Bearer admin-secret\r\n\r\n",
        &options,
    );
    assert!(
        allowed.contains("200 OK"),
        "admin token should access stats: {allowed}"
    );
}

#[test]
fn data_token_cannot_access_dashboard() {
    let dir = tempfile::tempdir().unwrap();
    let options = admin_and_data_options();

    let denied = handle_http_with_options(
        dir.path(),
        "GET /dashboard HTTP/1.1\r\nAuthorization: Bearer data-secret\r\n\r\n",
        &options,
    );
    assert!(
        denied.contains("403 Forbidden"),
        "dashboard should require admin role: {denied}"
    );

    let allowed = handle_http_with_options(
        dir.path(),
        "GET /dashboard HTTP/1.1\r\nAuthorization: Bearer admin-secret\r\n\r\n",
        &options,
    );
    assert!(
        allowed.contains("200 OK"),
        "admin token should access dashboard: {allowed}"
    );
}

#[test]
fn data_token_can_access_data_routes_and_health() {
    let dir = tempfile::tempdir().unwrap();
    let options = ServerOptions {
        auth_tokens: vec![AuthTokenPolicy::new("data-secret", AuthRole::Data)],
        ..Default::default()
    };

    let health = handle_http_with_options(
        dir.path(),
        "GET /v1/health HTTP/1.1\r\nAuthorization: Bearer data-secret\r\n\r\n",
        &options,
    );
    assert!(
        health.contains("200 OK"),
        "health should be public: {health}"
    );

    let search = handle_http_with_options(
        dir.path(),
        "POST /v1/search?scope=finance&q=budget HTTP/1.1\r\nAuthorization: Bearer data-secret\r\n\r\n",
        &options,
    );
    assert!(
        search.contains("200 OK"),
        "data token should access data route: {search}"
    );
}

#[test]
fn token_policy_agent_id_applies_agent_view_scope() {
    let dir = tempfile::tempdir().unwrap();
    {
        let db = Database::open(dir.path()).unwrap();
        db.save_agent_view(&agent_view(AgentId(7), "finance"))
            .unwrap();
    }
    let options = ServerOptions {
        auth_tokens: vec![AuthTokenPolicy::new("scoped-data", AuthRole::Data).with_agent_id(7)],
        ..Default::default()
    };

    let denied = handle_http_with_options(
        dir.path(),
        "POST /v1/search?scope=secret&q=budget HTTP/1.1\r\nAuthorization: Bearer scoped-data\r\n\r\n",
        &options,
    );
    assert!(
        denied.contains("403 Forbidden"),
        "agent-scoped token should deny unreadable scope: {denied}"
    );

    let allowed = handle_http_with_options(
        dir.path(),
        "POST /v1/search?scope=finance&q=budget HTTP/1.1\r\nAuthorization: Bearer scoped-data\r\n\r\n",
        &options,
    );
    assert!(
        allowed.contains("200 OK"),
        "agent-scoped token should allow readable scope: {allowed}"
    );
}

#[test]
fn encoded_scope_in_query_is_decoded_for_agent_scoped_routes() {
    let dir = tempfile::tempdir().unwrap();
    {
        let db = Database::open(dir.path()).unwrap();
        db.save_agent_view(&agent_view(AgentId(9), "project:investments"))
            .unwrap();
    }

    let options = ServerOptions {
        auth_tokens: vec![AuthTokenPolicy::new("scoped-data", AuthRole::Data).with_agent_id(9)],
        ..Default::default()
    };

    let search = handle_http_with_options(
        dir.path(),
        "POST /v1/search?scope=project%3Ainvestments&q=budget HTTP/1.1\r\nAuthorization: Bearer scoped-data\r\n\r\n",
        &options,
    );
    assert!(
        search.contains("200 OK"),
        "encoded scope must be decoded for search: {search}"
    );
    assert!(
        search.contains(r#""results":[]"#),
        "empty-search response should be valid JSON array: {search}"
    );

    let ingest = handle_http_with_options(
        dir.path(),
        "POST /v1/ingest/text?scope=project%3Ainvestments&source=http%20post HTTP/1.1\r\nAuthorization: Bearer scoped-data\r\ncontent-length: 5\r\n\r\nbudget",
        &options,
    );
    assert!(
        ingest.contains("200 OK"),
        "encoded scope must be decoded for ingest routes: {ingest}"
    );
    assert!(
        ingest.contains(r#""job_id":"#),
        "ingest response should include job id: {ingest}"
    );
}

#[test]
fn token_policy_file_rotates_without_new_options() {
    let dir = tempfile::tempdir().unwrap();
    let token_file = dir.path().join("auth.tokens");
    std::fs::write(&token_file, "data:first\n").unwrap();
    let options = ServerOptions {
        auth_tokens_file: Some(token_file.clone()),
        ..Default::default()
    };

    let first_allowed = handle_http_with_options(
        dir.path(),
        "GET /v1/health HTTP/1.1\r\nAuthorization: Bearer first\r\n\r\n",
        &options,
    );
    assert!(
        first_allowed.contains("200 OK"),
        "initial file token should work: {first_allowed}"
    );

    std::fs::write(&token_file, "data:second\n").unwrap();

    let old_denied = handle_http_with_options(
        dir.path(),
        "GET /v1/health HTTP/1.1\r\nAuthorization: Bearer first\r\n\r\n",
        &options,
    );
    assert!(
        old_denied.contains("401 Unauthorized"),
        "rotated-out token should fail: {old_denied}"
    );

    let second_allowed = handle_http_with_options(
        dir.path(),
        "GET /v1/health HTTP/1.1\r\nAuthorization: Bearer second\r\n\r\n",
        &options,
    );
    assert!(
        second_allowed.contains("200 OK"),
        "rotated-in token should work: {second_allowed}"
    );
}

#[test]
fn token_policy_file_failure_fails_closed() {
    let dir = tempfile::tempdir().unwrap();
    let options = ServerOptions {
        auth_tokens_file: Some(dir.path().join("missing.tokens")),
        ..Default::default()
    };

    let denied = handle_http_with_options(
        dir.path(),
        "GET /v1/health HTTP/1.1\r\nAuthorization: Bearer anything\r\n\r\n",
        &options,
    );
    assert!(
        !denied.contains("200 OK"),
        "missing token file must not allow access: {denied}"
    );
}

fn admin_and_data_options() -> ServerOptions {
    ServerOptions {
        auth_tokens: vec![
            AuthTokenPolicy::new("data-secret", AuthRole::Data),
            AuthTokenPolicy::new("admin-secret", AuthRole::Admin),
        ],
        ..Default::default()
    }
}

fn agent_view(agent_id: AgentId, scope: &str) -> AgentView {
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
