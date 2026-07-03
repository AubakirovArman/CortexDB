use std::collections::BTreeMap;

use cortex_core::CellId;
use cortex_crypto::{blake3_256_domain, hex_lower};
use serde_json::{json, Value};

use super::receipt_leaves::{
    access_leaves, budget_leaves, cell_set_leaves, conflict_leaves, provenance_leaves,
    verification_leaves,
};

use super::retrieved_cell_content_hash;
use crate::canonical::{canonical_context_pack_bytes, canonical_json_bytes};
use crate::context::ContextPack;
use crate::database::{CapturedAccessDenialSet, RetrievedCell};
use crate::determinism_hash::{determinism_hash, DeterminismHashInput};
use crate::error::{EngineError, EngineResult};
use crate::verification::VerificationReport;

pub const ACCOUNTABILITY_RECEIPT_BODY_SCHEMA: &str = "cortexdb.accountability.receipt_body.v1";
pub const ACCOUNTABILITY_RECEIPT_MERKLE_EMPTY_SCHEMA: &str =
    "cortexdb.accountability.merkle.empty.v1";
pub const ACCOUNTABILITY_RECEIPT_ACCESS_ROOT_DOMAIN: &str =
    "cortexdb.accountability.receipt.access_root.v1";
pub const ACCOUNTABILITY_RECEIPT_PROVENANCE_ROOT_DOMAIN: &str =
    "cortexdb.accountability.receipt.provenance_root.v1";
pub const ACCOUNTABILITY_RECEIPT_CELL_SET_ROOT_DOMAIN: &str =
    "cortexdb.accountability.receipt.cell_set_root.v1";
pub const ACCOUNTABILITY_RECEIPT_VERIFICATION_ROOT_DOMAIN: &str =
    "cortexdb.accountability.receipt.verification_root.v1";
pub const ACCOUNTABILITY_RECEIPT_BUDGET_COMMITMENT_DOMAIN: &str =
    "cortexdb.accountability.receipt.budget_commitment.v1";
pub const ACCOUNTABILITY_RECEIPT_CONFLICT_COMMITMENT_DOMAIN: &str =
    "cortexdb.accountability.receipt.conflict_commitment.v1";
pub const ACCOUNTABILITY_RECEIPT_PACK_ROOT_DOMAIN: &str =
    "cortexdb.accountability.receipt.pack_root.v1";
pub const ACCOUNTABILITY_RECEIPT_DETERMINISM_DOMAIN: &str = "cortexdb.determinism_hash.v1";
const ACCOUNTABILITY_RECEIPT_LEAF_DOMAIN_SUFFIX: &str = ".leaf.v1";
const ACCOUNTABILITY_RECEIPT_NODE_DOMAIN_SUFFIX: &str = ".node.v1";

