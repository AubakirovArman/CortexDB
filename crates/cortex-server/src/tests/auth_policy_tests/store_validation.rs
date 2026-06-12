use super::helpers::*;

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
