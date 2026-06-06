use super::run;
use crate::cli_audit::{AuditRecord, LlmAuditFields};
use crate::cli_audit_chain::{self, AUDIT_CHAIN_ID, AUDIT_CHAIN_ZERO_HASH};

#[test]
fn audit_verify_accepts_llm_chain_and_rejects_llm_metadata_tampering() {
    let path = unique_path("cortexdb-cli-audit-chain-llm.jsonl");
    let record = chained_record(1, AUDIT_CHAIN_ZERO_HASH, Some(llm_fields("deepseek-chat")));
    std::fs::write(
        &path,
        format!("{}\n", serde_json::to_string(&record).unwrap()),
    )
    .unwrap();

    let output = run(vec![
        "cortexdb".to_owned(),
        "audit".to_owned(),
        "verify".to_owned(),
        path.to_string_lossy().into_owned(),
    ])
    .unwrap();
    assert!(output.contains("chain_ok=true chain_violations=0"));

    let tampered = std::fs::read_to_string(&path).unwrap().replace(
        r#""model":"deepseek-chat""#,
        r#""model":"deepseek-reasoner""#,
    );
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

#[test]
fn audit_verify_detects_deleted_or_reordered_records() {
    let path = unique_path("cortexdb-cli-audit-chain-order.jsonl");
    let first = chained_record(1, AUDIT_CHAIN_ZERO_HASH, None);
    let second = chained_record(2, first.event_hash.as_deref().unwrap(), None);
    let third = chained_record(3, second.event_hash.as_deref().unwrap(), None);

    std::fs::write(
        &path,
        format!(
            "{}\n{}\n{}\n",
            serde_json::to_string(&first).unwrap(),
            serde_json::to_string(&second).unwrap(),
            serde_json::to_string(&third).unwrap()
        ),
    )
    .unwrap();
    run(vec![
        "cortexdb".to_owned(),
        "audit".to_owned(),
        "verify".to_owned(),
        path.to_string_lossy().into_owned(),
    ])
    .unwrap();

    std::fs::write(
        &path,
        format!(
            "{}\n{}\n",
            serde_json::to_string(&first).unwrap(),
            serde_json::to_string(&third).unwrap()
        ),
    )
    .unwrap();
    let deleted_error = run(vec![
        "cortexdb".to_owned(),
        "audit".to_owned(),
        "verify".to_owned(),
        path.to_string_lossy().into_owned(),
    ])
    .unwrap_err();
    assert!(deleted_error.contains("chain_violations=1"));

    std::fs::write(
        &path,
        format!(
            "{}\n{}\n{}\n",
            serde_json::to_string(&second).unwrap(),
            serde_json::to_string(&first).unwrap(),
            serde_json::to_string(&third).unwrap()
        ),
    )
    .unwrap();
    let reordered_error = run(vec![
        "cortexdb".to_owned(),
        "audit".to_owned(),
        "verify".to_owned(),
        path.to_string_lossy().into_owned(),
    ])
    .unwrap_err();
    assert!(reordered_error.contains("chain_violations="));
    assert!(!reordered_error.contains("chain_violations=0"));

    let _ = std::fs::remove_file(path);
}

fn chained_record(sequence: u64, prev_hash: &str, llm: Option<LlmAuditFields>) -> AuditRecord {
    let mut record = AuditRecord {
        schema_version: "cortexdb.audit.v1".to_owned(),
        audit_event: if llm.is_some() {
            "llm_inference_decision"
        } else {
            "http_response"
        }
        .to_owned(),
        audit_action: if llm.is_some() { "inference" } else { "health" }.to_owned(),
        chain_id: Some(AUDIT_CHAIN_ID.to_owned()),
        sequence: Some(sequence),
        prev_hash: Some(prev_hash.to_owned()),
        event_hash: None,
        principal_id: Some("principal-a".to_owned()),
        auth_role: Some("data".to_owned()),
        auth_agent_id: Some(7),
        method: "POST".to_owned(),
        path: if llm.is_some() {
            "/v1/inference"
        } else {
            "/v1/health"
        }
        .to_owned(),
        tenant: "default".to_owned(),
        request_id: Some(format!("req-{sequence}")),
        status: 200,
        error_code: String::new(),
        duration_ms: 1,
        unix_time_ms: sequence as u128,
        llm,
    };
    record.event_hash = Some(cli_audit_chain::event_hash_for_record(&record));
    record
}

fn llm_fields(model: &str) -> LlmAuditFields {
    LlmAuditFields {
        outcome: "allowed".to_owned(),
        reason: "policy_allowed".to_owned(),
        provider: "deepseek".to_owned(),
        model: model.to_owned(),
        context_cell_count: 3,
        citation_count: 2,
        request_api_key_present: true,
        prompt_body_logged: false,
        secrets_logged: false,
    }
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
