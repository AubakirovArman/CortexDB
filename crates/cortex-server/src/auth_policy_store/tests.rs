use super::io::decode_store_value;
use super::SCHEMA_VERSION;

#[test]
fn legacy_v0_policy_store_migrates_to_v1_principals() {
    let value = serde_json::json!({
        "schema_version": "cortexdb.auth_policy.v0",
        "tokens": [
            {"principal_id": "data-a", "token": "secret", "role": "data", "agent_id": 7}
        ]
    });

    let store = decode_store_value(value).expect("legacy store should migrate");

    assert_eq!(store.schema_version, SCHEMA_VERSION);
    assert_eq!(store.principals.len(), 1);
    assert_eq!(store.principals[0].principal_id, "data-a");
    assert_eq!(store.principals[0].agent_id, Some(7));
    assert!(!store.principals[0].disabled);
}

#[test]
fn unsupported_policy_store_schema_fails_closed() {
    let value = serde_json::json!({
        "schema_version": "cortexdb.auth_policy.v9",
        "principals": []
    });

    let error = decode_store_value(value).unwrap_err();

    assert!(
        error.contains("schema_version must be cortexdb.auth_policy.v1 or cortexdb.auth_policy.v0")
    );
}
