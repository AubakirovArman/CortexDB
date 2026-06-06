use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, ScopeId, Q16_ZERO};
use cortex_engine::{scope_id, Database};
use serde_json::Value;

use crate::auth_policy_cells;
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
    let options = ServerOptions {
        dashboard_enabled: true,
        ..admin_and_data_options()
    };

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

#[test]
fn auth_policy_store_allows_active_principal_and_denies_disabled() {
    let dir = tempfile::tempdir().unwrap();
    let policy_store = dir.path().join("auth-policy.json");
    std::fs::write(
        &policy_store,
        r#"{
          "schema_version": "cortexdb.auth_policy.v1",
          "principals": [
            {"principal_id":"data-a","token":"active-data","role":"data"},
            {"principal_id":"data-b","token":"disabled-data","role":"data","disabled":true}
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
        "GET /v1/health HTTP/1.1\r\nAuthorization: Bearer active-data\r\n\r\n",
        &options,
    );
    assert!(
        allowed.contains("200 OK"),
        "active policy-store principal should work: {allowed}"
    );

    let denied = handle_http_with_options(
        dir.path(),
        "GET /v1/health HTTP/1.1\r\nAuthorization: Bearer disabled-data\r\n\r\n",
        &options,
    );
    assert!(
        denied.contains("401 Unauthorized"),
        "disabled policy-store principal should fail closed: {denied}"
    );
}

#[test]
fn auth_policy_store_agent_scope_is_enforced() {
    let dir = tempfile::tempdir().unwrap();
    {
        let db = Database::open(dir.path()).unwrap();
        db.save_agent_view(&agent_view(AgentId(11), "finance"))
            .unwrap();
    }
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
    let options = ServerOptions {
        auth_policy_store_file: Some(policy_store),
        ..Default::default()
    };

    let denied = handle_http_with_options(
        dir.path(),
        "POST /v1/search?scope=secret&q=budget HTTP/1.1\r\nAuthorization: Bearer finance-token\r\n\r\n",
        &options,
    );
    assert!(
        denied.contains("403 Forbidden"),
        "policy-store AgentView must deny unreadable scope: {denied}"
    );

    let allowed = handle_http_with_options(
        dir.path(),
        "POST /v1/search?scope=finance&q=budget HTTP/1.1\r\nAuthorization: Bearer finance-token\r\n\r\n",
        &options,
    );
    assert!(
        allowed.contains("200 OK"),
        "policy-store AgentView should allow readable scope: {allowed}"
    );
}

#[test]
fn auth_policy_store_capabilities_restrict_data_routes() {
    let dir = tempfile::tempdir().unwrap();
    let policy_store = dir.path().join("auth-policy.json");
    std::fs::write(
        &policy_store,
        r#"{
          "schema_version": "cortexdb.auth_policy.v1",
          "principals": [
            {"principal_id":"search-only","token":"search-token","role":"data","capabilities":["search"]}
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
        "POST /v1/search?scope=finance&q=budget HTTP/1.1\r\nAuthorization: Bearer search-token\r\n\r\n",
        &options,
    );
    assert!(
        allowed.contains("200 OK"),
        "search capability should allow search route: {allowed}"
    );

    let denied = handle_http_with_options(
        dir.path(),
        "POST /v1/cell?cell_id=1 HTTP/1.1\r\nAuthorization: Bearer search-token\r\ncontent-length: 0\r\n\r\n",
        &options,
    );
    assert!(
        denied.contains("403 Forbidden"),
        "search-only principal must not write cells: {denied}"
    );
}

#[test]
fn auth_policy_store_invalid_capability_fails_closed() {
    let dir = tempfile::tempdir().unwrap();
    let policy_store = dir.path().join("auth-policy.json");
    std::fs::write(
        &policy_store,
        r#"{
          "schema_version": "cortexdb.auth_policy.v1",
          "principals": [
            {"principal_id":"bad","token":"bad-token","role":"data","capabilities":["unknown"]}
          ]
        }"#,
    )
    .unwrap();
    let options = ServerOptions {
        auth_policy_store_file: Some(policy_store),
        ..Default::default()
    };

    let denied = handle_http_with_options(
        dir.path(),
        "GET /v1/health HTTP/1.1\r\nAuthorization: Bearer bad-token\r\n\r\n",
        &options,
    );
    assert!(
        !denied.contains("200 OK"),
        "invalid capability store must fail closed: {denied}"
    );
}

#[test]
fn auth_policy_store_invalid_tenant_fails_closed() {
    let dir = tempfile::tempdir().unwrap();
    let policy_store = dir.path().join("auth-policy.json");
    std::fs::write(
        &policy_store,
        r#"{
          "schema_version": "cortexdb.auth_policy.v1",
          "principals": [
            {"principal_id":"bad","token":"bad-token","role":"data","tenants":["../escape"]}
          ]
        }"#,
    )
    .unwrap();
    let options = ServerOptions {
        auth_policy_store_file: Some(policy_store),
        ..Default::default()
    };

    let denied = handle_http_with_options(
        dir.path(),
        "GET /v1/health HTTP/1.1\r\nAuthorization: Bearer bad-token\r\n\r\n",
        &options,
    );
    assert!(
        !denied.contains("200 OK"),
        "invalid tenant policy store must fail closed: {denied}"
    );
}

