use serde::{Deserialize, Serialize};
use serde_json::json;

use super::receipt::hash_value;
use super::transparency::{TransparencyLogRecord, TRANSPARENCY_LOG_RECORD_SCHEMA};
use crate::error::{EngineError, EngineResult};

pub const TRANSPARENCY_INCLUSION_PROOF_SCHEMA: &str = "cortexdb.transparency.inclusion.proof.v1";
pub const TRANSPARENCY_INCLUSION_LEAF_HASH_DOMAIN: &str =
    "cortexdb.transparency.inclusion.leaf_hash.v1";
pub const TRANSPARENCY_INCLUSION_NODE_HASH_DOMAIN: &str =
    "cortexdb.transparency.inclusion.node_hash.v1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransparencyInclusionProof {
    pub schema_version: String,
    pub sequence: u64,
    pub record_hash: String,
    pub log_record_count: u64,
    pub log_head_hash: String,
    pub merkle_leaf_hash: String,
    pub merkle_root_hash: String,
    pub proof_path: Vec<TransparencyInclusionSibling>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransparencyInclusionSibling {
    pub side: String,
    pub hash: String,
}

pub fn build_transparency_inclusion_proof(
    records: &[TransparencyLogRecord],
    sequence: u64,
) -> EngineResult<TransparencyInclusionProof> {
    validate_log_records(records)?;
    let index = usize::try_from(sequence)
        .map_err(|_| inclusion_invariant("transparency inclusion sequence is too large"))?;
    let record = records
        .get(index)
        .ok_or_else(|| inclusion_invariant("transparency inclusion sequence is out of range"))?;
    if record.sequence != sequence {
        return Err(inclusion_invariant(
            "transparency inclusion sequence does not match record",
        ));
    }

    let leaves = inclusion_leaf_hashes(records);
    let (merkle_root_hash, proof_path) = build_merkle_path(&leaves, index)?;
    let proof = TransparencyInclusionProof {
        schema_version: TRANSPARENCY_INCLUSION_PROOF_SCHEMA.to_owned(),
        sequence,
        record_hash: record.record_hash.clone(),
        log_record_count: records.len() as u64,
        log_head_hash: records
            .last()
            .ok_or_else(|| inclusion_invariant("transparency inclusion log is empty"))?
            .record_hash
            .clone(),
        merkle_leaf_hash: leaves[index].clone(),
        merkle_root_hash,
        proof_path,
    };
    verify_transparency_inclusion_proof(&proof)?;
    Ok(proof)
}

pub fn verify_transparency_inclusion_proof(proof: &TransparencyInclusionProof) -> EngineResult<()> {
    verify_proof_shape(proof)?;
    let expected_leaf = transparency_inclusion_leaf_hash(proof.sequence, &proof.record_hash);
    if proof.merkle_leaf_hash != expected_leaf {
        return Err(inclusion_invariant(
            "transparency inclusion leaf hash mismatch",
        ));
    }

    let mut current = expected_leaf;
    for sibling in &proof.proof_path {
        require_hash("proof_path.hash", &sibling.hash)?;
        match sibling.side.as_str() {
            "left" => current = transparency_inclusion_node_hash(&sibling.hash, &current),
            "right" => current = transparency_inclusion_node_hash(&current, &sibling.hash),
            _ => {
                return Err(inclusion_invariant(
                    "transparency inclusion proof path has invalid side",
                ));
            }
        }
    }

    if current != proof.merkle_root_hash {
        return Err(inclusion_invariant(
            "transparency inclusion root hash mismatch",
        ));
    }
    Ok(())
}

pub fn transparency_inclusion_root_hash(records: &[TransparencyLogRecord]) -> EngineResult<String> {
    validate_log_records(records)?;
    let leaves = inclusion_leaf_hashes(records);
    merkle_root(&leaves)
}

fn validate_log_records(records: &[TransparencyLogRecord]) -> EngineResult<()> {
    if records.is_empty() {
        return Err(inclusion_invariant(
            "transparency inclusion requires at least one record",
        ));
    }
    for (index, record) in records.iter().enumerate() {
        if record.schema_version != TRANSPARENCY_LOG_RECORD_SCHEMA {
            return Err(inclusion_invariant(
                "transparency inclusion saw invalid log record schema",
            ));
        }
        if record.sequence != index as u64 {
            return Err(inclusion_invariant(
                "transparency inclusion requires contiguous log records",
            ));
        }
        require_hash("record_hash", &record.record_hash)?;
    }
    Ok(())
}

fn verify_proof_shape(proof: &TransparencyInclusionProof) -> EngineResult<()> {
    if proof.schema_version != TRANSPARENCY_INCLUSION_PROOF_SCHEMA {
        return Err(inclusion_invariant(
            "invalid transparency inclusion proof schema",
        ));
    }
    if proof.log_record_count == 0 || proof.sequence >= proof.log_record_count {
        return Err(inclusion_invariant(
            "transparency inclusion sequence is outside log record count",
        ));
    }
    require_hash("record_hash", &proof.record_hash)?;
    require_hash("log_head_hash", &proof.log_head_hash)?;
    require_hash("merkle_leaf_hash", &proof.merkle_leaf_hash)?;
    require_hash("merkle_root_hash", &proof.merkle_root_hash)?;
    Ok(())
}

fn inclusion_leaf_hashes(records: &[TransparencyLogRecord]) -> Vec<String> {
    records
        .iter()
        .map(|record| transparency_inclusion_leaf_hash(record.sequence, &record.record_hash))
        .collect()
}

fn build_merkle_path(
    leaves: &[String],
    mut index: usize,
) -> EngineResult<(String, Vec<TransparencyInclusionSibling>)> {
    if index >= leaves.len() {
        return Err(inclusion_invariant(
            "transparency inclusion leaf is missing",
        ));
    }
    let mut level = leaves.to_vec();
    let mut path = Vec::new();

    while level.len() > 1 {
        if index.is_multiple_of(2) {
            if let Some(sibling) = level.get(index + 1) {
                path.push(TransparencyInclusionSibling {
                    side: "right".to_owned(),
                    hash: sibling.clone(),
                });
            }
        } else {
            path.push(TransparencyInclusionSibling {
                side: "left".to_owned(),
                hash: level[index - 1].clone(),
            });
        }

        level = next_merkle_level(&level);
        index /= 2;
    }

    Ok((level[0].clone(), path))
}

fn merkle_root(leaves: &[String]) -> EngineResult<String> {
    let mut level = leaves.to_vec();
    if level.is_empty() {
        return Err(inclusion_invariant(
            "transparency inclusion requires at least one leaf",
        ));
    }
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
        return Err(inclusion_invariant(format!("{name} must be 64 hex chars")));
    }
    Ok(())
}

fn inclusion_invariant(message: impl Into<String>) -> EngineError {
    EngineError::StorageInvariant(message.into())
}
