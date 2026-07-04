use super::config_env::{
    parse_actor_queue_capacity, parse_audit_log_fsync_policy, parse_audit_log_mac_key,
    parse_audit_log_path, parse_auth_agent_id, parse_auth_policy_store_file_path,
    parse_auth_tokens, parse_auth_tokens_file_path, parse_bool_flag, parse_cluster_ingress_leader,
    parse_cluster_ingress_max_in_flight, parse_positive_u64, parse_receipt_external_signer,
    parse_receipt_signing_key, parse_receipt_signing_key_file_json, parse_request_rate_limit,
    route_timeout_from_env,
};

#[test]
fn parse_actor_queue_capacity_accepts_positive_integer() {
    assert_eq!(parse_actor_queue_capacity("64").unwrap(), 64);
}

#[test]
fn parse_actor_queue_capacity_rejects_zero_and_invalid_values() {
    assert!(parse_actor_queue_capacity("0").is_err());
    assert!(parse_actor_queue_capacity("abc").is_err());
}

#[test]
fn parse_request_rate_limit_accepts_positive_integer() {
    assert_eq!(parse_request_rate_limit("60").unwrap(), 60);
}

#[test]
fn parse_request_rate_limit_rejects_zero_and_invalid_values() {
    assert!(parse_request_rate_limit("0").is_err());
    assert!(parse_request_rate_limit("abc").is_err());
}

#[test]
fn parse_cluster_ingress_leader_accepts_positive_node_id() {
    assert_eq!(parse_cluster_ingress_leader("2").unwrap().0, 2);
    assert!(parse_cluster_ingress_leader("0").is_err());
    assert!(parse_cluster_ingress_leader("abc").is_err());
}

#[test]
fn parse_cluster_ingress_max_in_flight_accepts_positive_integer() {
    assert_eq!(parse_cluster_ingress_max_in_flight("128").unwrap(), 128);
}

#[test]
fn parse_cluster_ingress_max_in_flight_rejects_zero_and_invalid_values() {
    assert!(parse_cluster_ingress_max_in_flight("0").is_err());
    assert!(parse_cluster_ingress_max_in_flight("abc").is_err());
}

#[test]
fn route_timeout_env_defaults_and_rejects_zero() {
    const VAR: &str = "CORTEXDB_TEST_ROUTE_TIMEOUT_MS";
    std::env::remove_var(VAR);
    assert_eq!(route_timeout_from_env(VAR, 123).unwrap(), 123);
    std::env::set_var(VAR, "77");
    assert_eq!(route_timeout_from_env(VAR, 123).unwrap(), 77);
    std::env::set_var(VAR, "0");
    assert!(route_timeout_from_env(VAR, 123).is_err());
    std::env::remove_var(VAR);
}

#[test]
fn parse_tenant_quota_values_accept_positive_integer() {
    assert_eq!(
        parse_positive_u64("50", "CORTEXDB_TENANT_MAX_CELLS").unwrap(),
        50
    );
    assert!(parse_positive_u64("0", "CORTEXDB_TENANT_QUEUE_QUOTA").is_err());
    assert!(parse_positive_u64("abc", "CORTEXDB_TENANT_MAX_MEMORY_BYTES").is_err());
}

#[test]
fn parse_auth_agent_id_accepts_positive_integer() {
    assert_eq!(parse_auth_agent_id("7").unwrap(), 7);
}

#[test]
fn parse_auth_agent_id_rejects_zero_and_invalid_values() {
    assert!(parse_auth_agent_id("0").is_err());
    assert!(parse_auth_agent_id("abc").is_err());
}

