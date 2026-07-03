use cortex_core::{CellDescriptor, CellId};

use super::*;
use crate::context::{
    ContextAccessDecision, ContextAccessDecisionOutcome, ContextPack, ContextPackAnomaly,
    ContextPackAnomalyCode, ContextPackCell, ContextSpanProvenance,
};
use crate::database::{CapturedAccessDenial, CapturedAccessDenialSet, RetrievedCell};
use crate::error::EngineError;

// C4-2: the receipt's blake3 Merkle-root construction is language-independent.
// A pure-Python re-implementation (scripts/canonical_jcs_cross_language_check.py)
// committed these roots; this asserts the Rust merkle_root produces the same
// bytes, so the tree construction + domain hashing reproduce cross-language.
#[test]
fn merkle_root_matches_cross_language_vectors() {
    let vectors: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../../fixtures/canonical/merkle_conformance_vectors.v1.json"
    ))
    .expect("merkle conformance vectors parse");
    assert!(!vectors.is_empty());
    for (index, entry) in vectors.iter().enumerate() {
        let domain = entry["domain"].as_str().expect("domain");
        let leaves: Vec<serde_json::Value> = entry["leaves"].as_array().expect("leaves").clone();
        let expected = entry["merkle_root_blake3"].as_str().expect("root");
        let root = super::receipt::merkle_root(domain, &leaves);
        assert_eq!(
            root, expected,
            "merkle vector {index}: Rust root differs from the committed (Python) root"
        );
    }
}

// C4-2: the receipt's Ed25519 signing (over a domain-wrapped message) is
// language-independent. Ed25519 (RFC 8032) is deterministic, so a Python
// `cryptography` re-derivation must produce the identical signature + public key;
// this asserts the Rust ReceiptSigningKey matches the committed Python-derived
// vectors byte-for-byte.
#[test]
fn ed25519_signature_matches_cross_language_vectors() {
    use cortex_crypto::ReceiptSigningKey;
    let vectors: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../../fixtures/canonical/ed25519_conformance_vectors.v1.json"
    ))
    .expect("ed25519 conformance vectors parse");
    assert!(!vectors.is_empty());
    for (index, entry) in vectors.iter().enumerate() {
        let seed_hex = entry["seed_hex"].as_str().expect("seed");
        let message = entry["message"].as_str().expect("message");
        let key =
            ReceiptSigningKey::from_seed_hex("cross-lang-test", seed_hex).expect("seed decodes");
        assert_eq!(
            key.public_key().to_hex(),
            entry["public_key_hex"].as_str().unwrap(),
            "ed25519 vector {index}: public key differs cross-language"
        );
        assert_eq!(
            key.sign(message.as_bytes()).to_hex(),
            entry["signature_hex"].as_str().unwrap(),
            "ed25519 vector {index}: signature differs cross-language"
        );
    }
}

// C4-2 (pack_root): the receipt's `pack_root` hashes a canonical *mapping* of the
// ContextPack (canonical.rs::context_pack_value) — not a Merkle leaf set — so
// verifying it cross-language means re-implementing that mapping, not just the
// hash. A pure-Python mirror
// (scripts/canonical_jcs_cross_language_check.py::context_pack_value) computed the
// committed pack_root for this minimal pack; here we build the *same* pack through
// the real engine `canonical_context_pack_bytes` and assert the identical bytes +
// root, so the pack canonicalization reproduces byte-for-byte in both languages.
// The minimal pack the pack_root + leaf-extraction cross-language vectors pin: a
// one-cell ContextPack whose plain payload carries no `source_id=`/`source=`/
// `citation=` header (so metadata.source_ref is None) with every optional +
// anomaly absent, keeping the canonical mapping a total, reproducible function.
fn minimal_cross_language_pack() -> ContextPack {
    let payload = b"cortex pack root cross-language fixture body".to_vec();
    let descriptor = CellDescriptor::from_payload_lossy(&payload);
    let retrieved = RetrievedCell {
        cell_id: CellId(1),
        payload: payload.clone(),
        descriptor,
        captured_access_decision: None,
    };
    let metadata = retrieved.metadata();
    assert!(
        metadata.source_ref.is_none(),
        "fixture assumes a payload whose metadata carries no source_ref"
    );
    ContextPack {
        cells: vec![ContextPackCell {
            cell_id: CellId(1),
            payload,
            metadata,
            estimated_tokens: 5,
            citation: None,
            provenance: None,
            explain: None,
            access_decision: None,
        }],
        token_budget_tokens: 128,
        estimated_tokens: 5,
        truncated: false,
        citations_required: false,
        answerability_q16: 0,
        conflict_visibility_q16: 0,
        visible_conflict_count: 0,
        anomalies: vec![],
        grounding_report: None,
    }
}

