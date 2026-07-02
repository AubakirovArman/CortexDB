use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::receipt::hash_value;
use super::transparency_witness::{verify_transparency_witness_record, TransparencyWitnessRecord};
use crate::error::{EngineError, EngineResult};

pub const TRANSPARENCY_WITNESS_QUORUM_SCHEMA: &str = "cortexdb.transparency.witness.quorum.v1";
pub const TRANSPARENCY_WITNESS_QUORUM_HASH_DOMAIN: &str =
    "cortexdb.transparency.witness.quorum_hash.v1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransparencyWitnessQuorumEvidence {
    pub schema_version: String,
    pub quorum_id: String,
    pub min_witnesses: u64,
    pub witness_count: u64,
    pub witness_ids: Vec<String>,
    pub witness_key_ids: Vec<String>,
    pub witness_public_key_hexes: Vec<String>,
    pub witness_record_hashes: Vec<String>,
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
    pub quorum_hash: String,
}

pub fn verify_transparency_witness_quorum(
    records: &[TransparencyWitnessRecord],
    quorum_id: &str,
    min_witnesses: u64,
) -> EngineResult<TransparencyWitnessQuorumEvidence> {
    if min_witnesses < 2 {
        return Err(quorum_invariant(
            "transparency witness quorum requires at least two witnesses",
        ));
    }
    if records.len() < min_witnesses as usize {
        return Err(quorum_invariant(
            "transparency witness quorum has insufficient witnesses",
        ));
    }

    let quorum_id = required_label("quorum_id", quorum_id)?;
    let mut verified: Vec<&TransparencyWitnessRecord> = Vec::with_capacity(records.len());
    for record in records {
        verify_transparency_witness_record(record)?;
        verified.push(record);
    }
    verified.sort_by(|left, right| {
        left.witness_id
            .cmp(&right.witness_id)
            .then_with(|| left.witness_key_id.cmp(&right.witness_key_id))
            .then_with(|| left.witness_record_hash.cmp(&right.witness_record_hash))
    });

    let first = *verified
        .first()
        .ok_or_else(|| quorum_invariant("transparency witness quorum is empty"))?;
    let mut witness_ids = BTreeSet::new();
    let mut witness_key_ids = BTreeSet::new();
    let mut witness_public_keys = BTreeSet::new();
    let mut witness_record_hashes = Vec::with_capacity(verified.len());

    for record in &verified {
        require_same_witnessed_log(first, record)?;
        insert_unique(&mut witness_ids, &record.witness_id, "witness_id")?;
        insert_unique(
            &mut witness_key_ids,
            &record.witness_key_id,
            "witness_key_id",
        )?;
        insert_unique(
            &mut witness_public_keys,
            &record.witness_public_key_hex,
            "witness_public_key_hex",
        )?;
        witness_record_hashes.push(record.witness_record_hash.clone());
    }

    let mut evidence = TransparencyWitnessQuorumEvidence {
        schema_version: TRANSPARENCY_WITNESS_QUORUM_SCHEMA.to_owned(),
        quorum_id,
        min_witnesses,
        witness_count: verified.len() as u64,
        witness_ids: witness_ids.into_iter().collect(),
        witness_key_ids: witness_key_ids.into_iter().collect(),
        witness_public_key_hexes: witness_public_keys.into_iter().collect(),
        witness_record_hashes,
        witnessed_record_count: first.witnessed_record_count,
        first_sequence: first.first_sequence,
        last_sequence: first.last_sequence,
        first_record_hash: first.first_record_hash.clone(),
        log_head_hash: first.log_head_hash.clone(),
        first_db_instance_id: first.first_db_instance_id.clone(),
        last_db_instance_id: first.last_db_instance_id.clone(),
        first_receipt_key_id: first.first_receipt_key_id.clone(),
        last_receipt_key_id: first.last_receipt_key_id.clone(),
        first_pack_root: first.first_pack_root.clone(),
        last_pack_root: first.last_pack_root.clone(),
        first_determinism_hash: first.first_determinism_hash.clone(),
        last_determinism_hash: first.last_determinism_hash.clone(),
        quorum_hash: String::new(),
    };
    evidence.quorum_hash = transparency_witness_quorum_hash(&evidence);
    Ok(evidence)
}

pub fn transparency_witness_quorum_hash(evidence: &TransparencyWitnessQuorumEvidence) -> String {
    hash_value(
        TRANSPARENCY_WITNESS_QUORUM_HASH_DOMAIN,
        &transparency_witness_quorum_body(evidence),
    )
}

fn require_same_witnessed_log(
    expected: &TransparencyWitnessRecord,
    actual: &TransparencyWitnessRecord,
) -> EngineResult<()> {
    if expected.witnessed_record_count != actual.witnessed_record_count
        || expected.first_sequence != actual.first_sequence
        || expected.last_sequence != actual.last_sequence
        || expected.first_record_hash != actual.first_record_hash
        || expected.log_head_hash != actual.log_head_hash
        || expected.first_db_instance_id != actual.first_db_instance_id
        || expected.last_db_instance_id != actual.last_db_instance_id
        || expected.first_receipt_key_id != actual.first_receipt_key_id
        || expected.last_receipt_key_id != actual.last_receipt_key_id
        || expected.first_pack_root != actual.first_pack_root
        || expected.last_pack_root != actual.last_pack_root
        || expected.first_determinism_hash != actual.first_determinism_hash
        || expected.last_determinism_hash != actual.last_determinism_hash
    {
        return Err(quorum_invariant(
            "transparency witness quorum has mismatched witnessed log heads",
        ));
    }
    Ok(())
}

fn transparency_witness_quorum_body(evidence: &TransparencyWitnessQuorumEvidence) -> Value {
    json!({
        "schema_version": evidence.schema_version,
        "quorum_id": evidence.quorum_id,
        "min_witnesses": evidence.min_witnesses,
        "witness_count": evidence.witness_count,
        "witness_ids": evidence.witness_ids,
        "witness_key_ids": evidence.witness_key_ids,
        "witness_public_key_hexes": evidence.witness_public_key_hexes,
        "witness_record_hashes": evidence.witness_record_hashes,
        "witnessed_record_count": evidence.witnessed_record_count,
        "first_sequence": evidence.first_sequence,
        "last_sequence": evidence.last_sequence,
        "first_record_hash": evidence.first_record_hash,
        "log_head_hash": evidence.log_head_hash,
        "first_db_instance_id": evidence.first_db_instance_id,
        "last_db_instance_id": evidence.last_db_instance_id,
        "first_receipt_key_id": evidence.first_receipt_key_id,
        "last_receipt_key_id": evidence.last_receipt_key_id,
        "first_pack_root": evidence.first_pack_root,
        "last_pack_root": evidence.last_pack_root,
        "first_determinism_hash": evidence.first_determinism_hash,
        "last_determinism_hash": evidence.last_determinism_hash,
    })
}

fn insert_unique(set: &mut BTreeSet<String>, value: &str, name: &str) -> EngineResult<()> {
    if !set.insert(value.to_owned()) {
        return Err(quorum_invariant(format!(
            "transparency witness quorum has duplicate {name}",
        )));
    }
    Ok(())
}

fn required_label(name: &str, value: &str) -> EngineResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(quorum_invariant(format!("{name} is required")));
    }
    Ok(trimmed.to_owned())
}

fn quorum_invariant(message: impl Into<String>) -> EngineError {
    EngineError::StorageInvariant(message.into())
}
