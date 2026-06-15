use super::{
    parse_actor_queue_capacity, parse_audit_log_fsync_policy, parse_audit_log_path,
    parse_auth_agent_id, parse_auth_policy_store_file_path, parse_auth_tokens,
    parse_auth_tokens_file_path, parse_bool_flag, parse_positive_u64, parse_request_rate_limit,
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
