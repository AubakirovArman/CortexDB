use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::receipt::hash_value;
use super::transparency::{TransparencyLogRecord, TRANSPARENCY_LOG_RECORD_SCHEMA};
use super::transparency_inclusion::{
    TRANSPARENCY_INCLUSION_LEAF_HASH_DOMAIN, TRANSPARENCY_INCLUSION_NODE_HASH_DOMAIN,
    TRANSPARENCY_INCLUSION_PROOF_SCHEMA,
};
use crate::error::{EngineError, EngineResult};

pub const TRANSPARENCY_CONSISTENCY_SCHEMA: &str = "cortexdb.transparency.consistency.v1";
pub const TRANSPARENCY_CONSISTENCY_HASH_DOMAIN: &str = "cortexdb.transparency.consistency_hash.v1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransparencyConsistencyEvidence {
    pub schema_version: String,
    pub monitor_id: String,
    pub old_log_record_count: u64,
    pub new_log_record_count: u64,
    pub old_log_head_hash: String,
    pub new_log_head_hash: String,
    pub old_merkle_root_hash: String,
    pub new_merkle_root_hash: String,
    pub old_record_hashes: Vec<String>,
    pub new_record_hashes: Vec<String>,
    pub consistency_hash: String,
}

pub fn build_transparency_consistency_evidence(
    old_records: &[TransparencyLogRecord],
    new_records: &[TransparencyLogRecord],
    monitor_id: &str,
) -> EngineResult<TransparencyConsistencyEvidence> {
    let monitor_id = required_label("monitor_id", monitor_id)?;
    let old_hashes = record_hashes(old_records)?;
    let new_hashes = record_hashes(new_records)?;
    require_prefix(&old_hashes, &new_hashes)?;

    let mut evidence = TransparencyConsistencyEvidence {
        schema_version: TRANSPARENCY_CONSISTENCY_SCHEMA.to_owned(),
        monitor_id,
        old_log_record_count: old_hashes.len() as u64,
        new_log_record_count: new_hashes.len() as u64,
        old_log_head_hash: old_hashes
            .last()
            .ok_or_else(|| consistency_invariant("old transparency snapshot is empty"))?
            .clone(),
        new_log_head_hash: new_hashes
            .last()
            .ok_or_else(|| consistency_invariant("new transparency snapshot is empty"))?
            .clone(),
        old_merkle_root_hash: merkle_root_from_record_hashes(&old_hashes)?,
        new_merkle_root_hash: merkle_root_from_record_hashes(&new_hashes)?,
        old_record_hashes: old_hashes,
        new_record_hashes: new_hashes,
        consistency_hash: String::new(),
    };
    evidence.consistency_hash = transparency_consistency_hash(&evidence);
    verify_transparency_consistency_evidence(&evidence)?;
    Ok(evidence)
}

pub fn verify_transparency_consistency_evidence(
    evidence: &TransparencyConsistencyEvidence,
) -> EngineResult<()> {
    verify_evidence_shape(evidence)?;
    require_prefix(&evidence.old_record_hashes, &evidence.new_record_hashes)?;

    let old_head = evidence
        .old_record_hashes
        .last()
        .ok_or_else(|| consistency_invariant("old transparency snapshot is empty"))?;
    let new_head = evidence
        .new_record_hashes
        .last()
        .ok_or_else(|| consistency_invariant("new transparency snapshot is empty"))?;
    if evidence.old_log_head_hash != *old_head || evidence.new_log_head_hash != *new_head {
        return Err(consistency_invariant(
            "transparency consistency log head mismatch",
        ));
    }

    if evidence.old_merkle_root_hash != merkle_root_from_record_hashes(&evidence.old_record_hashes)?
        || evidence.new_merkle_root_hash
            != merkle_root_from_record_hashes(&evidence.new_record_hashes)?
    {
        return Err(consistency_invariant(
            "transparency consistency merkle root mismatch",
        ));
    }

    if evidence.consistency_hash != transparency_consistency_hash(evidence) {
        return Err(consistency_invariant(
            "transparency consistency hash mismatch",
        ));
    }
    Ok(())
}

fn record_hashes(records: &[TransparencyLogRecord]) -> EngineResult<Vec<String>> {
    if records.is_empty() {
        return Err(consistency_invariant(
            "transparency consistency requires non-empty snapshots",
        ));
    }
    let mut hashes = Vec::with_capacity(records.len());
    for (index, record) in records.iter().enumerate() {
        if record.schema_version != TRANSPARENCY_LOG_RECORD_SCHEMA {
            return Err(consistency_invariant(
                "transparency consistency saw invalid record schema",
            ));
        }
        if record.sequence != index as u64 {
            return Err(consistency_invariant(
                "transparency consistency requires contiguous snapshots",
            ));
        }
        require_hash("record_hash", &record.record_hash)?;
        hashes.push(record.record_hash.clone());
    }
    Ok(hashes)
}

