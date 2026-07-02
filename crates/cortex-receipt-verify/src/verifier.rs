use std::collections::{BTreeMap, BTreeSet};

use cortex_crypto::{ReceiptPublicKey, ReceiptSignature};
use serde_json::Value;

use crate::canonical::canonical_json_bytes;
use crate::hex::decode_hex;
use crate::model::{AdmittedCellInput, VerifyInput};
use crate::receipt_hash::{
    canonical_header_bytes, hash_bytes, hash_value, merkle_root, ACCESS_ROOT_DOMAIN,
    BUDGET_COMMITMENT_DOMAIN, CELL_SET_ROOT_DOMAIN, CONFLICT_COMMITMENT_DOMAIN, DETERMINISM_DOMAIN,
    HASH_ALG, PACK_ROOT_DOMAIN, PROVENANCE_ROOT_DOMAIN, RECEIPT_SCHEMA, SIG_ALG,
    VERIFICATION_ROOT_DOMAIN,
};

pub type VerifyResult<T> = Result<T, VerifyError>;

#[derive(Debug, PartialEq, Eq)]
pub enum VerifyError {
    InvalidSchema(&'static str),
    InvalidSignature,
    RootMismatch(&'static str),
    InvalidAccessLeaf(String),
    InvalidCellEvidence(String),
    InvalidProvenance(String),
    InvalidBudget(String),
    InvalidVerificationLeaf(String),
}

const VERIFY_INPUT_SCHEMA: &str = "cortexdb.accountability_receipt_verify_input.v1";

pub fn verify_input(input: &VerifyInput) -> VerifyResult<()> {
    verify_header_shape(input)?;
    verify_signature(input)?;
    verify_roots(input)?;
    verify_access(&input.receipt.leaves.access)?;
    let admitted = admitted_cells_by_id(&input.admitted_cells);
    verify_cell_set(&input.receipt.leaves.cell_set, &admitted)?;
    verify_provenance(&input.receipt.leaves.provenance, &admitted)?;
    verify_budget(&input.receipt.leaves.budget)?;
    verify_verification_references(&input.receipt.leaves.verification, &admitted)?;
    Ok(())
}

fn verify_header_shape(input: &VerifyInput) -> VerifyResult<()> {
    let header = &input.receipt.header;
    if input.schema_version != VERIFY_INPUT_SCHEMA {
        return Err(VerifyError::InvalidSchema("verify_input.schema_version"));
    }
    if input.receipt.schema_version != RECEIPT_SCHEMA
        || header.schema_version != RECEIPT_SCHEMA
        || header.hash_alg != HASH_ALG
        || header.sig_alg != SIG_ALG
        || header.signature.sig_alg != SIG_ALG
        || header.key_id != header.signature.key_id
        || header.db_instance_id.trim().is_empty()
        || !is_hex_hash(&header.audit_chain_head)
    {
        return Err(VerifyError::InvalidSchema("receipt.header"));
    }
    if input.public_key.key_id != header.key_id
        || input.public_key.public_key_hex != header.signature.public_key_hex
    {
        return Err(VerifyError::InvalidSignature);
    }
    Ok(())
}

fn verify_signature(input: &VerifyInput) -> VerifyResult<()> {
    let header = &input.receipt.header;
    let public_key =
        ReceiptPublicKey::from_hex(&input.public_key.key_id, &input.public_key.public_key_hex)
            .map_err(|_| VerifyError::InvalidSignature)?;
    let signature = ReceiptSignature::from_hex(&header.signature.signature_hex)
        .map_err(|_| VerifyError::InvalidSignature)?;
    public_key
        .verify(&canonical_header_bytes(header), &signature)
        .map_err(|_| VerifyError::InvalidSignature)
}

fn verify_roots(input: &VerifyInput) -> VerifyResult<()> {
    let header = &input.receipt.header;
    let leaves = &input.receipt.leaves;
    compare_root(
        "access_root",
        &header.access_root,
        merkle_root(ACCESS_ROOT_DOMAIN, &leaves.access),
    )?;
    compare_root(
        "provenance_root",
        &header.provenance_root,
        merkle_root(PROVENANCE_ROOT_DOMAIN, &leaves.provenance),
    )?;
    compare_root(
        "cell_set_root",
        &header.cell_set_root,
        merkle_root(CELL_SET_ROOT_DOMAIN, &leaves.cell_set),
    )?;
    compare_root(
        "verification_root",
        &header.verification_root,
        merkle_root(VERIFICATION_ROOT_DOMAIN, &leaves.verification),
    )?;
    compare_root(
        "budget_commitment",
        &header.budget_commitment,
        merkle_root(BUDGET_COMMITMENT_DOMAIN, &leaves.budget),
    )?;
    compare_root(
        "conflict_commitment",
        &header.conflict_commitment,
        merkle_root(CONFLICT_COMMITMENT_DOMAIN, &leaves.conflict),
    )?;
    compare_root(
        "pack_root",
        &header.pack_root,
        hash_bytes(PACK_ROOT_DOMAIN, &canonical_json_bytes(&input.pack)),
    )?;
    compare_root(
        "determinism_hash",
        &header.determinism_hash,
        hash_value(DETERMINISM_DOMAIN, &input.determinism_input),
    )
}

fn compare_root(label: &'static str, expected: &str, actual: String) -> VerifyResult<()> {
    if expected == actual {
        Ok(())
    } else {
        Err(VerifyError::RootMismatch(label))
    }
}

fn is_hex_hash(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn verify_access(leaves: &[Value]) -> VerifyResult<()> {
    for leaf in leaves {
        if leaf.get("leaf_type").and_then(Value::as_str) == Some("admitted_cell")
            && leaf.get("decision").and_then(Value::as_str) != Some("allowed")
        {
            return Err(VerifyError::InvalidAccessLeaf(
                "admitted cell access leaf must be allowed".to_owned(),
            ));
        }
    }
    Ok(())
}

fn verify_cell_set(
    leaves: &[Value],
    admitted: &BTreeMap<u64, &AdmittedCellInput>,
) -> VerifyResult<()> {
    for leaf in leaves {
        let cell_id = value_u64(leaf, "cell_id").ok_or_else(|| {
            VerifyError::InvalidCellEvidence("cell_set missing cell_id".to_owned())
        })?;
        let hash = value_str(leaf, "cell_content_hash").ok_or_else(|| {
            VerifyError::InvalidCellEvidence("cell_set missing cell_content_hash".to_owned())
        })?;
        let Some(cell) = admitted.get(&cell_id) else {
            return Err(VerifyError::InvalidCellEvidence(format!(
                "cell_set references missing admitted cell {cell_id}"
            )));
        };
        if cell.cell_content_hash != hash {
            return Err(VerifyError::InvalidCellEvidence(format!(
                "cell_content_hash mismatch for cell {cell_id}"
            )));
        }
    }
    Ok(())
}

fn verify_provenance(
    leaves: &[Value],
    admitted: &BTreeMap<u64, &AdmittedCellInput>,
) -> VerifyResult<()> {
    for leaf in leaves {
        let cell_id = value_u64(leaf, "cell_id").ok_or_else(|| {
            VerifyError::InvalidProvenance("provenance missing cell_id".to_owned())
        })?;
        let hash = value_str(leaf, "cell_content_hash").ok_or_else(|| {
            VerifyError::InvalidProvenance("provenance missing cell_content_hash".to_owned())
        })?;
        let Some(cell) = admitted.get(&cell_id) else {
            return Err(VerifyError::InvalidProvenance(format!(
                "provenance references missing admitted cell {cell_id}"
            )));
        };
        if cell.cell_content_hash != hash {
            return Err(VerifyError::InvalidProvenance(format!(
                "provenance hash mismatch for cell {cell_id}"
            )));
        }
        verify_span_if_present(leaf, admitted)?;
    }
    Ok(())
}

fn verify_span_if_present(
    leaf: &Value,
    admitted: &BTreeMap<u64, &AdmittedCellInput>,
) -> VerifyResult<()> {
    let Some(source_cell_id) = value_u64(leaf, "source_cell_id") else {
        return Ok(());
    };
    let start = value_u64(leaf, "source_byte_start").ok_or_else(|| {
        VerifyError::InvalidProvenance(
            "span start missing when source_cell_id is present".to_owned(),
        )
    })? as usize;
    let end = value_u64(leaf, "source_byte_end").ok_or_else(|| {
        VerifyError::InvalidProvenance("span end missing when source_cell_id is present".to_owned())
    })? as usize;
    let Some(raw_hex) = admitted
        .get(&source_cell_id)
        .and_then(|cell| cell.raw_content_hex.as_ref())
    else {
        return Ok(());
    };
    let raw = decode_hex(raw_hex)
        .map_err(|error| VerifyError::InvalidProvenance(format!("raw_content_hex: {error}")))?;
    if start > end || end > raw.len() {
        return Err(VerifyError::InvalidProvenance(format!(
            "span {start}..{end} is outside source cell {source_cell_id}"
        )));
    }
    Ok(())
}

fn verify_budget(leaves: &[Value]) -> VerifyResult<()> {
    let mut token_budget = None;
    let mut estimated_total = None;
    let mut cell_sum: u64 = 0;
    for leaf in leaves {
        if value_u64(leaf, "cell_id").is_some() {
            cell_sum =
                cell_sum.saturating_add(value_u64(leaf, "cell_estimated_tokens").unwrap_or(0));
        } else {
            token_budget = value_u64(leaf, "token_budget_tokens");
            estimated_total = value_u64(leaf, "estimated_tokens");
        }
    }
    let budget = token_budget
        .ok_or_else(|| VerifyError::InvalidBudget("missing summary leaf".to_owned()))?;
    let estimated = estimated_total
        .ok_or_else(|| VerifyError::InvalidBudget("missing estimated_tokens".to_owned()))?;
    if cell_sum > budget || estimated > budget || cell_sum != estimated {
        return Err(VerifyError::InvalidBudget(format!(
            "budget mismatch: cell_sum={cell_sum} estimated={estimated} budget={budget}"
        )));
    }
    Ok(())
}

fn verify_verification_references(
    leaves: &[Value],
    admitted: &BTreeMap<u64, &AdmittedCellInput>,
) -> VerifyResult<()> {
    let admitted_ids = admitted.keys().copied().collect::<BTreeSet<_>>();
    for leaf in leaves {
        if let Some(cell_id) = value_u64(leaf, "cell_id") {
            if !admitted_ids.contains(&cell_id) {
                return Err(VerifyError::InvalidVerificationLeaf(format!(
                    "verification leaf references missing admitted cell {cell_id}"
                )));
            }
        }
    }
    Ok(())
}

fn admitted_cells_by_id(cells: &[AdmittedCellInput]) -> BTreeMap<u64, &AdmittedCellInput> {
    cells.iter().map(|cell| (cell.cell_id, cell)).collect()
}

fn value_u64(value: &Value, key: &str) -> Option<u64> {
    value.get(key).and_then(Value::as_u64)
}

fn value_str<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}
