use super::helpers::*;

#[test]
fn admin_can_grant_and_revoke_agent_scope() {
    let dir = tempfile::tempdir().unwrap();
    {
        let db = Database::open(dir.path()).unwrap();
        db.save_agent_view(&agent_view(AgentId(7), "finance"))
            .unwrap();
    }
    let options = ServerOptions {
        auth_tokens: vec![AuthTokenPolicy::new("admin-secret", AuthRole::Admin)],
        ..Default::default()
    };

    let grant_body = r#"{"agent_id":7,"scope":"project:alpha","access":"read_write"}"#;
    let grant = handle_http_with_options(
        dir.path(),
        &post_with_body("/v1/admin/auth/scope/grant", "admin-secret", grant_body),
        &options,
    );
    assert!(grant.contains("200 OK"), "grant should succeed: {grant}");
    let value = body_json(&grant);
    assert_eq!(value["schema_version"], "cortexdb.auth_scope_mutation.v1");
    assert_eq!(value["action"], "grant_scope");
    assert_eq!(value["agent_id"], 7);
    assert_eq!(value["scope"], "project:alpha");
    assert_eq!(value["access"], "read_write");

    {
        let db = Database::open(dir.path()).unwrap();
        let view = db.load_agent_view(AgentId(7)).unwrap().unwrap();
        let project_alpha = scope_id("project:alpha");
        assert!(view.readable_scopes.contains(&project_alpha));
        assert!(view.writable_scopes.contains(&project_alpha));
    }

    let revoke_body = r#"{"agent_id":7,"scope":"project:alpha","access":"read_write"}"#;
    let revoke = handle_http_with_options(
        dir.path(),
        &post_with_body("/v1/admin/auth/scope/revoke", "admin-secret", revoke_body),
        &options,
    );
    assert!(revoke.contains("200 OK"), "revoke should succeed: {revoke}");
    let value = body_json(&revoke);
    assert_eq!(value["action"], "revoke_scope");

    let db = Database::open(dir.path()).unwrap();
    let view = db.load_agent_view(AgentId(7)).unwrap().unwrap();
    let project_alpha = scope_id("project:alpha");
    assert!(!view.readable_scopes.contains(&project_alpha));
    assert!(!view.writable_scopes.contains(&project_alpha));
}

#[test]
fn policy_store_context_budget_clamps_agent_view_context_pack_budget() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.save_agent_view(&agent_view(AgentId(7), "finance"))
            .unwrap();
        db.put_cell(
            cortex_core::CellId(1),
            b"scope=finance\nstatus=ready\nsource=doc-a\n\nfinance budget memo".to_vec(),
        )
        .unwrap();
    }
    let policy_store = dir.path().join("auth-policy.json");
    std::fs::write(
        &policy_store,
        r#"{
          "schema_version": "cortexdb.auth_policy.v1",
          "principals": [
            {"principal_id":"finance-agent","token":"finance-token","role":"data","agent_id":7,"context_budget_tokens":500}
          ]
        }"#,
    )
    .unwrap();
    let options = ServerOptions {
        auth_policy_store_file: Some(policy_store),
        ..Default::default()
    };

    let response = handle_http_with_options(
        dir.path(),
        &post_with_body(
            "/v1/context?scope=finance",
            "finance-token",
            "RETRIEVE CONTEXT FOR TASK \"budget\" IN BRAIN default;",
        ),
        &options,
    );
    assert!(
        response.contains("200 OK"),
        "context route should succeed: {response}"
    );
    let value = body_json(&response);
    assert_eq!(value["token_budget_tokens"], 500);
    let decision = &value["cells"][0]["access_decision"];
    assert_eq!(decision["cell_id"], 1);
    assert_eq!(decision["decision"], "allowed");
    assert_eq!(decision["policy"], "agent_view_readable_scope");
    assert_eq!(decision["scope"], "finance");
    assert_eq!(decision["agent_id"], 7);
    assert_eq!(decision["principal_id"], "finance-agent");
    assert_eq!(decision["auth_role"], "data");
}

#[test]
fn auth_policy_store_tenants_restrict_database_realms() {
    let dir = tempfile::tempdir().unwrap();
    let policy_store = dir.path().join("auth-policy.json");
    std::fs::write(
        &policy_store,
        r#"{
          "schema_version": "cortexdb.auth_policy.v1",
          "principals": [
            {"principal_id":"alpha-data","token":"alpha-secret","role":"data","tenants":["alpha"]}
          ]
        }"#,
    )
    .unwrap();
    let options = ServerOptions {
        auth_policy_store_file: Some(policy_store),
        ..Default::default()
    };

    let allowed = handle_http_with_options(
        dir.path(),
        "GET /v1/health?tenant=alpha HTTP/1.1\r\nAuthorization: Bearer alpha-secret\r\n\r\n",
        &options,
    );
    assert!(
        allowed.contains("200 OK"),
        "tenant allowlist should allow alpha realm: {allowed}"
    );

    let denied = handle_http_with_options(
        dir.path(),
        "GET /v1/health?tenant=beta HTTP/1.1\r\nAuthorization: Bearer alpha-secret\r\n\r\n",
        &options,
    );
    assert!(
        denied.contains("403 Forbidden"),
        "tenant allowlist should deny beta realm: {denied}"
    );
    assert!(
        denied.contains("token tenant policy is not allowed"),
        "denial should explain tenant policy: {denied}"
    );

    let alpha_write = handle_http_with_options(
        dir.path(),
        concat!(
            "POST /v1/cell?tenant=alpha&cell_id=1 HTTP/1.1\r\n",
            "Authorization: Bearer alpha-secret\r\n\r\n",
            "scope=project:investments\nstatus=ready\nalpha policy payload"
        ),
        &options,
    );
    assert!(
        alpha_write.contains("200 OK"),
        "tenant allowlist should allow alpha data write: {alpha_write}"
    );

    let beta_write = handle_http_with_options(
        dir.path(),
        concat!(
            "POST /v1/cell?tenant=beta&cell_id=1 HTTP/1.1\r\n",
            "Authorization: Bearer alpha-secret\r\n\r\n",
            "scope=project:investments\nstatus=ready\nbeta policy payload"
        ),
        &options,
    );
    assert!(
        beta_write.contains("403 Forbidden"),
        "tenant allowlist should deny beta data write: {beta_write}"
    );
    assert!(
        !dir.path().join("realms").join("beta").exists(),
        "denied tenant data route must not create the beta realm"
    );

    let alpha_read = handle_http_with_options(
        dir.path(),
        "GET /v1/cell?tenant=alpha&cell_id=1 HTTP/1.1\r\nAuthorization: Bearer alpha-secret\r\n\r\n",
        &options,
    );
    assert!(alpha_read.contains("alpha policy payload"));
}