#[test]
fn auth_policy_store_invalid_json_fails_closed() {
    let dir = tempfile::tempdir().unwrap();
    let policy_store = dir.path().join("auth-policy.json");
    std::fs::write(&policy_store, "{not json").unwrap();
    let options = ServerOptions {
        auth_policy_store_file: Some(policy_store),
        ..Default::default()
    };

    let denied = handle_http_with_options(
        dir.path(),
        "GET /v1/health HTTP/1.1\r\nAuthorization: Bearer any-token\r\n\r\n",
        &options,
    );
    assert!(
        !denied.contains("200 OK"),
        "invalid auth policy store must fail closed: {denied}"
    );
}

#[test]
fn auth_policy_store_v0_tokens_migrate_and_authenticate() {
    let dir = tempfile::tempdir().unwrap();
    let policy_store = dir.path().join("auth-policy.json");
    std::fs::write(
        &policy_store,
        r#"{
          "schema_version": "cortexdb.auth_policy.v0",
          "tokens": [
            {"principal_id":"legacy-data","token":"legacy-secret","role":"data"}
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
        "GET /v1/health HTTP/1.1\r\nAuthorization: Bearer legacy-secret\r\n\r\n",
        &options,
    );
    assert!(
        allowed.contains("200 OK"),
        "legacy v0 policy-store token should migrate in memory: {allowed}"
    );
}

#[test]
fn auth_policy_store_unknown_schema_fails_closed() {
    let dir = tempfile::tempdir().unwrap();
    let policy_store = dir.path().join("auth-policy.json");
    std::fs::write(
        &policy_store,
        r#"{
          "schema_version": "cortexdb.auth_policy.v9",
          "principals": [
            {"principal_id":"future-data","token":"future-secret","role":"data"}
          ]
        }"#,
    )
    .unwrap();
    let options = ServerOptions {
        auth_policy_store_file: Some(policy_store),
        ..Default::default()
    };

    let denied = handle_http_with_options(
        dir.path(),
        "GET /v1/health HTTP/1.1\r\nAuthorization: Bearer future-secret\r\n\r\n",
        &options,
    );
    assert!(
        !denied.contains("200 OK"),
        "unknown policy-store schema must fail closed: {denied}"
    );
}