fn pack_root_vector() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../../fixtures/canonical/pack_root_conformance_vector.v1.json"
    ))
    .expect("pack_root conformance vector parses")
}

#[test]
fn pack_root_matches_cross_language_vector() {
    use crate::canonical::canonical_context_pack_bytes;
    use cortex_crypto::hex_lower;

    let vector = pack_root_vector();
    let pack = minimal_cross_language_pack();

    let canonical = canonical_context_pack_bytes(&pack);
    let derived_root = super::receipt::hash_bytes(
        super::receipt::ACCOUNTABILITY_RECEIPT_PACK_ROOT_DOMAIN,
        &canonical,
    );

    assert_eq!(
        hex_lower(&canonical),
        vector["canonical_pack_bytes_hex"].as_str().unwrap(),
        "Rust canonical pack bytes differ from the committed (Python) bytes"
    );
    assert_eq!(
        derived_root,
        vector["pack_root_blake3"].as_str().unwrap(),
        "Rust pack_root differs from the committed (Python) pack_root"
    );
}

// C4-2 (leaf extraction): the receipt's Merkle roots are already verified
// cross-language over committed leaves; this closes the *inputs -> leaves* half
// for the two scalar-only leaf families (budget, conflict), whose extraction is a
// pure function of the pack (no cell-content hash). The content-hash-bearing
// families (access/provenance/cell_set/verification) need the CellDescriptor /
// verification-report canonicalization mirrored too — spawned task_8f2c7c22.
#[test]
fn pack_leaf_families_match_cross_language_vector() {
    let vector = pack_root_vector();
    let pack = minimal_cross_language_pack();

    assert_eq!(
        serde_json::Value::Array(super::receipt_leaves::budget_leaves(&pack)),
        vector["budget_leaves"],
        "Rust budget_leaves extraction differs from the committed (Python) leaves"
    );
    assert_eq!(
        serde_json::Value::Array(super::receipt_leaves::conflict_leaves(&pack)),
        vector["conflict_leaves"],
        "Rust conflict_leaves extraction differs from the committed (Python) leaves"
    );
}

// C4-2 (access leaf extraction): the access family is self-contained — each
// admitted leaf's evidence_digest is a hash over a small access-evidence object
// (no cell-content hash) and denied leaves carry a precomputed digest. The Python
// mirror (access_leaves) re-derives the leaves for the deterministic
// sample_receipt_inputs decision + denial; here the real engine
// receipt_leaves::access_leaves must produce the identical leaves, including the
// blake3 evidence_digest. This closes a 3rd of the 6 leaf families cross-language.
#[test]
fn access_leaves_match_cross_language_vector() {
    let vector = pack_root_vector();
    let (pack, _retrieved, denials, _input) = sample_receipt_inputs();
    let leaves = super::receipt_leaves::access_leaves(&pack, &denials)
        .expect("sample access decision is allowed + captured");
    assert_eq!(
        serde_json::Value::Array(leaves),
        vector["access_leaves"],
        "Rust access_leaves extraction differs from the committed (Python) leaves"
    );
}