#[test]
fn parse_auth_tokens_accepts_role_token_agent_entries() {
    let tokens = parse_auth_tokens("admin:root,data:worker:7").unwrap();
    assert_eq!(tokens.len(), 2);
    assert_eq!(tokens[0].role, cortex_server::AuthRole::Admin);
    assert_eq!(tokens[0].token, "root");
    assert_eq!(tokens[0].agent_id, None);
    assert_eq!(tokens[1].role, cortex_server::AuthRole::Data);
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
fn parse_auth_tokens_file_path_rejects_empty_value() {
    assert!(parse_auth_tokens_file_path(" ").is_err());
    assert_eq!(
        parse_auth_tokens_file_path("/tmp/cortexdb-auth.tokens")
            .unwrap()
            .to_string_lossy(),
        "/tmp/cortexdb-auth.tokens"
    );
}

#[test]
fn parse_auth_policy_store_file_path_rejects_empty_value() {
    assert!(parse_auth_policy_store_file_path(" ").is_err());
    assert_eq!(
        parse_auth_policy_store_file_path("/tmp/cortexdb-auth-policy.json")
            .unwrap()
            .to_string_lossy(),
        "/tmp/cortexdb-auth-policy.json"
    );
}

#[test]
fn parse_bool_flag_accepts_common_true_values() {
    assert!(parse_bool_flag("1"));
    assert!(parse_bool_flag("true"));
    assert!(parse_bool_flag("YES"));
    assert!(parse_bool_flag("on"));
    assert!(!parse_bool_flag("0"));
    assert!(!parse_bool_flag("false"));
    assert!(!parse_bool_flag("anything_else"));
}

#[test]
fn parse_audit_log_path_rejects_empty_value() {
    assert!(parse_audit_log_path(" ").is_err());
    assert_eq!(
        parse_audit_log_path("/tmp/cortexdb-audit.jsonl")
            .unwrap()
            .to_string_lossy(),
        "/tmp/cortexdb-audit.jsonl"
    );
}

#[test]
fn parse_audit_log_fsync_policy_accepts_supported_values() {
    assert_eq!(
        parse_audit_log_fsync_policy("always").unwrap(),
        cortex_server::AuditLogFsyncPolicy::Always
    );
    assert_eq!(
        parse_audit_log_fsync_policy("flush-only").unwrap(),
        cortex_server::AuditLogFsyncPolicy::FlushOnly
    );
    assert_eq!(
        parse_audit_log_fsync_policy("flush").unwrap(),
        cortex_server::AuditLogFsyncPolicy::FlushOnly
    );
    assert!(parse_audit_log_fsync_policy("never").is_err());
}

#[test]
fn parse_audit_log_mac_key_validates_hex_and_key_id() {
    let key = parse_audit_log_mac_key(
        "audit-key.1",
        "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
    )
    .unwrap();
    assert_eq!(key.key_id(), "audit-key.1");
    assert!(parse_audit_log_mac_key("audit-key.1", "abcd").is_err());
    assert!(parse_audit_log_mac_key(
        "bad key id",
        "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"
    )
    .is_err());
}

#[test]
fn parse_receipt_signing_key_validates_seed_key_id_and_redacts_debug() {
    let key = parse_receipt_signing_key(
        "receipt-key.1",
        "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
    )
    .unwrap();
    assert_eq!(key.key_id(), "receipt-key.1");
    assert_eq!(key.public_key_hex().len(), 64);
    let debug = format!("{key:?}");
    assert!(debug.contains("receipt-key.1"));
    assert!(debug.contains("redacted"));
    assert!(!debug.contains("00010203"));
    assert!(parse_receipt_signing_key("receipt-key.1", "abcd").is_err());
    assert!(parse_receipt_signing_key(
        "bad key id",
        "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"
    )
    .is_err());
}

#[test]
fn parse_receipt_signing_key_file_json_checks_schema_and_public_key() {
    let key = parse_receipt_signing_key(
        "receipt-key.1",
        "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
    )
    .unwrap();
    let raw = format!(
        r#"{{
  "schema_version": "cortexdb.receipt_signing_key.v1",
  "key_id": "receipt-key.1",
  "signing_seed_hex": "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
  "public_key_hex": "{}"
}}"#,
        key.public_key_hex()
    );
    assert_eq!(
        parse_receipt_signing_key_file_json(&raw).unwrap().key_id(),
        "receipt-key.1"
    );
    assert!(parse_receipt_signing_key_file_json(
        r#"{"schema_version":"wrong","key_id":"receipt-key.1","signing_seed_hex":"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f","public_key_hex":"bad"}"#
    )
    .is_err());
    assert!(parse_receipt_signing_key_file_json(
        r#"{"schema_version":"cortexdb.receipt_signing_key.v1","key_id":"receipt-key.1","signing_seed_hex":"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f","public_key_hex":"0000000000000000000000000000000000000000000000000000000000000000"}"#
    )
    .is_err());
}

#[test]
fn parse_receipt_external_signer_validates_public_key_and_ref() {
    let signer = parse_receipt_external_signer(
        &std::path::PathBuf::from("/usr/bin/receipt-signer"),
        "receipt-key.external",
        "03a107bff3ce10be1d70dd18e74bc09967e9359b73eafcbc8ee3d22a69d5edb5",
        Some(" kms://test/receipt-key ".to_owned()),
    )
    .unwrap();

    assert_eq!(signer.key_id(), "receipt-key.external");
    assert_eq!(
        signer.public_key_hex(),
        "03a107bff3ce10be1d70dd18e74bc09967e9359b73eafcbc8ee3d22a69d5edb5"
    );
    assert_eq!(signer.signer_ref(), Some("kms://test/receipt-key"));
    assert_eq!(
        signer.command(),
        std::path::Path::new("/usr/bin/receipt-signer")
    );
    assert!(parse_receipt_external_signer(
        &std::path::PathBuf::from("/usr/bin/receipt-signer"),
        "bad key id",
        "03a107bff3ce10be1d70dd18e74bc09967e9359b73eafcbc8ee3d22a69d5edb5",
        None,
    )
    .is_err());
    assert!(parse_receipt_external_signer(
        &std::path::PathBuf::from("/usr/bin/receipt-signer"),
        "receipt-key.external",
        "bad",
        None,
    )
    .is_err());
}
