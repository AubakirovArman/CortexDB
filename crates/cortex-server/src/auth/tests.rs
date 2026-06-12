use super::*;

#[test]
fn data_role_cannot_access_admin_routes() {
    assert!(!role_can_access(AuthRole::Data, "GET", "/v1/stats"));
    assert!(!role_can_access(AuthRole::Data, "POST", "/v1/flush"));
    assert!(!role_can_access(
        AuthRole::Data,
        "POST",
        "/v1/admin/compact/pause"
    ));
    assert!(!role_can_access(
        AuthRole::Data,
        "POST",
        "/v1/admin/compact/resume"
    ));
}

#[test]
fn data_role_can_access_data_and_health_routes() {
    assert!(role_can_access(AuthRole::Data, "GET", "/v1/health"));
    assert!(role_can_access(AuthRole::Data, "POST", "/v1/search"));
}

#[test]
fn parse_auth_tokens_accepts_role_token_agent_entries() {
    let tokens = parse_auth_tokens("admin:root,data:worker:7").unwrap();
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].role, AuthRole::Admin);
    assert_eq!(tokens[0].token, "root");
    assert_eq!(tokens[0].agent_id, None);
    assert_eq!(tokens[1].role, AuthRole::Data);
    assert_eq!(tokens[1].token, "worker");
    assert_eq!(tokens[1].agent_id, Some(7));
}

#[test]
fn parse_auth_tokens_rejects_invalid_entries() {
    assert!(parse_auth_tokens("").is_err());
    assert!(parse_auth_tokens("root").is_err());
    assert!(parse_auth_tokens("admin:").is_err());
    assert!(parse_auth_tokens("owner:root").is_err());
    assert!(parse_auth_tokens("data:worker:0").is_err());
}

#[test]
fn token_policy_file_allows_comments_and_blank_lines() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("auth.tokens");
    std::fs::write(&path, "# comment\n\nadmin:root\ndata:worker:9\n").unwrap();
    let tokens = load_auth_tokens_file(&path).unwrap();
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].role, AuthRole::Admin);
    assert_eq!(tokens[1].agent_id, Some(9));
}

#[test]
fn auth_policy_store_loads_active_principals_and_skips_disabled() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("auth-policy.json");
    std::fs::write(
        &path,
        r#"{
              "schema_version": "cortexdb.auth_policy.v1",
              "principals": [
                {"principal_id":"admin-a","token":"root","role":"admin"},
                {"principal_id":"data-a","token":"worker","role":"data","agent_id":7},
                {"principal_id":"disabled-a","token":"disabled","role":"data","disabled":true}
              ]
            }"#,
    )
    .unwrap();
    let tokens = load_auth_policy_store(&path).unwrap();
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].principal_id.as_deref(), Some("admin-a"));
    assert_eq!(tokens[0].role, AuthRole::Admin);
    assert_eq!(tokens[1].principal_id.as_deref(), Some("data-a"));
    assert_eq!(tokens[1].agent_id, Some(7));
}

#[test]
fn auth_policy_store_rejects_duplicate_tokens() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("auth-policy.json");
    std::fs::write(
        &path,
        r#"{
              "schema_version": "cortexdb.auth_policy.v1",
              "principals": [
                {"principal_id":"a","token":"same","role":"admin"},
                {"principal_id":"b","token":"same","role":"data"}
              ]
            }"#,
    )
    .unwrap();
    assert!(load_auth_policy_store(&path).is_err());
}

#[test]
fn auth_policy_store_rejects_zero_quota() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("auth-policy.json");
    std::fs::write(
        &path,
        r#"{
              "schema_version": "cortexdb.auth_policy.v1",
              "principals": [
                {"principal_id":"a","token":"worker","role":"data","request_quota_per_minute":0}
              ]
            }"#,
    )
    .unwrap();
    assert!(load_auth_policy_store(&path).is_err());
}

#[test]
fn auth_policy_store_rejects_zero_body_and_queue_quota() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("auth-policy.json");

    std::fs::write(
        &path,
        r#"{
              "schema_version": "cortexdb.auth_policy.v1",
              "principals": [
                {"principal_id":"a","token":"worker","role":"data","body_quota_bytes_per_minute":0}
              ]
            }"#,
    )
    .unwrap();
    assert!(load_auth_policy_store(&path).is_err());

    std::fs::write(
        &path,
        r#"{
              "schema_version": "cortexdb.auth_policy.v1",
              "principals": [
                {"principal_id":"a","token":"worker","role":"data","queue_quota":0}
              ]
            }"#,
    )
    .unwrap();
    assert!(load_auth_policy_store(&path).is_err());

    std::fs::write(
        &path,
        r#"{
              "schema_version": "cortexdb.auth_policy.v1",
              "principals": [
                {"principal_id":"a","token":"worker","role":"data","context_budget_tokens":0}
              ]
            }"#,
    )
    .unwrap();
    assert!(load_auth_policy_store(&path).is_err());
}
