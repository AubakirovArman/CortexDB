use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, ScopeId, Q16_ZERO};
use cortex_engine::{scope_id, Database};

use crate::{handle_http_with_options, AuthRole, AuthTokenPolicy, ServerOptions};

#[test]
fn tenant_route_matrix_isolates_public_surfaces() {
    let dir = tempfile::tempdir().unwrap();
    seed_tenant(
        dir.path(),
        "alpha",
        1,
        "project:matrix",
        "alpha-route-matrix unique evidence",
    );
    seed_tenant(
        dir.path(),
        "beta",
        1,
        "project:matrix",
        "beta-route-matrix unique evidence",
    );

    assert_tenant_surface(
        dir.path(),
        "alpha",
        "alpha-route-matrix",
        "beta-route-matrix",
    );
    assert_tenant_surface(
        dir.path(),
        "beta",
        "beta-route-matrix",
        "alpha-route-matrix",
    );

    let alpha_stats = handle_http_with_options(
        dir.path(),
        "GET /v1/stats?tenant=alpha HTTP/1.1\r\n\r\n",
        &ServerOptions::default(),
    );
    let beta_stats = handle_http_with_options(
        dir.path(),
        "GET /v1/stats?tenant=beta HTTP/1.1\r\n\r\n",
        &ServerOptions::default(),
    );
    assert!(alpha_stats.contains(r#""current_seq":1"#));
    assert!(beta_stats.contains(r#""current_seq":1"#));

    let alpha_validate = handle_http_with_options(
        dir.path(),
        "GET /v1/validate?tenant=alpha HTTP/1.1\r\n\r\n",
        &ServerOptions::default(),
    );
    let beta_validate = handle_http_with_options(
        dir.path(),
        "GET /v1/validate?tenant=beta HTTP/1.1\r\n\r\n",
        &ServerOptions::default(),
    );
    assert!(alpha_validate.contains(r#""ok":true"#));
    assert!(beta_validate.contains(r#""ok":true"#));

    let alpha_metrics = handle_http_with_options(
        dir.path(),
        "GET /v1/metrics?tenant=alpha HTTP/1.1\r\n\r\n",
        &ServerOptions::default(),
    );
    let beta_metrics = handle_http_with_options(
        dir.path(),
        "GET /v1/metrics?tenant=beta HTTP/1.1\r\n\r\n",
        &ServerOptions::default(),
    );
    assert!(alpha_metrics.contains("200 OK"));
    assert!(beta_metrics.contains("200 OK"));
    assert!(!alpha_metrics.contains("beta-route-matrix"));
    assert!(!beta_metrics.contains("alpha-route-matrix"));
}

#[test]
fn tenant_agent_views_are_loaded_from_the_requested_realm() {
    let dir = tempfile::tempdir().unwrap();
    seed_tenant(
        dir.path(),
        "alpha",
        1,
        "project:matrix",
        "alpha-finance-agent evidence",
    );
    seed_tenant(
        dir.path(),
        "beta",
        1,
        "project:other",
        "beta-hr-agent evidence",
    );
    save_tenant_agent_view(dir.path(), "alpha", AgentId(7), "project:matrix");
    save_tenant_agent_view(dir.path(), "beta", AgentId(7), "project:other");

    let options = ServerOptions {
        auth_tokens: vec![AuthTokenPolicy::new("scoped-data", AuthRole::Data).with_agent_id(7)],
        ..Default::default()
    };

    let alpha = handle_http_with_options(
        dir.path(),
        "POST /v1/search?tenant=alpha&scope=project:matrix&q=agent HTTP/1.1\r\nAuthorization: Bearer scoped-data\r\n\r\n",
        &options,
    );
    assert!(alpha.contains("200 OK"), "alpha should be allowed: {alpha}");
    assert!(alpha.contains("alpha-finance-agent"));
    assert!(!alpha.contains("beta-hr-agent"));

    let beta_denied = handle_http_with_options(
        dir.path(),
        "POST /v1/search?tenant=beta&scope=project:matrix&q=agent HTTP/1.1\r\nAuthorization: Bearer scoped-data\r\n\r\n",
        &options,
    );
    assert!(
        beta_denied.contains("403 Forbidden"),
        "beta must use its own AgentView, not alpha permissions: {beta_denied}"
    );
    assert!(!beta_denied.contains("alpha-finance-agent"));
    assert!(!beta_denied.contains("beta-hr-agent"));

    let beta_allowed = handle_http_with_options(
        dir.path(),
        "POST /v1/search?tenant=beta&scope=project:other&q=agent HTTP/1.1\r\nAuthorization: Bearer scoped-data\r\n\r\n",
        &options,
    );
    assert!(
        beta_allowed.contains("200 OK"),
        "beta-local AgentView scope should be allowed: {beta_allowed}"
    );
    assert!(beta_allowed.contains("beta-hr-agent"));
    assert!(!beta_allowed.contains("alpha-finance-agent"));
}

fn seed_tenant(root: &std::path::Path, tenant: &str, cell_id: u64, scope: &str, body: &str) {
    let request = format!(
        "POST /v1/cell?tenant={tenant}&cell_id={cell_id} HTTP/1.1\r\n\r\nscope={scope}\nstatus=ready\n{body}"
    );
    let response = handle_http_with_options(root, &request, &ServerOptions::default());
    assert!(
        response.contains(r#""seq":1"#),
        "seed failed for tenant={tenant}: {response}"
    );
}

fn save_tenant_agent_view(root: &std::path::Path, tenant: &str, agent_id: AgentId, scope: &str) {
    let tenant_root = root.join("realms").join(tenant);
    std::fs::create_dir_all(&tenant_root).unwrap();
    let db = Database::open(&tenant_root).unwrap();
    db.save_agent_view(&test_agent_view(agent_id, scope))
        .unwrap();
}

fn test_agent_view(agent_id: AgentId, scope: &str) -> AgentView {
    let scope_id = scope_id(scope);
    AgentView {
        agent_id,
        label: Some("tenant-agent".to_owned()),
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

fn assert_tenant_surface(root: &std::path::Path, tenant: &str, expected: &str, forbidden: &str) {
    let get = format!("GET /v1/cell?tenant={tenant}&cell_id=1 HTTP/1.1\r\n\r\n");
    assert_contains_only_tenant_payload(root, &get, expected, forbidden);

    let search = format!(
        "POST /v1/search?tenant={tenant}&scope=project:matrix&q=route-matrix HTTP/1.1\r\n\r\n"
    );
    assert_contains_only_tenant_payload(root, &search, expected, forbidden);

    let context = format!(
        "POST /v1/context?tenant={tenant}&scope=project:matrix HTTP/1.1\r\n\r\n{}",
        "RETRIEVE CONTEXT FOR TASK \"route matrix\" IN BRAIN default WHERE space = project:matrix AND status = \"ready\" LIMIT 10 CANDIDATES;"
    );
    assert_contains_only_tenant_payload(root, &context, expected, forbidden);

    let aql = format!(
        "POST /v1/aql?tenant={tenant}&scope=project:matrix HTTP/1.1\r\n\r\n{}",
        "RETRIEVE CONTEXT FOR TASK \"route matrix\" IN BRAIN default WHERE space = project:matrix AND status = \"ready\" LIMIT 10 CANDIDATES;"
    );
    assert_contains_only_tenant_payload(root, &aql, expected, forbidden);

    let verify = format!(
        "POST /v1/verify?tenant={tenant}&scope=project:matrix HTTP/1.1\r\n\r\nVERIFY FACT \"{expected}\" IN BRAIN default;"
    );
    assert_contains_only_tenant_payload(root, &verify, expected, forbidden);
}

fn assert_contains_only_tenant_payload(
    root: &std::path::Path,
    request: &str,
    expected: &str,
    forbidden: &str,
) {
    let response = handle_http_with_options(root, request, &ServerOptions::default());
    assert!(
        response.contains(expected),
        "response should contain tenant payload {expected:?}: {response}"
    );
    assert!(
        !response.contains(forbidden),
        "response leaked other tenant payload {forbidden:?}: {response}"
    );
}
