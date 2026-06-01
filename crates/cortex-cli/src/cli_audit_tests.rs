use super::run;
use crate::cli_audit::{review, AuditRecord, AuditReviewOptions};
use crate::cli_audit_chain::{self, AUDIT_CHAIN_ID};

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
fn audit_review_filters_and_summarizes_records() {
    let path = unique_path("cortexdb-cli-audit-review.jsonl");
    std::fs::write(
        &path,
        r#"{"schema_version":"cortexdb.audit.v1","audit_event":"http_response","audit_action":"read","method":"GET","path":"/v1/cell","tenant":"default","status":200,"error_code":"","duration_ms":3,"unix_time_ms":1}
{"schema_version":"cortexdb.audit.v1","audit_event":"http_response","audit_action":"write","method":"POST","path":"/v1/cell","tenant":"tenant-a","status":403,"error_code":"permission_denied","duration_ms":7,"unix_time_ms":2}
"#,
    )
    .unwrap();

    let output = review(AuditReviewOptions {
        path: path.to_str().unwrap(),
        route: Some("/v1/cell"),
        status: Some(403),
        action: Some("write"),
        tenant: Some("tenant-a"),
        summary_only: false,
        redaction_check: true,
        verify_chain: false,
        json: false,
    })
    .unwrap();
    assert!(output.contains("audit_records=2 matched_records=1 redaction_ok=true"));
    assert!(output.contains("by_action=write:1"));
    assert!(output.contains("by_status=403:1"));
    assert!(output.contains("by_tenant=tenant-a:1"));
    assert!(output.contains("record method=POST path=/v1/cell"));

    let _ = std::fs::remove_file(path);
}

#[test]
fn audit_review_redaction_check_rejects_query_or_body_fields() {
    let path = unique_path("cortexdb-cli-audit-redaction.jsonl");
    std::fs::write(
        &path,
        r#"{"schema_version":"cortexdb.audit.v1","audit_event":"http_response","audit_action":"read","method":"GET","path":"/v1/cell?cell_id=1","tenant":"default","status":200,"error_code":"","duration_ms":3,"unix_time_ms":1}
"#,
    )
    .unwrap();

    let error = review(AuditReviewOptions {
        path: path.to_str().unwrap(),
        route: None,
        status: None,
        action: None,
        tenant: None,
        summary_only: true,
        redaction_check: true,
        verify_chain: false,
        json: false,
    })
    .unwrap_err();
    assert!(error.contains("redaction_violations=1"));

    let _ = std::fs::remove_file(path);
}

#[test]
fn audit_review_verify_chain_accepts_valid_sequence_and_rejects_tampering() {
    let path = unique_path("cortexdb-cli-audit-chain-review.jsonl");
    let first = chained_record(1, "0000000000000000", "GET", "/v1/health", 200);
    let second = chained_record(
        2,
        first.event_hash.as_deref().unwrap(),
        "POST",
        "/v1/cell",
        403,
    );
    std::fs::write(
        &path,
        format!(
            "{}\n{}\n",
            serde_json::to_string(&first).unwrap(),
            serde_json::to_string(&second).unwrap()
        ),
    )
    .unwrap();

    let output = review(AuditReviewOptions {
        path: path.to_str().unwrap(),
        route: None,
        status: None,
        action: None,
        tenant: None,
        summary_only: true,
        redaction_check: true,
        verify_chain: true,
        json: false,
    })
    .unwrap();
    assert!(output.contains("chain_ok=true chain_violations=0"));

    let tampered = std::fs::read_to_string(&path)
        .unwrap()
        .replace(r#""status":403"#, r#""status":200"#);
    std::fs::write(&path, tampered).unwrap();
    let error = review(AuditReviewOptions {
        path: path.to_str().unwrap(),
        route: None,
        status: None,
        action: None,
        tenant: None,
        summary_only: true,
        redaction_check: true,
        verify_chain: true,
        json: false,
    })
    .unwrap_err();
    assert!(error.contains("chain_violations=1"));

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

#[test]
fn audit_command_can_verify_chain() {
    let path = unique_path("cortexdb-cli-audit-chain.jsonl");
    std::fs::write(&path, crate::cli_audit_chain::test_chained_record_jsonl()).unwrap();

    let output = run(vec![
        "cortexdb".to_owned(),
        "audit".to_owned(),
        path.to_string_lossy().into_owned(),
        "--summary".to_owned(),
        "--redaction-check".to_owned(),
        "--verify-chain".to_owned(),
    ])
    .unwrap();

    assert!(output.contains("chain_ok=true chain_violations=0"));

    let _ = std::fs::remove_file(path);
}

#[test]
fn audit_verify_alias_accepts_valid_chain() {
    let path = unique_path("cortexdb-cli-audit-verify-alias.jsonl");
    std::fs::write(&path, crate::cli_audit_chain::test_chained_record_jsonl()).unwrap();

    let output = run(vec![
        "cortexdb".to_owned(),
        "audit".to_owned(),
        "verify".to_owned(),
        path.to_string_lossy().into_owned(),
    ])
    .unwrap();

    assert!(output.contains("chain_ok=true chain_violations=0"));
    assert!(!output.contains("record method="));

    let _ = std::fs::remove_file(path);
}

#[test]
fn audit_verify_alias_rejects_tampered_chain() {
    let path = unique_path("cortexdb-cli-audit-verify-alias-tampered.jsonl");
    let first = chained_record(1, "0000000000000000", "GET", "/v1/health", 200);
    let second = chained_record(
        2,
        first.event_hash.as_deref().unwrap(),
        "POST",
        "/v1/cell",
        403,
    );
    std::fs::write(
        &path,
        format!(
            "{}\n{}\n",
            serde_json::to_string(&first).unwrap(),
            serde_json::to_string(&second).unwrap()
        ),
    )
    .unwrap();
    let tampered = std::fs::read_to_string(&path)
        .unwrap()
        .replace(r#""path":"/v1/cell""#, r#""path":"/v1/compact""#);
    std::fs::write(&path, tampered).unwrap();

    let error = run(vec![
        "cortexdb".to_owned(),
        "audit".to_owned(),
        "verify".to_owned(),
        path.to_string_lossy().into_owned(),
    ])
    .unwrap_err();

    assert!(error.contains("audit chain verification failed"));
    assert!(error.contains("chain_violations=1"));

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

fn chained_record(
    sequence: u64,
    prev_hash: &str,
    method: &str,
    path: &str,
    status: u16,
) -> AuditRecord {
    let mut record = AuditRecord {
        schema_version: "cortexdb.audit.v1".to_owned(),
        audit_event: "http_response".to_owned(),
        audit_action: if status == 200 { "health" } else { "write" }.to_owned(),
        chain_id: Some(AUDIT_CHAIN_ID.to_owned()),
        sequence: Some(sequence),
        prev_hash: Some(prev_hash.to_owned()),
        event_hash: None,
        principal_id: Some("principal-a".to_owned()),
        auth_role: Some("data".to_owned()),
        auth_agent_id: Some(7),
        method: method.to_owned(),
        path: path.to_owned(),
        tenant: "default".to_owned(),
        request_id: Some(format!("req-{sequence}")),
        status,
        error_code: if status == 200 {
            String::new()
        } else {
            "permission_denied".to_owned()
        },
        duration_ms: 1,
        unix_time_ms: sequence as u128,
    };
    record.event_hash = Some(cli_audit_chain::event_hash_for_record(&record));
    record
}