#[test]
fn admin_can_upsert_policy_store_principal() {
    let dir = tempfile::tempdir().unwrap();
    let policy_store = dir.path().join("auth-policy.json");
    std::fs::write(
        &policy_store,
        r#"{"schema_version":"cortexdb.auth_policy.v1","principals":[]}"#,
    )
    .unwrap();
    let options = ServerOptions {
        auth_tokens: vec![AuthTokenPolicy::new("admin-secret", AuthRole::Admin)],
        auth_policy_store_file: Some(policy_store.clone()),
        ..Default::default()
    };

    let response = handle_http_with_options(
        dir.path(),
        "POST /v1/admin/auth/principal HTTP/1.1\r\nAuthorization: Bearer admin-secret\r\ncontent-length: 102\r\n\r\n{\"principal_id\":\"data-a\",\"token\":\"data-secret\",\"role\":\"data\",\"request_quota_per_minute\":600}",
        &options,
    );
    assert!(
        response.contains("200 OK"),
        "admin policy mutation should succeed: {response}"
    );
    let value = body_json(&response);
    assert_eq!(value["schema_version"], "cortexdb.auth_policy_mutation.v1");
    assert_eq!(value["action"], "upsert_principal");
    assert_eq!(value["principal_id"], "data-a");
    assert_eq!(value["active_principals"], 1);
    assert_eq!(value["rollback_available"], true);

    let allowed = handle_http_with_options(
        dir.path(),
        "GET /v1/health HTTP/1.1\r\nAuthorization: Bearer data-secret\r\n\r\n",
        &options,
    );
    assert!(
        allowed.contains("200 OK"),
        "newly upserted data principal should authenticate: {allowed}"
    );
    assert!(
        policy_store.with_extension("json.rollback").is_file(),
        "mutation should leave rollback snapshot"
    );
}

#[test]
fn admin_upsert_syncs_redacted_policy_cells() {
    let dir = tempfile::tempdir().unwrap();
    let policy_store = dir.path().join("auth-policy.json");
    std::fs::write(
        &policy_store,
        r#"{"schema_version":"cortexdb.auth_policy.v1","principals":[]}"#,
    )
    .unwrap();
    let options = ServerOptions {
        auth_tokens: vec![AuthTokenPolicy::new("admin-secret", AuthRole::Admin)],
        auth_policy_store_file: Some(policy_store),
        ..Default::default()
    };

    let body = r#"{"principal_id":"data-a","token":"data-secret","role":"data","agent_id":7,"request_quota_per_minute":600,"body_quota_bytes_per_minute":2048,"queue_quota":2,"context_budget_tokens":500,"capabilities":["search","read"],"tenants":["alpha","default"]}"#;
    let request = post_with_body("/v1/admin/auth/principal", "admin-secret", body);
    let response = handle_http_with_options(dir.path(), &request, &options);
    assert!(
        response.contains("200 OK"),
        "admin policy mutation should sync cells: {response}"
    );

    let db = Database::open(dir.path()).unwrap();
    let records = auth_policy_cells::load_policy_cell_records(&db).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].principal_id, "data-a");
    assert_eq!(records[0].role, "data");
    assert_eq!(records[0].agent_id, Some(7));
    assert_eq!(records[0].request_quota_per_minute, Some(600));
    assert_eq!(records[0].body_quota_bytes_per_minute, Some(2048));
    assert_eq!(records[0].queue_quota, Some(2));
    assert_eq!(records[0].context_budget_tokens, Some(500));
    assert_eq!(records[0].capabilities, vec!["read", "search"]);
    assert_eq!(records[0].tenants, vec!["alpha", "default"]);
    assert!(records[0].token_fingerprint.starts_with("fnv64:"));
    assert!(!serde_json::to_string(&records)
        .unwrap()
        .contains("data-secret"));

    let effective = auth_policy_cells::effective_policy_mapping_from_cells(&db).unwrap();
    assert_eq!(effective.len(), 1);
    assert_eq!(effective[0].principal_id.as_deref(), Some("data-a"));
    assert_eq!(effective[0].role, AuthRole::Data);
    assert_eq!(effective[0].agent_id, Some(7));
    assert_eq!(effective[0].request_quota_per_minute, Some(600));
    assert_eq!(effective[0].body_quota_bytes_per_minute, Some(2048));
    assert_eq!(effective[0].queue_quota, Some(2));
    assert_eq!(effective[0].context_budget_tokens, Some(500));
    assert_eq!(
        effective[0].tenants.as_ref().unwrap(),
        &BTreeSet::from(["alpha".to_owned(), "default".to_owned()])
    );
    assert!(effective[0].token.starts_with("fnv64:"));
}