#[derive(Clone, Debug, PartialEq)]
pub struct AccountabilityReceiptBody {
    pub schema_version: &'static str,
    pub access_root: String,
    pub provenance_root: String,
    pub cell_set_root: String,
    pub verification_root: String,
    pub budget_commitment: String,
    pub conflict_commitment: String,
    pub pack_root: String,
    pub determinism_hash: String,
    pub leaves: AccountabilityReceiptLeaves,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AccountabilityReceiptLeaves {
    pub access: Vec<Value>,
    pub provenance: Vec<Value>,
    pub cell_set: Vec<Value>,
    pub verification: Vec<Value>,
    pub budget: Vec<Value>,
    pub conflict: Vec<Value>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccountabilityDeterminismInput {
    pub query: String,
    pub agent_view_digest: Option<String>,
    pub context_options_digest: Option<String>,
    pub bitmap_program_digest: Option<String>,
    pub frozen_weights_version: String,
    pub frozen_weights_hash: String,
}

impl AccountabilityDeterminismInput {
    pub fn as_hash_input(&self) -> DeterminismHashInput<'_> {
        DeterminismHashInput {
            query: &self.query,
            agent_view_digest: self.agent_view_digest.as_deref(),
            context_options_digest: self.context_options_digest.as_deref(),
            bitmap_program_digest: self.bitmap_program_digest.as_deref(),
            frozen_weights_version: &self.frozen_weights_version,
            frozen_weights_artifact_hash: &self.frozen_weights_hash,
        }
    }
}

pub fn accountability_receipt_body(
    pack: &ContextPack,
    retrieved_cells: &[RetrievedCell],
    captured_access_denials: &CapturedAccessDenialSet,
    verification_report: Option<&VerificationReport>,
    determinism_input: &AccountabilityDeterminismInput,
) -> EngineResult<AccountabilityReceiptBody> {
    let cell_hashes = selected_cell_hashes(pack, retrieved_cells)?;
    let leaves = AccountabilityReceiptLeaves {
        access: sorted_values(access_leaves(pack, captured_access_denials)?),
        provenance: sorted_values(provenance_leaves(pack, &cell_hashes)),
        cell_set: sorted_values(cell_set_leaves(pack, &cell_hashes)),
        verification: sorted_values(verification_leaves(verification_report)),
        budget: sorted_values(budget_leaves(pack)),
        conflict: sorted_values(conflict_leaves(pack)),
    };

    Ok(AccountabilityReceiptBody {
        schema_version: ACCOUNTABILITY_RECEIPT_BODY_SCHEMA,
        access_root: merkle_root(ACCOUNTABILITY_RECEIPT_ACCESS_ROOT_DOMAIN, &leaves.access),
        provenance_root: merkle_root(
            ACCOUNTABILITY_RECEIPT_PROVENANCE_ROOT_DOMAIN,
            &leaves.provenance,
        ),
        cell_set_root: merkle_root(
            ACCOUNTABILITY_RECEIPT_CELL_SET_ROOT_DOMAIN,
            &leaves.cell_set,
        ),
        verification_root: merkle_root(
            ACCOUNTABILITY_RECEIPT_VERIFICATION_ROOT_DOMAIN,
            &leaves.verification,
        ),
        budget_commitment: merkle_root(
            ACCOUNTABILITY_RECEIPT_BUDGET_COMMITMENT_DOMAIN,
            &leaves.budget,
        ),
        conflict_commitment: merkle_root(
            ACCOUNTABILITY_RECEIPT_CONFLICT_COMMITMENT_DOMAIN,
            &leaves.conflict,
        ),
        pack_root: hash_bytes(
            ACCOUNTABILITY_RECEIPT_PACK_ROOT_DOMAIN,
            &canonical_context_pack_bytes(pack),
        ),
        determinism_hash: determinism_hash(&determinism_input.as_hash_input()),
        leaves,
    })
}

fn selected_cell_hashes(
    pack: &ContextPack,
    retrieved_cells: &[RetrievedCell],
) -> EngineResult<BTreeMap<CellId, String>> {
    let mut source_hashes = BTreeMap::new();
    for cell in retrieved_cells {
        source_hashes.insert(cell.cell_id, retrieved_cell_content_hash(cell));
    }

    let mut selected_hashes = BTreeMap::new();
    for cell in &pack.cells {
        let hash = source_hashes
            .get(&cell.cell_id)
            .ok_or(EngineError::InvalidOperation)?;
        selected_hashes.insert(cell.cell_id, hash.clone());
    }
    Ok(selected_hashes)
}

fn sorted_values(mut values: Vec<Value>) -> Vec<Value> {
    values.sort_by_key(canonical_json_bytes);
    values
}

pub(super) fn merkle_root(domain: &str, leaves: &[Value]) -> String {
    if leaves.is_empty() {
        return hash_value(
            domain,
            &json!({
                "schema_version": ACCOUNTABILITY_RECEIPT_MERKLE_EMPTY_SCHEMA,
                "leaf_count": 0,
            }),
        );
    }

    let leaf_domain = format!("{domain}{ACCOUNTABILITY_RECEIPT_LEAF_DOMAIN_SUFFIX}");
    let node_domain = format!("{domain}{ACCOUNTABILITY_RECEIPT_NODE_DOMAIN_SUFFIX}");
    let mut level = leaves
        .iter()
        .map(|leaf| hash_value(&leaf_domain, leaf))
        .collect::<Vec<_>>();
    while level.len() > 1 {
        let mut next = Vec::with_capacity(level.len().div_ceil(2));
        for pair in level.chunks(2) {
            let right = pair.get(1).unwrap_or(&pair[0]);
            next.push(hash_value(
                &node_domain,
                &json!({
                    "left": pair[0],
                    "right": right,
                }),
            ));
        }
        level = next;
    }
    level.remove(0)
}

pub(super) fn hash_value(domain: &str, value: &Value) -> String {
    hash_bytes(domain, &canonical_json_bytes(value))
}

pub(super) fn hash_bytes(domain: &str, bytes: &[u8]) -> String {
    hex_lower(&blake3_256_domain(domain, bytes))
}
