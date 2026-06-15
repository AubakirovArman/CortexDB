use super::run;

#[test]
fn auth_review_redacts_policy_store_tokens() {
    let path = unique_path("cortexdb-auth-review-policy.json");
    std::fs::write(
        &path,
        r#"{
          "schema_version": "cortexdb.auth_policy.v1",
          "principals": [
            {"principal_id":"admin-a","token":"root-secret-token","role":"admin"},
            {"principal_id":"agent-a","token":"agent-secret-token","role":"data","agent_id":7,"request_quota_per_minute":600,"body_quota_bytes_per_minute":2048,"queue_quota":2,"context_budget_tokens":500,"capabilities":["search","read"],"tenants":["default","alpha"]},
            {"principal_id":"old-agent","token":"disabled-secret-token","role":"data","disabled":true}
          ]
        }"#,
    )
    .unwrap();

    let output = run(vec![
        "cortexdb".to_owned(),
        "auth-review".to_owned(),
        "--policy-store".to_owned(),
        path.to_string_lossy().into_owned(),
    ])
    .unwrap();
    assert!(output.contains("auth_policy_records=3"));
    assert!(output.contains("active_records=2"));
    assert!(output.contains("disabled_records=1"));
    assert!(output.contains("principal=agent-a role=data"));
    assert!(output.contains("agent_id=7"));
    assert!(output.contains("quota_per_minute=600"));
    assert!(output.contains("body_quota_bytes_per_minute=2048"));
    assert!(output.contains("queue_quota=2"));
    assert!(output.contains("context_budget_tokens=500"));
    assert!(output.contains("capabilities=search,read"));
    assert!(output.contains("tenants=default,alpha"));
    assert!(output.contains("principal=old-agent role=data active=false disabled=true"));
    assert!(!output.contains("root-secret-token"));
    assert!(!output.contains("agent-secret-token"));
    assert!(!output.contains("disabled-secret-token"));

    let _ = std::fs::remove_file(path);
}

#[test]
fn auth_review_rejects_invalid_capability() {
    let path = unique_path("cortexdb-auth-review-invalid-capability.json");
    std::fs::write(
        &path,
        r#"{
          "schema_version": "cortexdb.auth_policy.v1",
          "principals": [
            {"principal_id":"agent-a","token":"agent-secret-token","role":"data","capabilities":["unknown"]}
          ]
        }"#,
    )
    .unwrap();

    let error = run(vec![
        "cortexdb".to_owned(),
        "auth-review".to_owned(),
        "--policy-store".to_owned(),
        path.to_string_lossy().into_owned(),
    ])
    .unwrap_err();
    assert!(error.contains("capability is not recognized"));
    assert!(!error.contains("agent-secret-token"));

    let _ = std::fs::remove_file(path);
}

#[test]
fn auth_review_rejects_invalid_tenant() {
    let path = unique_path("cortexdb-auth-review-invalid-tenant.json");
    std::fs::write(
        &path,
        r#"{
          "schema_version": "cortexdb.auth_policy.v1",
          "principals": [
            {"principal_id":"agent-a","token":"agent-secret-token","role":"data","tenants":["tenant:alpha"]}
          ]
        }"#,
    )
    .unwrap();

    let error = run(vec![
        "cortexdb".to_owned(),
        "auth-review".to_owned(),
        "--policy-store".to_owned(),
        path.to_string_lossy().into_owned(),
    ])
    .unwrap_err();
    assert!(error.contains("auth policy tenant is invalid"));
    assert!(!error.contains("agent-secret-token"));

    let _ = std::fs::remove_file(path);
}

#[test]
fn auth_review_json_covers_token_file_and_env_tokens() {
    let path = unique_path("cortexdb-auth-review.tokens");
    std::fs::write(
        &path,
        "# comment\nadmin:file-secret\ndata:file-agent-secret:9\n",
    )
    .unwrap();
    let env_name = format!("CORTEXDB_AUTH_REVIEW_TEST_{}", unique_suffix());
    std::env::set_var(&env_name, "data:env-secret:11");

    let output = run(vec![
        "cortexdb".to_owned(),
        "--json".to_owned(),
        "auth-review".to_owned(),
        "--tokens-file".to_owned(),
        path.to_string_lossy().into_owned(),
        "--tokens-env".to_owned(),
        env_name.clone(),
    ])
    .unwrap();
    std::env::remove_var(&env_name);
    let value = serde_json::from_str::<serde_json::Value>(&output).unwrap();
    assert_eq!(value["schema_version"], "cortexdb.auth_review.v1");
    assert_eq!(value["total_records"], 3);
    assert_eq!(value["active_records"], 3);
    assert_eq!(value["disabled_records"], 0);
    assert_eq!(value["records"][1]["agent_id"], 9);
    assert_eq!(value["records"][2]["agent_id"], 11);
    assert_eq!(value["records"][0]["token_redacted"], true);
    assert!(!output.contains("file-secret"));
    assert!(!output.contains("file-agent-secret"));
    assert!(!output.contains("env-secret"));

    let _ = std::fs::remove_file(path);
}

#[test]
fn auth_review_rejects_inline_tokens_argument_without_echoing_value() {
    let error = run(vec![
        "cortexdb".to_owned(),
        "auth-review".to_owned(),
        "--tokens".to_owned(),
        "data:argv-secret-token:11".to_owned(),
    ])
    .unwrap_err();
    assert!(error.contains("--tokens is not accepted"));
    assert!(!error.contains("argv-secret-token"));
}

#[test]
fn auth_review_rejects_zero_quota() {
    let path = unique_path("cortexdb-auth-review-zero-quota.json");
    std::fs::write(
        &path,
        r#"{
          "schema_version": "cortexdb.auth_policy.v1",
          "principals": [
            {"principal_id":"agent-a","token":"agent-secret-token","role":"data","request_quota_per_minute":0}
          ]
        }"#,
    )
    .unwrap();

    let error = run(vec![
        "cortexdb".to_owned(),
        "auth-review".to_owned(),
        "--policy-store".to_owned(),
        path.to_string_lossy().into_owned(),
    ])
    .unwrap_err();
    assert!(error.contains("request_quota_per_minute must be greater than zero"));
    assert!(!error.contains("agent-secret-token"));

    let _ = std::fs::remove_file(path);
}

#[test]
fn auth_review_rejects_zero_context_budget() {
    let path = unique_path("cortexdb-auth-review-zero-context-budget.json");
    std::fs::write(
        &path,
        r#"{
          "schema_version": "cortexdb.auth_policy.v1",
          "principals": [
            {"principal_id":"agent-a","token":"agent-secret-token","role":"data","context_budget_tokens":0}
          ]
        }"#,
    )
    .unwrap();

    let error = run(vec![
        "cortexdb".to_owned(),
        "auth-review".to_owned(),
        "--policy-store".to_owned(),
        path.to_string_lossy().into_owned(),
    ])
    .unwrap_err();
    assert!(error.contains("context_budget_tokens must be greater than zero"));
    assert!(!error.contains("agent-secret-token"));

    let _ = std::fs::remove_file(path);
}

fn unique_path(prefix: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("{prefix}-{}", unique_suffix()))
}

fn unique_suffix() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}
