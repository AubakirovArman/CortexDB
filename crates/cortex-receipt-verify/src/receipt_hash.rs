use cortex_crypto::{blake3_256_domain, hex_lower, RECEIPT_SIGNING_DOMAIN};
use serde_json::{json, Value};

use crate::canonical::canonical_json_bytes;
use crate::model::ReceiptHeader;

pub const RECEIPT_SCHEMA: &str = "accountability_receipt.v1";
pub const HASH_ALG: &str = "blake3-256";
pub const SIG_ALG: &str = "ed25519";
pub const ACCESS_ROOT_DOMAIN: &str = "cortexdb.accountability.receipt.access_root.v1";
pub const PROVENANCE_ROOT_DOMAIN: &str = "cortexdb.accountability.receipt.provenance_root.v1";
pub const CELL_SET_ROOT_DOMAIN: &str = "cortexdb.accountability.receipt.cell_set_root.v1";
pub const VERIFICATION_ROOT_DOMAIN: &str = "cortexdb.accountability.receipt.verification_root.v1";
pub const BUDGET_COMMITMENT_DOMAIN: &str = "cortexdb.accountability.receipt.budget_commitment.v1";
pub const CONFLICT_COMMITMENT_DOMAIN: &str =
    "cortexdb.accountability.receipt.conflict_commitment.v1";
pub const PACK_ROOT_DOMAIN: &str = "cortexdb.accountability.receipt.pack_root.v1";
pub const DETERMINISM_DOMAIN: &str = "cortexdb.determinism_hash.v1";

const MERKLE_EMPTY_SCHEMA: &str = "cortexdb.accountability.merkle.empty.v1";
const LEAF_DOMAIN_SUFFIX: &str = ".leaf.v1";
const NODE_DOMAIN_SUFFIX: &str = ".node.v1";

pub fn canonical_header_bytes(header: &ReceiptHeader) -> Vec<u8> {
    canonical_json_bytes(&json!({
        "schema_version": header.schema_version,
        "hash_alg": header.hash_alg,
        "sig_alg": header.sig_alg,
        "signing_domain": RECEIPT_SIGNING_DOMAIN,
        "db_instance_id": header.db_instance_id,
        "key_id": header.key_id,
        "created_unix_seconds": header.created_unix_seconds,
        "access_root": header.access_root,
        "provenance_root": header.provenance_root,
        "cell_set_root": header.cell_set_root,
        "verification_root": header.verification_root,
        "budget_commitment": header.budget_commitment,
        "conflict_commitment": header.conflict_commitment,
        "pack_root": header.pack_root,
        "determinism_hash": header.determinism_hash,
        "audit_chain_head": header.audit_chain_head,
    }))
}

pub fn merkle_root(domain: &str, leaves: &[Value]) -> String {
    if leaves.is_empty() {
        return hash_value(
            domain,
            &json!({
                "schema_version": MERKLE_EMPTY_SCHEMA,
                "leaf_count": 0,
            }),
        );
    }

    let mut leaves = leaves.to_vec();
    leaves.sort_by_key(canonical_json_bytes);

    let leaf_domain = format!("{domain}{LEAF_DOMAIN_SUFFIX}");
    let node_domain = format!("{domain}{NODE_DOMAIN_SUFFIX}");
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

pub fn hash_value(domain: &str, value: &Value) -> String {
    hash_bytes(domain, &canonical_json_bytes(value))
}

pub fn hash_bytes(domain: &str, bytes: &[u8]) -> String {
    hex_lower(&blake3_256_domain(domain, bytes))
}
