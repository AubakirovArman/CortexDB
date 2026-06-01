use super::run;

#[test]
fn audit_export_siem_writes_normalized_jsonl() {
    let input_path = unique_path("cortexdb-cli-audit-siem-input.jsonl");
    let output_path = unique_path("cortexdb-cli-audit-siem-output.jsonl");
    std::fs::write(
        &input_path,
        crate::cli_audit_chain::test_chained_record_jsonl(),
    )
    .unwrap();

    let output = run(vec![
        "cortexdb".to_owned(),
        "audit-export-siem".to_owned(),
        input_path.to_string_lossy().into_owned(),
        output_path.to_string_lossy().into_owned(),
        "--redaction-check".to_owned(),
        "--verify-chain".to_owned(),
    ])
    .unwrap();
    assert!(output.contains("siem_exported_records=1"));
    assert!(output.contains("redaction_checked=true"));
    assert!(output.contains("chain_checked=true"));

    let exported = std::fs::read_to_string(&output_path).unwrap();
    let value = serde_json::from_str::<serde_json::Value>(exported.trim()).unwrap();
    assert_eq!(value["schema_version"], "cortexdb.siem.audit.v1");
    assert_eq!(value["event_kind"], "event");
    assert_eq!(value["event_category"], "web");
    assert_eq!(value["service_name"], "cortexdb");
    assert_eq!(value["event_action"], "health");
    assert_eq!(value["event_outcome"], "success");
    assert_eq!(value["principal_id"], "principal-a");
    assert_eq!(value["auth_role"], "data");
    assert_eq!(value["audit_sequence"], 1);
    assert!(!exported.contains("Bearer"));

    let _ = std::fs::remove_file(input_path);
    let _ = std::fs::remove_file(output_path);
}

#[test]
fn audit_export_siem_can_emit_json_summary() {
    let input_path = unique_path("cortexdb-cli-audit-siem-json-input.jsonl");
    let output_path = unique_path("cortexdb-cli-audit-siem-json-output.jsonl");
    std::fs::write(
        &input_path,
        crate::cli_audit_chain::test_chained_record_jsonl(),
    )
    .unwrap();

    let output = run(vec![
        "cortexdb".to_owned(),
        "--json".to_owned(),
        "audit-export-siem".to_owned(),
        input_path.to_string_lossy().into_owned(),
        output_path.to_string_lossy().into_owned(),
        "--redaction-check".to_owned(),
    ])
    .unwrap();
    let value = serde_json::from_str::<serde_json::Value>(&output).unwrap();
    assert_eq!(value["schema_version"], "cortexdb.siem_export.v1");
    assert_eq!(value["input_records"], 1);
    assert_eq!(value["exported_records"], 1);
    assert_eq!(value["redaction_checked"], true);
    assert_eq!(value["chain_checked"], false);

    let _ = std::fs::remove_file(input_path);
    let _ = std::fs::remove_file(output_path);
}

fn unique_path(prefix: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "{prefix}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}