#[test]
fn admin_can_list_redacted_policy_store_principals() {
    let dir = tempfile::tempdir().unwrap();
    let policy_store = dir.path().join("auth-policy.json");
    std::fs::write(
        &policy_store,
        r#"{
          "schema_version": "cortexdb.auth_policy.v1",
          "principals": [
            {
              "principal_id":"data-a",
              "token":"data-secret",
              "role":"data",
              "agent_id":7,
              "request_quota_per_minute":600,
              "body_quota_bytes_per_minute":2048,
              "queue_quota":2,
              "context_budget_tokens":500,
              "capabilities":["search","read"],
              "tenants":["default","alpha"]
            },
            {"principal_id":"disabled-admin","token":"admin-secret-2","role":"admin","disabled":true}
          ]
        }"#,
    )
    .unwrap();
    let options = ServerOptions {
        auth_tokens: vec![AuthTokenPolicy::new("admin-secret", AuthRole::Admin)],
        auth_policy_store_file: Some(policy_store),
        ..Default::default()
    };

    let response = handle_http_with_options(
        dir.path(),
        "GET /v1/admin/auth/policies HTTP/1.1\r\nAuthorization: Bearer admin-secret\r\n\r\n",
        &options,
    );
    assert!(
        response.contains("200 OK"),
        "admin policy list should succeed: {response}"
    );
    assert!(
        !response.contains("data-secret"),
        "policy list must not disclose raw token: {response}"
    );

    let value = body_json(&response);
    assert_eq!(value["schema_version"], "cortexdb.auth_policy_list.v1");
    assert_eq!(value["principal_count"], 2);
    assert_eq!(value["active_principals"], 1);
    assert_eq!(value["disabled_principals"], 1);
    assert_eq!(
        value["supported_roles"],
        serde_json::json!(["admin", "data"])
    );
    assert_eq!(value["principals"][0]["principal_id"], "data-a");
    assert_eq!(value["principals"][0]["role"], "data");
    assert_eq!(value["principals"][0]["agent_id"], 7);
    assert_eq!(value["principals"][0]["context_budget_tokens"], 500);
    assert_eq!(
        value["principals"][0]["capabilities"],
        serde_json::json!(["read", "search"])
    );
    assert_eq!(
        value["principals"][0]["tenants"],
        serde_json::json!(["alpha", "default"])
    );
    assert_eq!(value["principals"][0]["token_present"], true);
    assert!(value["principals"][0]["token_fingerprint"]
        .as_str()
        .unwrap()
        .starts_with("fnv64:"));
}

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
}

#[test]
fn admin_disable_updates_policy_cell_mapping() {
    let dir = tempfile::tempdir().unwrap();
    let policy_store = dir.path().join("auth-policy.json");
    std::fs::write(
        &policy_store,
        r#"{
          "schema_version": "cortexdb.auth_policy.v1",
          "principals": [
            {"principal_id":"data-a","token":"data-secret","role":"data"}
          ]
        }"#,
    )
    .unwrap();
    let options = ServerOptions {
        auth_tokens: vec![AuthTokenPolicy::new("admin-secret", AuthRole::Admin)],
        auth_policy_store_file: Some(policy_store),
        ..Default::default()
    };

    let disabled = handle_http_with_options(
        dir.path(),
        "DELETE /v1/admin/auth/principal?principal_id=data-a HTTP/1.1\r\nAuthorization: Bearer admin-secret\r\n\r\n",
        &options,
    );
    assert!(
        disabled.contains("200 OK"),
        "disable should sync policy cell: {disabled}"
    );

    let db = Database::open(dir.path()).unwrap();
    let records = auth_policy_cells::load_policy_cell_records(&db).unwrap();
    assert_eq!(records.len(), 1);
    assert!(records[0].disabled);
    assert!(auth_policy_cells::effective_policy_mapping_from_cells(&db)
        .unwrap()
        .is_empty());
}

