use std::fs;
use std::path::Path;

use cortex_crypto::{
    blake3_256_domain, hex_lower, ReceiptPublicKey, ReceiptSignature, ReceiptSigningKey,
};
use serde::{Deserialize, Serialize};

use super::ReceiptTrustFile;

const REANCHOR_SCHEMA: &str = "cortexdb.receipt_audit_reanchor.v1";
const TRUST_HASH_DOMAIN: &str = "cortexdb.receipt_trust.hash.v1";
const REANCHOR_HASH_DOMAIN: &str = "cortexdb.receipt_audit_reanchor.hash.v1";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct ReceiptReanchorFile {
    pub(super) schema_version: String,
    pub(super) previous_key_id: String,
    pub(super) current_key_id: String,
    pub(super) previous_public_key_hex: String,
    pub(super) current_public_key_hex: String,
    pub(super) audit_chain_id: String,
    pub(super) audit_chain_head: String,
    pub(super) audit_sequence: u64,
    pub(super) trust_manifest_hash: String,
    pub(super) reanchor_hash: String,
    pub(super) previous_signature_hex: String,
    pub(super) current_signature_hex: String,
}

#[derive(Clone)]
struct ReceiptReanchorBody {
    schema_version: String,
    previous_key_id: String,
    current_key_id: String,
    previous_public_key_hex: String,
    current_public_key_hex: String,
    audit_chain_id: String,
    audit_chain_head: String,
    audit_sequence: u64,
    trust_manifest_hash: String,
}

pub(super) fn build_reanchor_record(
    previous: &ReceiptSigningKey,
    current: &ReceiptSigningKey,
    trust: &ReceiptTrustFile,
    audit_chain_head: &str,
    audit_sequence: u64,
) -> Result<ReceiptReanchorFile, String> {
    if previous.key_id() == current.key_id() {
        return Err("re-anchor previous and current receipt key ids must differ".to_owned());
    }
    validate_audit_chain_head(audit_chain_head)?;
    let body = ReceiptReanchorBody {
        schema_version: REANCHOR_SCHEMA.to_owned(),
        previous_key_id: previous.key_id().to_owned(),
        current_key_id: current.key_id().to_owned(),
        previous_public_key_hex: previous.public_key().to_hex(),
        current_public_key_hex: current.public_key().to_hex(),
        audit_chain_id: crate::cli_audit_chain::AUDIT_CHAIN_ID.to_owned(),
        audit_chain_head: audit_chain_head.to_owned(),
        audit_sequence,
        trust_manifest_hash: trust_manifest_hash(trust),
    };
    let reanchor_hash = reanchor_body_hash(&body);
    Ok(ReceiptReanchorFile {
        schema_version: body.schema_version,
        previous_key_id: body.previous_key_id,
        current_key_id: body.current_key_id,
        previous_public_key_hex: body.previous_public_key_hex,
        current_public_key_hex: body.current_public_key_hex,
        audit_chain_id: body.audit_chain_id,
        audit_chain_head: body.audit_chain_head,
        audit_sequence: body.audit_sequence,
        trust_manifest_hash: body.trust_manifest_hash,
        reanchor_hash: reanchor_hash.clone(),
        previous_signature_hex: previous.sign(reanchor_hash.as_bytes()).to_hex(),
        current_signature_hex: current.sign(reanchor_hash.as_bytes()).to_hex(),
    })
}

pub(super) fn read_and_verify_reanchor(
    path: &Path,
    trust: Option<&ReceiptTrustFile>,
) -> Result<ReceiptReanchorFile, String> {
    let raw = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let record: ReceiptReanchorFile =
        serde_json::from_str(&raw).map_err(|error| format!("invalid JSON: {error}"))?;
    verify_reanchor_record(&record, trust)?;
    Ok(record)
}

fn verify_reanchor_record(
    record: &ReceiptReanchorFile,
    trust: Option<&ReceiptTrustFile>,
) -> Result<(), String> {
    if record.schema_version != REANCHOR_SCHEMA {
        return Err(format!("schema_version must be {REANCHOR_SCHEMA}"));
    }
    if record.previous_key_id == record.current_key_id {
        return Err("previous_key_id and current_key_id must differ".to_owned());
    }
    if record.audit_chain_id != crate::cli_audit_chain::AUDIT_CHAIN_ID {
        return Err("audit_chain_id does not match CortexDB audit chain".to_owned());
    }
    validate_audit_chain_head(&record.audit_chain_head)?;
    if let Some(trust) = trust {
        let expected = trust_manifest_hash(trust);
        if record.trust_manifest_hash != expected {
            return Err("trust_manifest_hash does not match trust file".to_owned());
        }
    }
    let body = ReceiptReanchorBody::from(record);
    let expected_hash = reanchor_body_hash(&body);
    if record.reanchor_hash != expected_hash {
        return Err("reanchor_hash does not match record body".to_owned());
    }
    verify_signature(
        &record.previous_key_id,
        &record.previous_public_key_hex,
        &expected_hash,
        &record.previous_signature_hex,
    )
    .map_err(|error| format!("previous signature check failed: {error}"))?;
    verify_signature(
        &record.current_key_id,
        &record.current_public_key_hex,
        &expected_hash,
        &record.current_signature_hex,
    )
    .map_err(|error| format!("current signature check failed: {error}"))
}

