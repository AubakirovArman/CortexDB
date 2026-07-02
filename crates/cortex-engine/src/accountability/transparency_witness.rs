use std::fmt;
use std::path::Path;

use cortex_crypto::{
    ed25519_public_key, ed25519_sign, ed25519_verify, hex_lower, KeyId, SigningSeed,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::receipt::hash_value;
use super::transparency::{read_transparency_log_records, TransparencyLogRecord};
use crate::error::{EngineError, EngineResult};

pub const TRANSPARENCY_WITNESS_RECORD_SCHEMA: &str = "cortexdb.transparency.witness.record.v1";
pub const TRANSPARENCY_WITNESS_RECORD_HASH_DOMAIN: &str =
    "cortexdb.transparency.witness.record_hash.v1";
pub const TRANSPARENCY_WITNESS_SIGNING_DOMAIN: &str = "cortexdb.transparency.witness.sign.v1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransparencyWitnessRecord {
    pub schema_version: String,
    pub witness_id: String,
    pub witnessed_unix_seconds: u64,
    pub witnessed_record_count: u64,
    pub first_sequence: u64,
    pub last_sequence: u64,
    pub first_record_hash: String,
    pub log_head_hash: String,
    pub first_db_instance_id: String,
    pub last_db_instance_id: String,
    pub first_receipt_key_id: String,
    pub last_receipt_key_id: String,
    pub first_pack_root: String,
    pub last_pack_root: String,
    pub first_determinism_hash: String,
    pub last_determinism_hash: String,
    pub witness_key_id: String,
    pub witness_public_key_hex: String,
    pub witness_record_hash: String,
    pub witness_signature_hex: String,
}

pub struct TransparencyWitnessSigningKey {
    key_id: KeyId,
    seed: SigningSeed,
}

impl TransparencyWitnessSigningKey {
    pub fn from_seed_hex(key_id: &str, seed_hex: &str) -> Result<Self, String> {
        Ok(Self {
            key_id: KeyId::new(key_id.to_owned()).map_err(|error| error.to_string())?,
            seed: SigningSeed::new(decode_hex_array("transparency witness seed", seed_hex)?),
        })
    }

    pub fn key_id(&self) -> &str {
        self.key_id.as_str()
    }

    pub fn public_key_hex(&self) -> String {
        hex_lower(&ed25519_public_key(&self.seed))
    }

    fn sign_record_hash(&self, record_hash: &str) -> String {
        hex_lower(&ed25519_sign(
            &self.seed,
            &witness_signing_bytes(record_hash),
        ))
    }
}

impl fmt::Debug for TransparencyWitnessSigningKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TransparencyWitnessSigningKey")
            .field("key_id", &self.key_id)
            .field("seed", &"redacted")
            .finish()
    }
}

pub fn witness_transparency_log(
    path: &Path,
    witness_id: &str,
    witnessed_unix_seconds: u64,
    signing_key: &TransparencyWitnessSigningKey,
) -> EngineResult<TransparencyWitnessRecord> {
    let records = read_transparency_log_records(path)?;
    build_transparency_witness_record(&records, witness_id, witnessed_unix_seconds, signing_key)
}

pub fn verify_transparency_witness_record(record: &TransparencyWitnessRecord) -> EngineResult<()> {
    verify_witness_shape(record)?;
    let expected_hash = transparency_witness_record_hash(record);
    if record.witness_record_hash != expected_hash {
        return Err(witness_invariant(
            "transparency witness record hash mismatch",
        ));
    }
    let public_key = decode_hex_array(
        "transparency witness public key",
        &record.witness_public_key_hex,
    )
    .map_err(witness_invariant)?;
    let signature = decode_hex_array(
        "transparency witness signature",
        &record.witness_signature_hex,
    )
    .map_err(witness_invariant)?;
    ed25519_verify(
        &public_key,
        &witness_signing_bytes(&record.witness_record_hash),
        &signature,
    )
    .map_err(|_| witness_invariant("transparency witness signature mismatch"))
}

pub fn transparency_witness_record_hash(record: &TransparencyWitnessRecord) -> String {
    hash_value(
        TRANSPARENCY_WITNESS_RECORD_HASH_DOMAIN,
        &transparency_witness_record_body(record),
    )
}

