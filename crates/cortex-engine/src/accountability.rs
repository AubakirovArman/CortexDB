use cortex_core::{CellDescriptor, CellId};
use cortex_crypto::{blake3_256_domain, hex_lower};
use serde_json::{json, Value};

mod receipt;
#[cfg(test)]
mod receipt_cross_language_tests;
mod receipt_header;
mod receipt_leaves;
#[cfg(test)]
mod receipt_sign_tests;
#[cfg(test)]
mod receipt_tests;
mod transparency;
mod transparency_availability;
#[cfg(test)]
mod transparency_availability_tests;
mod transparency_consistency;
#[cfg(test)]
mod transparency_consistency_tests;
mod transparency_gossip;
#[cfg(test)]
mod transparency_gossip_tests;
mod transparency_inclusion;
#[cfg(test)]
mod transparency_inclusion_tests;
mod transparency_quorum;
#[cfg(test)]
mod transparency_quorum_tests;
mod transparency_slo;
#[cfg(test)]
mod transparency_slo_tests;
#[cfg(test)]
mod transparency_tests;
mod transparency_witness;

use crate::canonical::canonical_json_bytes;
use crate::database::RetrievedCell;
pub use receipt::{
    accountability_receipt_body, AccountabilityDeterminismInput, AccountabilityReceiptBody,
    AccountabilityReceiptLeaves, ACCOUNTABILITY_RECEIPT_ACCESS_ROOT_DOMAIN,
    ACCOUNTABILITY_RECEIPT_BODY_SCHEMA, ACCOUNTABILITY_RECEIPT_BUDGET_COMMITMENT_DOMAIN,
    ACCOUNTABILITY_RECEIPT_CELL_SET_ROOT_DOMAIN, ACCOUNTABILITY_RECEIPT_CONFLICT_COMMITMENT_DOMAIN,
    ACCOUNTABILITY_RECEIPT_DETERMINISM_DOMAIN, ACCOUNTABILITY_RECEIPT_MERKLE_EMPTY_SCHEMA,
    ACCOUNTABILITY_RECEIPT_PACK_ROOT_DOMAIN, ACCOUNTABILITY_RECEIPT_PROVENANCE_ROOT_DOMAIN,
    ACCOUNTABILITY_RECEIPT_VERIFICATION_ROOT_DOMAIN,
};
pub use receipt_header::{
    accountability_receipt_header_value, canonical_accountability_receipt_header_bytes,
    sign_accountability_receipt_header, sign_accountability_receipt_header_with_signer,
    verify_accountability_receipt_header, AccountabilityReceiptHeader,
    AccountabilityReceiptHeaderSigner, AccountabilityReceiptSignature,
    ACCOUNTABILITY_RECEIPT_HASH_ALG, ACCOUNTABILITY_RECEIPT_SCHEMA_VERSION,
    ACCOUNTABILITY_RECEIPT_SIG_ALG,
};
pub use transparency::{
    append_transparency_log_record, read_transparency_log_records, TransparencyLogRecord,
    TRANSPARENCY_LOG_RECORD_HASH_DOMAIN, TRANSPARENCY_LOG_RECORD_SCHEMA,
    TRANSPARENCY_LOG_ZERO_HASH,
};
pub use transparency_availability::{
    build_transparency_availability_evidence, verify_transparency_availability_evidence,
    TransparencyAvailabilityEvidence, TransparencyAvailabilityObservation,
    TransparencyAvailabilityPolicy, TRANSPARENCY_AVAILABILITY_EVIDENCE_SCHEMA,
    TRANSPARENCY_AVAILABILITY_HASH_DOMAIN, TRANSPARENCY_AVAILABILITY_OBSERVATION_SCHEMA,
};
pub use transparency_consistency::{
    build_transparency_consistency_evidence, verify_transparency_consistency_evidence,
    TransparencyConsistencyEvidence, TRANSPARENCY_CONSISTENCY_HASH_DOMAIN,
    TRANSPARENCY_CONSISTENCY_SCHEMA,
};
pub use transparency_gossip::{
    build_transparency_gossip_evidence, verify_transparency_gossip_evidence,
    TransparencyGossipEvidence, TransparencyGossipExchange, TransparencyGossipPolicy,
    TRANSPARENCY_GOSSIP_EVIDENCE_SCHEMA, TRANSPARENCY_GOSSIP_EXCHANGE_SCHEMA,
    TRANSPARENCY_GOSSIP_HASH_DOMAIN,
};
pub use transparency_inclusion::{
    build_transparency_inclusion_proof, transparency_inclusion_root_hash,
    verify_transparency_inclusion_proof, TransparencyInclusionProof, TransparencyInclusionSibling,
    TRANSPARENCY_INCLUSION_LEAF_HASH_DOMAIN, TRANSPARENCY_INCLUSION_NODE_HASH_DOMAIN,
    TRANSPARENCY_INCLUSION_PROOF_SCHEMA,
};
pub use transparency_quorum::{
    transparency_witness_quorum_hash, verify_transparency_witness_quorum,
    TransparencyWitnessQuorumEvidence, TRANSPARENCY_WITNESS_QUORUM_HASH_DOMAIN,
    TRANSPARENCY_WITNESS_QUORUM_SCHEMA,
};
pub use transparency_slo::{
    build_transparency_slo_evidence, verify_transparency_slo_evidence, TransparencySloEvidence,
    TransparencySloPolicy, TransparencySloWindow, TRANSPARENCY_SLO_EVIDENCE_SCHEMA,
    TRANSPARENCY_SLO_HASH_DOMAIN, TRANSPARENCY_SLO_WINDOW_SCHEMA,
};
pub use transparency_witness::{
    transparency_witness_record_hash, verify_transparency_witness_record, witness_transparency_log,
    TransparencyWitnessRecord, TransparencyWitnessSigningKey,
    TRANSPARENCY_WITNESS_RECORD_HASH_DOMAIN, TRANSPARENCY_WITNESS_RECORD_SCHEMA,
    TRANSPARENCY_WITNESS_SIGNING_DOMAIN,
};