#[test]
fn data_token_cannot_mutate_policy_store() {
    let dir = tempfile::tempdir().unwrap();
    let policy_store = dir.path().join("auth-policy.json");
    std::fs::write(
        &policy_store,
        r#"{"schema_version":"cortexdb.auth_policy.v1","principals":[]}"#,
    )
    .unwrap();
    let options = ServerOptions {
        auth_tokens: vec![AuthTokenPolicy::new("data-secret", AuthRole::Data)],
        auth_policy_store_file: Some(policy_store),
        ..Default::default()
    };

    let response = handle_http_with_options(
        dir.path(),
        "POST /v1/admin/auth/principal HTTP/1.1\r\nAuthorization: Bearer data-secret\r\ncontent-length: 55\r\n\r\n{\"principal_id\":\"x\",\"token\":\"x\",\"role\":\"data\"}",
        &options,
    );
    assert!(
        response.contains("403 Forbidden"),
        "data role must not mutate policy store: {response}"
    );

    let response = handle_http_with_options(
        dir.path(),
        &post_with_body(
            "/v1/admin/auth/scope/grant",
            "data-secret",
            r#"{"agent_id":7,"scope":"finance","access":"read"}"#,
        ),
        &options,
    );
    assert!(
        response.contains("403 Forbidden"),
        "data role must not mutate agent scopes: {response}"
    );
}

#[test]
fn admin_can_disable_policy_store_principal_and_rollback() {
    let dir = tempfile::tempdir().unwrap();
    let policy_store = dir.path().join("auth-policy.json");
    std::fs::write(
        &policy_store,
        r#"{
          "schema_version": "cortexdb.auth_policy.v1",
          "principals": [
            {"principal_id":"data-a","token":"data-secret","role":"data"}
          ]
        }"#,
    )
    .unwrap();
    let options = ServerOptions {
        auth_tokens: vec![AuthTokenPolicy::new("admin-secret", AuthRole::Admin)],
        auth_policy_store_file: Some(policy_store),
        ..Default::default()
    };

    let disabled = handle_http_with_options(
        dir.path(),
        "DELETE /v1/admin/auth/principal?principal_id=data-a HTTP/1.1\r\nAuthorization: Bearer admin-secret\r\n\r\n",
        &options,
    );
    assert!(
        disabled.contains("200 OK"),
        "disable should succeed: {disabled}"
    );
    let value = body_json(&disabled);
    assert_eq!(value["action"], "disable_principal");
    assert_eq!(value["disabled_principals"], 1);

    let denied = handle_http_with_options(
        dir.path(),
        "GET /v1/health HTTP/1.1\r\nAuthorization: Bearer data-secret\r\n\r\n",
        &options,
    );
    assert!(
        denied.contains("401 Unauthorized"),
        "disabled principal should fail closed: {denied}"
    );

    let rollback = handle_http_with_options(
        dir.path(),
        "POST /v1/admin/auth/policy/rollback HTTP/1.1\r\nAuthorization: Bearer admin-secret\r\n\r\n",
        &options,
    );
    assert!(
        rollback.contains("200 OK"),
        "rollback should restore previous store: {rollback}"
    );
    let restored = handle_http_with_options(
        dir.path(),
        "GET /v1/health HTTP/1.1\r\nAuthorization: Bearer data-secret\r\n\r\n",
        &options,
    );
    assert!(
        restored.contains("200 OK"),
        "rollback should restore data principal: {restored}"
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

fn body_json(response: &str) -> Value {
    let (_, body) = response.split_once("\r\n\r\n").unwrap();
    serde_json::from_str(body).unwrap()
}

fn post_with_body(path: &str, token: &str, body: &str) -> String {
    format!(
        "POST {path} HTTP/1.1\r\nAuthorization: Bearer {token}\r\ncontent-length: {}\r\n\r\n{body}",
        body.len()
    )
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