fn build_transparency_witness_record(
    records: &[TransparencyLogRecord],
    witness_id: &str,
    witnessed_unix_seconds: u64,
    signing_key: &TransparencyWitnessSigningKey,
) -> EngineResult<TransparencyWitnessRecord> {
    let first = records
        .first()
        .ok_or_else(|| witness_invariant("transparency witness requires at least one record"))?;
    let last = records
        .last()
        .ok_or_else(|| witness_invariant("transparency witness requires at least one record"))?;
    let mut record = TransparencyWitnessRecord {
        schema_version: TRANSPARENCY_WITNESS_RECORD_SCHEMA.to_owned(),
        witness_id: required_label("witness_id", witness_id)?,
        witnessed_unix_seconds,
        witnessed_record_count: records.len() as u64,
        first_sequence: first.sequence,
        last_sequence: last.sequence,
        first_record_hash: first.record_hash.clone(),
        log_head_hash: last.record_hash.clone(),
        first_db_instance_id: first.db_instance_id.clone(),
        last_db_instance_id: last.db_instance_id.clone(),
        first_receipt_key_id: first.key_id.clone(),
        last_receipt_key_id: last.key_id.clone(),
        first_pack_root: first.pack_root.clone(),
        last_pack_root: last.pack_root.clone(),
        first_determinism_hash: first.determinism_hash.clone(),
        last_determinism_hash: last.determinism_hash.clone(),
        witness_key_id: signing_key.key_id().to_owned(),
        witness_public_key_hex: signing_key.public_key_hex(),
        witness_record_hash: String::new(),
        witness_signature_hex: String::new(),
    };
    record.witness_record_hash = transparency_witness_record_hash(&record);
    record.witness_signature_hex = signing_key.sign_record_hash(&record.witness_record_hash);
    verify_transparency_witness_record(&record)?;
    Ok(record)
}

fn verify_witness_shape(record: &TransparencyWitnessRecord) -> EngineResult<()> {
    if record.schema_version != TRANSPARENCY_WITNESS_RECORD_SCHEMA {
        return Err(witness_invariant("invalid transparency witness schema"));
    }
    if record.witness_id.trim().is_empty()
        || record.witness_key_id.trim().is_empty()
        || record.first_record_hash.trim().is_empty()
        || record.log_head_hash.trim().is_empty()
        || record.first_pack_root.trim().is_empty()
        || record.last_pack_root.trim().is_empty()
        || record.first_determinism_hash.trim().is_empty()
        || record.last_determinism_hash.trim().is_empty()
    {
        return Err(witness_invariant(
            "transparency witness has empty identity fields",
        ));
    }
    if record.witnessed_record_count == 0
        || record.first_sequence > record.last_sequence
        || record.last_sequence - record.first_sequence + 1 != record.witnessed_record_count
    {
        return Err(witness_invariant(
            "transparency witness sequence range mismatch",
        ));
    }
    Ok(())
}

fn transparency_witness_record_body(record: &TransparencyWitnessRecord) -> Value {
    json!({
        "schema_version": record.schema_version,
        "signing_domain": TRANSPARENCY_WITNESS_SIGNING_DOMAIN,
        "witness_id": record.witness_id,
        "witnessed_unix_seconds": record.witnessed_unix_seconds,
        "witnessed_record_count": record.witnessed_record_count,
        "first_sequence": record.first_sequence,
        "last_sequence": record.last_sequence,
        "first_record_hash": record.first_record_hash,
        "log_head_hash": record.log_head_hash,
        "first_db_instance_id": record.first_db_instance_id,
        "last_db_instance_id": record.last_db_instance_id,
        "first_receipt_key_id": record.first_receipt_key_id,
        "last_receipt_key_id": record.last_receipt_key_id,
        "first_pack_root": record.first_pack_root,
        "last_pack_root": record.last_pack_root,
        "first_determinism_hash": record.first_determinism_hash,
        "last_determinism_hash": record.last_determinism_hash,
        "witness_key_id": record.witness_key_id,
        "witness_public_key_hex": record.witness_public_key_hex,
    })
}

fn witness_signing_bytes(record_hash: &str) -> Vec<u8> {
    let mut bytes =
        Vec::with_capacity(TRANSPARENCY_WITNESS_SIGNING_DOMAIN.len() + 1 + record_hash.len());
    bytes.extend_from_slice(TRANSPARENCY_WITNESS_SIGNING_DOMAIN.as_bytes());
    bytes.push(0);
    bytes.extend_from_slice(record_hash.as_bytes());
    bytes
}

fn required_label(name: &str, value: &str) -> EngineResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(witness_invariant(format!("{name} is required")));
    }
    Ok(trimmed.to_owned())
}

fn decode_hex_array<const N: usize>(name: &'static str, value: &str) -> Result<[u8; N], String> {
    let value = value.trim();
    if value.len() != N * 2 {
        return Err(format!(
            "{name} must be {} hex characters, got {}",
            N * 2,
            value.len()
        ));
    }
    let mut bytes = [0_u8; N];
    for (index, chunk) in value.as_bytes().chunks_exact(2).enumerate() {
        let high = hex_nibble(chunk[0]).ok_or_else(|| format!("{name} contains invalid hex"))?;
        let low = hex_nibble(chunk[1]).ok_or_else(|| format!("{name} contains invalid hex"))?;
        bytes[index] = (high << 4) | low;
    }
    Ok(bytes)
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn witness_invariant(message: impl Into<String>) -> EngineError {
    EngineError::StorageInvariant(message.into())
}
