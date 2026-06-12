use super::helpers::*;

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