fn verify_evidence_shape(evidence: &TransparencyConsistencyEvidence) -> EngineResult<()> {
    if evidence.schema_version != TRANSPARENCY_CONSISTENCY_SCHEMA {
        return Err(consistency_invariant(
            "invalid transparency consistency evidence schema",
        ));
    }
    if evidence.old_log_record_count == 0
        || evidence.new_log_record_count == 0
        || evidence.old_log_record_count as usize != evidence.old_record_hashes.len()
        || evidence.new_log_record_count as usize != evidence.new_record_hashes.len()
    {
        return Err(consistency_invariant(
            "transparency consistency count mismatch",
        ));
    }
    required_label("monitor_id", &evidence.monitor_id)?;
    require_hash("old_log_head_hash", &evidence.old_log_head_hash)?;
    require_hash("new_log_head_hash", &evidence.new_log_head_hash)?;
    require_hash("old_merkle_root_hash", &evidence.old_merkle_root_hash)?;
    require_hash("new_merkle_root_hash", &evidence.new_merkle_root_hash)?;
    require_hash("consistency_hash", &evidence.consistency_hash)?;
    for hash in evidence
        .old_record_hashes
        .iter()
        .chain(evidence.new_record_hashes.iter())
    {
        require_hash("record_hash", hash)?;
    }
    Ok(())
}

fn require_prefix(old_hashes: &[String], new_hashes: &[String]) -> EngineResult<()> {
    if new_hashes.len() < old_hashes.len() {
        return Err(consistency_invariant(
            "transparency consistency new snapshot is shorter",
        ));
    }
    if old_hashes != &new_hashes[..old_hashes.len()] {
        return Err(consistency_invariant(
            "transparency consistency divergent prefix",
        ));
    }
    Ok(())
}

fn transparency_consistency_hash(evidence: &TransparencyConsistencyEvidence) -> String {
    hash_value(
        TRANSPARENCY_CONSISTENCY_HASH_DOMAIN,
        &transparency_consistency_body(evidence),
    )
}

fn transparency_consistency_body(evidence: &TransparencyConsistencyEvidence) -> Value {
    json!({
        "schema_version": evidence.schema_version,
        "monitor_id": evidence.monitor_id,
        "old_log_record_count": evidence.old_log_record_count,
        "new_log_record_count": evidence.new_log_record_count,
        "old_log_head_hash": evidence.old_log_head_hash,
        "new_log_head_hash": evidence.new_log_head_hash,
        "old_merkle_root_hash": evidence.old_merkle_root_hash,
        "new_merkle_root_hash": evidence.new_merkle_root_hash,
        "old_record_hashes": evidence.old_record_hashes,
        "new_record_hashes": evidence.new_record_hashes,
    })
}

fn merkle_root_from_record_hashes(record_hashes: &[String]) -> EngineResult<String> {
    if record_hashes.is_empty() {
        return Err(consistency_invariant(
            "transparency consistency requires at least one leaf",
        ));
    }
    let mut level: Vec<String> = record_hashes
        .iter()
        .enumerate()
        .map(|(sequence, hash)| transparency_inclusion_leaf_hash(sequence as u64, hash))
        .collect();
    while level.len() > 1 {
        level = next_merkle_level(&level);
    }
    Ok(level[0].clone())
}

fn next_merkle_level(level: &[String]) -> Vec<String> {
    let mut next = Vec::with_capacity(level.len().div_ceil(2));
    for pair in level.chunks(2) {
        if pair.len() == 2 {
            next.push(transparency_inclusion_node_hash(&pair[0], &pair[1]));
        } else {
            next.push(pair[0].clone());
        }
    }
    next
}

fn transparency_inclusion_leaf_hash(sequence: u64, record_hash: &str) -> String {
    hash_value(
        TRANSPARENCY_INCLUSION_LEAF_HASH_DOMAIN,
        &json!({
            "schema_version": TRANSPARENCY_INCLUSION_PROOF_SCHEMA,
            "sequence": sequence,
            "record_hash": record_hash,
        }),
    )
}

fn transparency_inclusion_node_hash(left_hash: &str, right_hash: &str) -> String {
    hash_value(
        TRANSPARENCY_INCLUSION_NODE_HASH_DOMAIN,
        &json!({
            "left_hash": left_hash,
            "right_hash": right_hash,
        }),
    )
}

fn require_hash(name: &str, value: &str) -> EngineResult<()> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(consistency_invariant(format!(
            "{name} must be 64 hex chars"
        )));
    }
    Ok(())
}

fn required_label(name: &str, value: &str) -> EngineResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(consistency_invariant(format!("{name} is required")));
    }
    Ok(trimmed.to_owned())
}

fn consistency_invariant(message: impl Into<String>) -> EngineError {
    EngineError::StorageInvariant(message.into())
}
