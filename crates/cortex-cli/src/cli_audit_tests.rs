use super::run;

#[test]
fn audit_command_filters_and_checks_redaction() {
    let path = unique_path("cortexdb-cli-audit.jsonl");
    std::fs::write(
        &path,
        r#"{"schema_version":"cortexdb.audit.v1","audit_event":"http_response","audit_action":"read","method":"GET","path":"/v1/cell","tenant":"default","status":200,"error_code":"","duration_ms":3,"unix_time_ms":1}
{"schema_version":"cortexdb.audit.v1","audit_event":"http_response","audit_action":"write","method":"POST","path":"/v1/cell","tenant":"tenant-a","status":403,"error_code":"permission_denied","duration_ms":7,"unix_time_ms":2}
"#,
    )
    .unwrap();

    let output = run(vec![
        "cortexdb".to_owned(),
        "audit".to_owned(),
        path.to_string_lossy().into_owned(),
        "--route".to_owned(),
        "/v1/cell".to_owned(),
        "--status".to_owned(),
        "403".to_owned(),
        "--action".to_owned(),
        "write".to_owned(),
        "--tenant-filter".to_owned(),
        "tenant-a".to_owned(),
        "--redaction-check".to_owned(),
    ])
    .unwrap();

    assert!(output.contains("audit_records=2 matched_records=1 redaction_ok=true"));
    assert!(output.contains("by_action=write:1"));
    assert!(output.contains("record method=POST path=/v1/cell"));

    let _ = std::fs::remove_file(path);
}

#[test]
fn audit_command_can_emit_json_summary() {
    let path = unique_path("cortexdb-cli-audit-json.jsonl");
    std::fs::write(
        &path,
        r#"{"schema_version":"cortexdb.audit.v1","audit_event":"http_response","audit_action":"health","method":"GET","path":"/v1/health","tenant":"default","status":200,"error_code":"","duration_ms":1,"unix_time_ms":1}
"#,
    )
    .unwrap();

    let output = run(vec![
        "cortexdb".to_owned(),
        "--json".to_owned(),
        "audit".to_owned(),
        path.to_string_lossy().into_owned(),
        "--summary".to_owned(),
        "--redaction-check".to_owned(),
    ])
    .unwrap();
    let value = serde_json::from_str::<serde_json::Value>(&output).unwrap();
    assert_eq!(value["total_records"], 1);
    assert_eq!(value["matched_records"], 1);
    assert_eq!(value["redaction_ok"], true);
    assert_eq!(value["records"].as_array().unwrap().len(), 0);

    let _ = std::fs::remove_file(path);
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
