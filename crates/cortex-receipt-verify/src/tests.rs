use cortex_crypto::ReceiptSigningKey;
use serde_json::{json, Value};

use crate::model::{
    AdmittedCellInput, PublicKeyInput, Receipt, ReceiptHeader, ReceiptLeaves, ReceiptSignature,
    VerifyInput,
};
use crate::receipt_hash::{
    canonical_header_bytes, hash_bytes, hash_value, merkle_root, ACCESS_ROOT_DOMAIN,
    BUDGET_COMMITMENT_DOMAIN, CELL_SET_ROOT_DOMAIN, CONFLICT_COMMITMENT_DOMAIN, DETERMINISM_DOMAIN,
    HASH_ALG, PACK_ROOT_DOMAIN, PROVENANCE_ROOT_DOMAIN, RECEIPT_SCHEMA, SIG_ALG,
    VERIFICATION_ROOT_DOMAIN,
};
use crate::verifier::{verify_input, VerifyError};

const FIXTURE_SEED: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
const FIXTURE_KEY_ID: &str = "receipt-key-ar7-fixture";
const FIXTURE_AUDIT_CHAIN_HEAD: &str =
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

#[test]
fn generated_fixture_verifies_and_rejects_signature_tamper() {
    let input = fixture_input();
    verify_input(&input).unwrap();

    let mut tampered = input.clone();
    tampered
        .receipt
        .header
        .signature
        .signature_hex
        .replace_range(0..2, "ff");
    assert_eq!(
        verify_input(&tampered).unwrap_err(),
        VerifyError::InvalidSignature
    );

    if std::env::var_os("CORTEXDB_PRINT_RECEIPT_VERIFY_FIXTURE").is_some() {
        println!("{}", serde_json::to_string_pretty(&input).unwrap());
    }
}

fn fixture_input() -> VerifyInput {
    let key = ReceiptSigningKey::from_seed_hex(FIXTURE_KEY_ID, FIXTURE_SEED).unwrap();
    let public_key = key.public_key();
    let pack = json!({
        "schema_version": "context_pack.canonical.v1",
        "token_budget_tokens": 1000,
        "estimated_tokens": 42,
        "truncated": false,
        "citations_required": true,
        "answerability_q16": 65535,
        "conflict_visibility_q16": 32768,
        "visible_conflict_count": 1,
        "cells": [
            {
                "cell_id": 1,
                "estimated_tokens": 42,
                "citation": "fixture:source#L1"
            }
        ],
        "anomalies": []
    });
    let determinism_input = json!({
        "schema_version": "cortexdb.determinism_hash.input.v1",
        "query": "RETRIEVE CONTEXT WHERE scope = fixture",
        "agent_view_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "context_options_digest": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "bitmap_program_digest": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
        "frozen_weights": {
            "version": "fixture.weights.v1",
            "artifact_hash": "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
        }
    });
    let cell_hash = "dededededededededededededededededededededededededededededededede";
    let access_evidence = json!({
        "cell_id": 1,
        "decision": "allowed",
        "policy": "agent_view_readable_scope",
        "policy_version": "agent_view_readable_scope.v1",
        "reason": "fixture admitted cell",
        "scope_id": 1001,
        "agent_id": 7,
        "agent_view_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    });
    let leaves = ReceiptLeaves {
        access: vec![json!({
            "leaf_type": "admitted_cell",
            "cell_id": 1,
            "candidate": Value::Null,
            "cell_id_hash": Value::Null,
            "decision": "allowed",
            "policy": "agent_view_readable_scope",
            "policy_version": "agent_view_readable_scope.v1",
            "reason": "fixture admitted cell",
            "scope_id": 1001,
            "agent_id": 7,
            "agent_view_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "evidence_digest": hash_value(ACCESS_ROOT_DOMAIN, &access_evidence),
        })],
        provenance: vec![json!({
            "cell_id": 1,
            "cell_content_hash": cell_hash,
            "source_cell_id": 1,
            "source_byte_start": 0,
            "source_byte_end": 12,
            "source_line_start": 1,
            "source_line_end": 1,
            "source_ref": "fixture:source",
            "citation": "fixture:source#L1",
        })],
        cell_set: vec![json!({
            "cell_id": 1,
            "cell_content_hash": cell_hash,
        })],
        verification: vec![json!({
            "leaf_type": "verification_evidence",
            "cell_id": 1,
            "status": "supported",
            "confidence_q16": 65535,
            "evidence_digest": hash_value(
                VERIFICATION_ROOT_DOMAIN,
                &json!({"cell_id": 1, "match_kind": "exact", "match_score_q16": 65535})
            ),
        })],
        budget: vec![
            json!({
                "token_budget_tokens": 1000,
                "estimated_tokens": 42,
                "truncated": false,
                "cell_id": Value::Null,
                "cell_estimated_tokens": Value::Null,
            }),
            json!({
                "token_budget_tokens": 1000,
                "estimated_tokens": 42,
                "truncated": false,
                "cell_id": 1,
                "cell_estimated_tokens": 42,
            }),
        ],
        conflict: vec![json!({
            "conflict_visibility_q16": 32768,
            "visible_conflict_count": 1,
            "anomalies": ["retrieval_incomplete", "visible_conflict"],
        })],
    };

    let mut header = ReceiptHeader {
        schema_version: RECEIPT_SCHEMA.to_owned(),
        hash_alg: HASH_ALG.to_owned(),
        sig_alg: SIG_ALG.to_owned(),
        db_instance_id: "fixture-db".to_owned(),
        key_id: FIXTURE_KEY_ID.to_owned(),
        created_unix_seconds: 0,
        access_root: merkle_root(ACCESS_ROOT_DOMAIN, &leaves.access),
        provenance_root: merkle_root(PROVENANCE_ROOT_DOMAIN, &leaves.provenance),
        cell_set_root: merkle_root(CELL_SET_ROOT_DOMAIN, &leaves.cell_set),
        verification_root: merkle_root(VERIFICATION_ROOT_DOMAIN, &leaves.verification),
        budget_commitment: merkle_root(BUDGET_COMMITMENT_DOMAIN, &leaves.budget),
        conflict_commitment: merkle_root(CONFLICT_COMMITMENT_DOMAIN, &leaves.conflict),
        pack_root: hash_bytes(
            PACK_ROOT_DOMAIN,
            &crate::canonical::canonical_json_bytes(&pack),
        ),
        determinism_hash: hash_value(DETERMINISM_DOMAIN, &determinism_input),
        audit_chain_head: FIXTURE_AUDIT_CHAIN_HEAD.to_owned(),
        signature: ReceiptSignature {
            key_id: FIXTURE_KEY_ID.to_owned(),
            sig_alg: SIG_ALG.to_owned(),
            public_key_hex: public_key.to_hex(),
            signature_hex: String::new(),
        },
    };
    header.signature.signature_hex = key.sign(&canonical_header_bytes(&header)).to_hex();

    VerifyInput {
        schema_version: "cortexdb.accountability_receipt_verify_input.v1".to_owned(),
        pack,
        determinism_input,
        receipt: Receipt {
            schema_version: RECEIPT_SCHEMA.to_owned(),
            header,
            leaves,
        },
        public_key: PublicKeyInput {
            key_id: FIXTURE_KEY_ID.to_owned(),
            public_key_hex: public_key.to_hex(),
        },
        admitted_cells: vec![AdmittedCellInput {
            cell_id: 1,
            cell_content_hash: cell_hash.to_owned(),
            raw_content_hex: Some("666978747572652074657874".to_owned()),
        }],
    }
}
