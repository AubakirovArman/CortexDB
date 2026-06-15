use crate::cli_audit::AuditRecord;

pub(crate) const AUDIT_CHAIN_ID: &str = "cortexdb.audit.chain.v1";
pub(crate) const AUDIT_CHAIN_ZERO_HASH: &str = "0000000000000000";

const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

pub(crate) fn verify_record(
    record: &AuditRecord,
    expected_sequence: u64,
    previous_hash: &str,
) -> bool {
    if record.chain_id.as_deref() != Some(AUDIT_CHAIN_ID) {
        return false;
    }
    if record.sequence != Some(expected_sequence) {
        return false;
    }
    if record.prev_hash.as_deref() != Some(previous_hash) {
        return false;
    }
    let Some(event_hash) = record.event_hash.as_deref() else {
        return false;
    };
    event_hash == event_hash_for_record(record)
}

pub(crate) fn event_hash_for_record(record: &AuditRecord) -> String {
    event_hash(&[
        (
            "chain_id",
            record.chain_id.as_deref().unwrap_or_default().to_owned(),
        ),
        ("sequence", record.sequence.unwrap_or_default().to_string()),
        (
            "prev_hash",
            record.prev_hash.as_deref().unwrap_or_default().to_owned(),
        ),
        ("schema_version", record.schema_version.clone()),
        ("audit_event", record.audit_event.clone()),
        ("audit_action", record.audit_action.clone()),
        (
            "principal_id",
            record
                .principal_id
                .as_deref()
                .unwrap_or_default()
                .to_owned(),
        ),
        (
            "auth_role",
            record.auth_role.as_deref().unwrap_or_default().to_owned(),
        ),
        (
            "auth_agent_id",
            record
                .auth_agent_id
                .map(|value| value.to_string())
                .unwrap_or_default(),
        ),
        (
            "scope_decision",
            record
                .scope_decision
                .as_deref()
                .unwrap_or_default()
                .to_owned(),
        ),
        ("method", record.method.clone()),
        ("path", record.path.clone()),
        ("tenant", record.tenant.clone()),
        (
            "request_id",
            record.request_id.as_deref().unwrap_or_default().to_owned(),
        ),
        ("status", record.status.to_string()),
        ("error_code", record.error_code.clone()),
        ("duration_ms", record.duration_ms.to_string()),
        ("unix_time_ms", record.unix_time_ms.to_string()),
        (
            "llm",
            record.llm.as_ref().map(llm_hash_value).unwrap_or_default(),
        ),
    ])
}

fn llm_hash_value(fields: &crate::cli_audit::LlmAuditFields) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}",
        fields.outcome,
        fields.reason,
        fields.provider,
        fields.model,
        fields.context_cell_count,
        fields.citation_count,
        fields.request_api_key_present,
        fields.prompt_body_logged,
        fields.secrets_logged,
    )
}

fn event_hash(fields: &[(&str, String)]) -> String {
    let mut hash = FNV_OFFSET;
    for (key, value) in fields {
        feed_hash(&mut hash, key, value);
    }
    format!("{hash:016x}")
}

fn feed_hash(hash: &mut u64, key: &str, value: &str) {
    for byte in key
        .as_bytes()
        .iter()
        .chain([0x1f].iter())
        .chain(value.as_bytes())
        .chain([0x1e].iter())
    {
        *hash ^= u64::from(*byte);
        *hash = hash.wrapping_mul(FNV_PRIME);
    }
}

#[cfg(test)]
pub(crate) fn test_chained_record_jsonl() -> String {
    let mut record = AuditRecord {
        schema_version: "cortexdb.audit.v1".to_owned(),
        audit_event: "http_response".to_owned(),
        audit_action: "health".to_owned(),
        chain_id: Some(AUDIT_CHAIN_ID.to_owned()),
        sequence: Some(1),
        prev_hash: Some(AUDIT_CHAIN_ZERO_HASH.to_owned()),
        event_hash: None,
        principal_id: Some("principal-a".to_owned()),
        auth_role: Some("data".to_owned()),
        auth_agent_id: Some(7),
        scope_decision: Some("not_applicable".to_owned()),
        method: "GET".to_owned(),
        path: "/v1/health".to_owned(),
        tenant: "default".to_owned(),
        request_id: Some("req-1".to_owned()),
        status: 200,
        error_code: String::new(),
        duration_ms: 1,
        unix_time_ms: 1,
        llm: None,
    };
    record.event_hash = Some(event_hash_for_record(&record));
    format!("{}\n", serde_json::to_string(&record).unwrap())
}
