use super::helpers::*;

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
