use crate::cli_audit::AuditRecord;

pub(crate) use cortex_crypto::audit_chain::{AUDIT_CHAIN_ID, AUDIT_CHAIN_ZERO_HASH};

const AUDIT_SCHEMA_VERSION_V1: &str = "cortexdb.audit.v1";
const AUDIT_SCHEMA_VERSION_V2: &str = "cortexdb.audit.v2";

pub(crate) fn verify_record(
    record: &AuditRecord,
    expected_sequence: u64,
    previous_hash: &str,
    mac_key: Option<&cortex_crypto::MacKey>,
) -> bool {
    if !matches!(
        record.schema_version.as_str(),
        AUDIT_SCHEMA_VERSION_V1 | AUDIT_SCHEMA_VERSION_V2
    ) {
        return false;
    }
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
    if event_hash != event_hash_for_record(record) {
        return false;
    }
    if record.schema_version == AUDIT_SCHEMA_VERSION_V2
        || record.mac_key_id.is_some()
        || record.event_mac.is_some()
    {
        return verify_record_mac(record, mac_key);
    }
    true
}

pub(crate) fn event_hash_for_record(record: &AuditRecord) -> String {
    event_hash(&event_fields(record))
}

pub(crate) fn event_mac_for_record(record: &AuditRecord, key: &cortex_crypto::MacKey) -> String {
    cortex_crypto::audit_chain::event_mac(key, &event_fields(record))
}

fn verify_record_mac(record: &AuditRecord, mac_key: Option<&cortex_crypto::MacKey>) -> bool {
    if record.schema_version != AUDIT_SCHEMA_VERSION_V2 {
        return false;
    }
    if record
        .mac_key_id
        .as_deref()
        .is_none_or(|key_id| key_id.is_empty())
    {
        return false;
    }
    let Some(event_mac) = record.event_mac.as_deref() else {
        return false;
    };
    if !cortex_crypto::audit_chain::is_hex_hash(event_mac) {
        return false;
    }
    let Some(mac_key) = mac_key else {
        return false;
    };
    event_mac == event_mac_for_record(record, mac_key)
}

fn event_fields(record: &AuditRecord) -> Vec<(&'static str, String)> {
    vec![
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
            "mac_key_id",
            record.mac_key_id.as_deref().unwrap_or_default().to_owned(),
        ),
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
            "accountability_receipt_hash",
            record
                .accountability_receipt_hash
                .as_deref()
                .unwrap_or_default()
                .to_owned(),
        ),
        (
            "llm",
            record.llm.as_ref().map(llm_hash_value).unwrap_or_default(),
        ),
    ]
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
    cortex_crypto::audit_chain::event_hash(fields)
}

pub(crate) fn mac_key_from_hex(raw_hex: &str) -> Result<cortex_crypto::MacKey, String> {
    let raw_hex = raw_hex.trim();
    if raw_hex.len() != 64 {
        return Err("audit MAC key must be 64 lowercase or uppercase hex characters".to_owned());
    }
    let mut bytes = [0_u8; 32];
    for (index, chunk) in raw_hex.as_bytes().chunks_exact(2).enumerate() {
        let high =
            hex_nibble(chunk[0]).ok_or_else(|| "audit MAC key contains non-hex data".to_owned())?;
        let low =
            hex_nibble(chunk[1]).ok_or_else(|| "audit MAC key contains non-hex data".to_owned())?;
        bytes[index] = (high << 4) | low;
    }
    cortex_crypto::MacKey::from_slice("audit MAC key", &bytes).map_err(|error| error.to_string())
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
pub(crate) fn test_chained_record_jsonl() -> String {
    let mut record = AuditRecord {
        schema_version: AUDIT_SCHEMA_VERSION_V1.to_owned(),
        audit_event: "http_response".to_owned(),
        audit_action: "health".to_owned(),
        chain_id: Some(AUDIT_CHAIN_ID.to_owned()),
        sequence: Some(1),
        prev_hash: Some(AUDIT_CHAIN_ZERO_HASH.to_owned()),
        event_hash: None,
        mac_key_id: None,
        event_mac: None,
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
        accountability_receipt_hash: None,
        llm: None,
    };
    record.event_hash = Some(event_hash_for_record(&record));
    format!("{}\n", serde_json::to_string(&record).unwrap())
}