// C4-2 (cell-content leaf extraction): cell_set + provenance leaves carry
// cell_content_hash = blake3(CELL_HASH_DOMAIN || canonical_cell_bytes), where
// canonical_cell_bytes wraps cell_id + payload_hex + descriptor_value(descriptor).
// We pin an *explicit* CellDescriptor (every field known) rather than reproduce
// the descriptor *parser* (upstream of the receipt): the Python mirror hashes the
// same fields, and here the real engine cell_content_hash + the two leaf
// extractions must produce the identical Values. Closes 5 of the 6 leaf families.
#[test]
fn cell_content_leaf_families_match_cross_language_vector() {
    use crate::accountability::retrieved_cell_content_hash;
    use cortex_core::KnowledgeCellType;
    use std::collections::BTreeMap;

    let vector = pack_root_vector();

    let payload = b"cortex cell hash cross-language fixture body".to_vec();
    let descriptor = CellDescriptor {
        scope: "project:xlang".to_owned(),
        status: "ready".to_owned(),
        cell_type: KnowledgeCellType::Fact,
        memory_type: None,
        ttl_seconds: None,
        created_unix_seconds: None,
        source_trust_q16: None,
        source: None,
        citation: Some("fixture:xlang#L1".to_owned()),
        content_hash: Some("d".repeat(64)),
        source_id: None,
        source_url: None,
        document_id: None,
        page: None,
        row: None,
        cell_range: None,
        json_path: None,
        confidence_q16: None,
        parent_id: None,
        valid_from: None,
        valid_to: None,
        session_id: None,
        session_kind: None,
    };
    let retrieved = RetrievedCell {
        cell_id: CellId(1),
        payload: payload.clone(),
        descriptor,
        captured_access_decision: None,
    };
    let mut cell_hashes = BTreeMap::new();
    cell_hashes.insert(CellId(1), retrieved_cell_content_hash(&retrieved));
    assert_eq!(
        cell_hashes[&CellId(1)],
        vector["cell_content_hash"].as_str().unwrap(),
        "Rust cell_content_hash differs from the committed (Python) hash"
    );

    let pack = ContextPack {
        cells: vec![ContextPackCell {
            cell_id: CellId(1),
            payload,
            metadata: retrieved.metadata(),
            estimated_tokens: 5,
            citation: Some("fixture:xlang#L1".to_owned()),
            provenance: Some(ContextSpanProvenance {
                source_cell_id: CellId(1),
                source_byte_start: 0,
                source_byte_end: 12,
                source_line_start: 1,
                source_line_end: 1,
                source_ref: None,
            }),
            explain: None,
            access_decision: None,
        }],
        token_budget_tokens: 128,
        estimated_tokens: 5,
        truncated: false,
        citations_required: false,
        answerability_q16: 0,
        conflict_visibility_q16: 0,
        visible_conflict_count: 0,
        anomalies: vec![],
        grounding_report: None,
    };

    assert_eq!(
        serde_json::Value::Array(super::receipt_leaves::cell_set_leaves(&pack, &cell_hashes)),
        vector["cell_set_leaves"],
        "Rust cell_set_leaves extraction differs from the committed (Python) leaves"
    );
    assert_eq!(
        serde_json::Value::Array(super::receipt_leaves::provenance_leaves(
            &pack,
            &cell_hashes
        )),
        vector["provenance_leaves"],
        "Rust provenance_leaves extraction differs from the committed (Python) leaves"
    );
}

#[test]
fn accountability_receipt_body_roots_are_deterministic_and_schema_aligned() {
    let (pack, retrieved_cells, denials, input) = sample_receipt_inputs();

    let first =
        accountability_receipt_body(&pack, &retrieved_cells, &denials, None, &input).unwrap();
    let second =
        accountability_receipt_body(&pack, &retrieved_cells, &denials, None, &input).unwrap();

    assert_eq!(first, second);
    assert_eq!(first.schema_version, ACCOUNTABILITY_RECEIPT_BODY_SCHEMA);
    assert_eq!(first.access_root.len(), 64);
    assert_eq!(first.provenance_root.len(), 64);
    assert_eq!(first.cell_set_root.len(), 64);
    assert_eq!(first.verification_root.len(), 64);
    assert_eq!(first.budget_commitment.len(), 64);
    assert_eq!(first.conflict_commitment.len(), 64);
    assert_eq!(first.pack_root.len(), 64);
    assert_eq!(first.determinism_hash.len(), 64);
    assert_eq!(first.leaves.access.len(), 2);
    assert_eq!(first.leaves.provenance.len(), 1);
    assert_eq!(first.leaves.cell_set.len(), 1);
    assert_eq!(first.leaves.budget.len(), 2);
    assert_eq!(first.leaves.conflict.len(), 1);
}

#[test]
fn accountability_receipt_body_changes_when_cell_payload_changes() {
    let (pack, mut retrieved_cells, denials, input) = sample_receipt_inputs();
    let original =
        accountability_receipt_body(&pack, &retrieved_cells, &denials, None, &input).unwrap();

    retrieved_cells[0].payload.push(b'!');
    let changed =
        accountability_receipt_body(&pack, &retrieved_cells, &denials, None, &input).unwrap();

    assert_ne!(original.cell_set_root, changed.cell_set_root);
    assert_ne!(original.provenance_root, changed.provenance_root);
    assert_eq!(original.access_root, changed.access_root);
    assert_eq!(original.determinism_hash, changed.determinism_hash);
}

#[test]
fn accountability_receipt_body_changes_when_determinism_input_changes() {
    let (pack, retrieved_cells, denials, mut input) = sample_receipt_inputs();
    let original =
        accountability_receipt_body(&pack, &retrieved_cells, &denials, None, &input).unwrap();

    input.query = "RETRIEVE CONTEXT FOR TASK \"different\"".to_owned();
    let changed =
        accountability_receipt_body(&pack, &retrieved_cells, &denials, None, &input).unwrap();

    assert_eq!(original.access_root, changed.access_root);
    assert_eq!(original.pack_root, changed.pack_root);
    assert_ne!(original.determinism_hash, changed.determinism_hash);
}