fn verify_signature(
    key_id: &str,
    public_key_hex: &str,
    reanchor_hash: &str,
    signature_hex: &str,
) -> Result<(), String> {
    let public_key =
        ReceiptPublicKey::from_hex(key_id, public_key_hex).map_err(|error| error.to_string())?;
    let signature = ReceiptSignature::from_hex(signature_hex).map_err(|error| error.to_string())?;
    public_key
        .verify(reanchor_hash.as_bytes(), &signature)
        .map_err(|error| error.to_string())
}

fn validate_audit_chain_head(value: &str) -> Result<(), String> {
    if cortex_crypto::audit_chain::is_hex_hash(value) {
        Ok(())
    } else {
        Err("audit_chain_head must be a 64-hex audit event hash".to_owned())
    }
}

fn trust_manifest_hash(trust: &ReceiptTrustFile) -> String {
    hex_lower(&blake3_256_domain(
        TRUST_HASH_DOMAIN,
        &trust_manifest_bytes(trust),
    ))
}

fn reanchor_body_hash(body: &ReceiptReanchorBody) -> String {
    hex_lower(&blake3_256_domain(
        REANCHOR_HASH_DOMAIN,
        &reanchor_body_bytes(body),
    ))
}

fn trust_manifest_bytes(trust: &ReceiptTrustFile) -> Vec<u8> {
    let mut out = Vec::new();
    push_field(&mut out, "schema_version", &trust.schema_version);
    push_field(&mut out, "current_key_id", &trust.current_key_id);
    push_field(
        &mut out,
        "trusted_public_keys_len",
        &trust.trusted_public_keys.len().to_string(),
    );
    for key in &trust.trusted_public_keys {
        push_field(&mut out, "trusted.schema_version", &key.schema_version);
        push_field(&mut out, "trusted.key_id", &key.key_id);
        push_field(&mut out, "trusted.public_key_hex", &key.public_key_hex);
    }
    out
}

fn reanchor_body_bytes(body: &ReceiptReanchorBody) -> Vec<u8> {
    let mut out = Vec::new();
    push_field(&mut out, "schema_version", &body.schema_version);
    push_field(&mut out, "previous_key_id", &body.previous_key_id);
    push_field(&mut out, "current_key_id", &body.current_key_id);
    push_field(
        &mut out,
        "previous_public_key_hex",
        &body.previous_public_key_hex,
    );
    push_field(
        &mut out,
        "current_public_key_hex",
        &body.current_public_key_hex,
    );
    push_field(&mut out, "audit_chain_id", &body.audit_chain_id);
    push_field(&mut out, "audit_chain_head", &body.audit_chain_head);
    push_field(&mut out, "audit_sequence", &body.audit_sequence.to_string());
    push_field(&mut out, "trust_manifest_hash", &body.trust_manifest_hash);
    out
}

fn push_field(out: &mut Vec<u8>, name: &str, value: &str) {
    out.extend_from_slice(name.as_bytes());
    out.push(0x1f);
    out.extend_from_slice(value.len().to_string().as_bytes());
    out.push(0);
    out.extend_from_slice(value.as_bytes());
    out.push(0x1e);
}

impl From<&ReceiptReanchorFile> for ReceiptReanchorBody {
    fn from(value: &ReceiptReanchorFile) -> Self {
        Self {
            schema_version: value.schema_version.clone(),
            previous_key_id: value.previous_key_id.clone(),
            current_key_id: value.current_key_id.clone(),
            previous_public_key_hex: value.previous_public_key_hex.clone(),
            current_public_key_hex: value.current_public_key_hex.clone(),
            audit_chain_id: value.audit_chain_id.clone(),
            audit_chain_head: value.audit_chain_head.clone(),
            audit_sequence: value.audit_sequence,
            trust_manifest_hash: value.trust_manifest_hash.clone(),
        }
    }
}