pub const ACCOUNTABILITY_CELL_BYTES_SCHEMA: &str = "cortexdb.accountability.cell.v1";
pub const ACCOUNTABILITY_CELL_HASH_DOMAIN: &str = "cortexdb.accountability.cell_hash.v1";
pub const FAIL_CLOSED_INVARIANT_MODEL_SCHEMA: &str = "cortexdb.fail_closed.invariant_model.v1";
pub const FAIL_CLOSED_INVARIANT_MODEL_HASH_DOMAIN: &str =
    "cortexdb.fail_closed.invariant_model_hash.v1";
pub const FAIL_CLOSED_INVARIANT_MODEL_HASH: &str =
    "cb0e81f8fd07d20769e27ea3f8bd3b4e7459e72504a939b393470d433df40e79";
pub const FAIL_CLOSED_INVARIANT_MODEL_SPEC: &str = concat!(
    "schema_version=cortexdb.fail_closed.invariant_model.v1\n",
    "invariant=admitted_set subseteq agent_allowed intersect live intersect where\n",
    "bitmap.seed=PushAgentAllowed PushLive And\n",
    "bitmap.where_composition=every WHERE predicate is AND-composed onto the seed\n",
    "bitmap.Push(handle)=provider bitmap(handle)\n",
    "bitmap.PushUniverse=provider universe\n",
    "bitmap.PushAgentAllowed=provider agent_allowed\n",
    "bitmap.PushLive=provider live\n",
    "bitmap.And=set intersection\n",
    "bitmap.Or=set union\n",
    "bitmap.Not=universe-relative complement\n",
    "engine.bitmap_scan=BitmapIndexScan evaluates plan.bitmap_program\n",
    "engine.persisted_allowed=allowed_cells_from_bound_retrieve_plan(plan) filters persisted lexical/vector/hybrid candidates\n",
    "generator=deterministic-lcg randomized catalog/view/status/type/where cases with positive admitted cases required\n",
);

pub fn fail_closed_invariant_model_hash() -> String {
    hex_lower(&blake3_256_domain(
        FAIL_CLOSED_INVARIANT_MODEL_HASH_DOMAIN,
        FAIL_CLOSED_INVARIANT_MODEL_SPEC.as_bytes(),
    ))
}

pub fn canonical_cell_bytes(
    cell_id: CellId,
    payload: &[u8],
    descriptor: &CellDescriptor,
) -> Vec<u8> {
    canonical_json_bytes(&json!({
        "schema_version": ACCOUNTABILITY_CELL_BYTES_SCHEMA,
        "cell_id": cell_id.0,
        "payload_hex": hex_lower(payload),
        "descriptor": descriptor_value(descriptor),
    }))
}

pub fn cell_content_hash(cell_id: CellId, payload: &[u8], descriptor: &CellDescriptor) -> String {
    let canonical = canonical_cell_bytes(cell_id, payload, descriptor);
    hex_lower(&blake3_256_domain(
        ACCOUNTABILITY_CELL_HASH_DOMAIN,
        &canonical,
    ))
}

pub fn retrieved_cell_content_hash(cell: &RetrievedCell) -> String {
    cell_content_hash(cell.cell_id, &cell.payload, &cell.descriptor)
}

fn descriptor_value(descriptor: &CellDescriptor) -> Value {
    json!({
        "scope": descriptor.scope,
        "status": descriptor.status,
        "cell_type": descriptor.cell_type.as_str(),
        "memory_type": descriptor.memory_type,
        "ttl_seconds": descriptor.ttl_seconds,
        "created_unix_seconds": descriptor.created_unix_seconds,
        "source_trust_q16": descriptor.source_trust_q16,
        "source": descriptor.source,
        "citation": descriptor.citation,
        "content_hash": descriptor.content_hash,
        "source_id": descriptor.source_id,
        "source_url": descriptor.source_url,
        "document_id": descriptor.document_id,
        "page": descriptor.page,
        "row": descriptor.row,
        "cell_range": descriptor.cell_range,
        "json_path": descriptor.json_path,
        "confidence_q16": descriptor.confidence_q16,
        "parent_id": descriptor.parent_id,
        "valid_from": descriptor.valid_from,
        "valid_to": descriptor.valid_to,
        "session_id": descriptor.session_id,
        "session_kind": descriptor.session_kind,
    })
}