#[test]
fn accountability_receipt_body_changes_when_frozen_weights_hash_changes() {
    let (pack, retrieved_cells, denials, mut input) = sample_receipt_inputs();
    let original =
        accountability_receipt_body(&pack, &retrieved_cells, &denials, None, &input).unwrap();

    input.frozen_weights_hash = "f".repeat(64);
    let changed =
        accountability_receipt_body(&pack, &retrieved_cells, &denials, None, &input).unwrap();

    assert_eq!(original.access_root, changed.access_root);
    assert_eq!(original.pack_root, changed.pack_root);
    assert_ne!(original.determinism_hash, changed.determinism_hash);
}

#[test]
fn accountability_receipt_body_requires_captured_allowed_access() {
    let (mut pack, retrieved_cells, denials, input) = sample_receipt_inputs();
    pack.cells[0].access_decision.as_mut().unwrap().decision =
        ContextAccessDecisionOutcome::NotRecorded;

    let error =
        accountability_receipt_body(&pack, &retrieved_cells, &denials, None, &input).unwrap_err();

    assert!(matches!(error, EngineError::InvalidOperation));
}

fn sample_receipt_inputs() -> (
    ContextPack,
    Vec<RetrievedCell>,
    CapturedAccessDenialSet,
    AccountabilityDeterminismInput,
) {
    let payload =
        b"scope=project:receipt\nstatus=ready\ncitation=fixture:source#L1\n\nReceipt fact".to_vec();
    let descriptor = CellDescriptor::from_payload_lossy(&payload);
    let retrieved = RetrievedCell {
        cell_id: CellId(1),
        payload: payload.clone(),
        descriptor,
        captured_access_decision: None,
    };
    let metadata = retrieved.metadata();
    let pack = ContextPack {
        cells: vec![ContextPackCell {
            cell_id: CellId(1),
            payload,
            metadata,
            estimated_tokens: 7,
            citation: Some("fixture:source#L1".to_owned()),
            provenance: Some(ContextSpanProvenance {
                source_cell_id: CellId(1),
                source_byte_start: 0,
                source_byte_end: 12,
                source_line_start: 1,
                source_line_end: 1,
                source_ref: None,
            }),
            explain: None,
            access_decision: Some(ContextAccessDecision {
                cell_id: CellId(1),
                decision: ContextAccessDecisionOutcome::Allowed,
                policy: "agent_view_readable_scope".to_owned(),
                policy_version: Some("agent_view_readable_scope.v1".to_owned()),
                reason:
                    "cell candidate survived AQL permission filtering before ContextPack packing"
                        .to_owned(),
                scope: "project:receipt".to_owned(),
                scope_id: 1001,
                agent_id: Some(7),
                agent_view_digest: Some("a".repeat(64)),
            }),
        }],
        token_budget_tokens: 1000,
        estimated_tokens: 7,
        truncated: false,
        citations_required: true,
        answerability_q16: 60_000,
        conflict_visibility_q16: 0,
        visible_conflict_count: 0,
        anomalies: vec![ContextPackAnomaly {
            cell_id: None,
            code: ContextPackAnomalyCode::RetrievalIncomplete,
            message: "retrieval budget was exhausted before scan completion".to_owned(),
            why_excluded: None,
        }],
        grounding_report: None,
    };
    let denials = CapturedAccessDenialSet {
        total_denied: 1,
        truncated: false,
        denials: vec![CapturedAccessDenial {
            candidate: 2,
            cell_id_hash: "b".repeat(64),
            policy: "agent_view_readable_scope".to_owned(),
            policy_version: "agent_view_readable_scope.v1".to_owned(),
            reason: "cell candidate was rejected by AQL agent access filtering before payload materialization"
                .to_owned(),
            agent_id: Some(7),
            agent_view_digest: "a".repeat(64),
            evidence_digest: "c".repeat(64),
        }],
    };
    let input = AccountabilityDeterminismInput {
        query: "RETRIEVE CONTEXT FOR TASK \"receipt\"".to_owned(),
        agent_view_digest: Some("a".repeat(64)),
        context_options_digest: Some("d".repeat(64)),
        bitmap_program_digest: Some("e".repeat(64)),
        frozen_weights_version: "manual-q16.v1".to_owned(),
        frozen_weights_hash: "0".repeat(64),
    };

    (pack, vec![retrieved], denials, input)
}
