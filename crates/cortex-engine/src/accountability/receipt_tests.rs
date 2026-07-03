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
