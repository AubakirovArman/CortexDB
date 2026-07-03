use cortex_crypto::{ReceiptKeyRing, ReceiptSigningKey};
use serde_json::Value;

use super::*;
use crate::error::EngineError;

const TEST_SEED_A: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
const TEST_SEED_B: &str = "1f1e1d1c1b1a191817161514131211100f0e0d0c0b0a09080706050403020100";
const TEST_AUDIT_CHAIN_HEAD: &str =
    "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

#[test]
fn accountability_receipt_header_signature_is_deterministic() {
    let body = sample_body();
    let key = ReceiptSigningKey::from_seed_hex("receipt-key-a", TEST_SEED_A).unwrap();

    let first =
        sign_accountability_receipt_header(&body, "fixture-db", 0, TEST_AUDIT_CHAIN_HEAD, &key)
            .unwrap();
    let second =
        sign_accountability_receipt_header(&body, "fixture-db", 0, TEST_AUDIT_CHAIN_HEAD, &key)
            .unwrap();

    assert_eq!(first, second);
    assert_eq!(first.schema_version, ACCOUNTABILITY_RECEIPT_SCHEMA_VERSION);
    assert_eq!(first.hash_alg, ACCOUNTABILITY_RECEIPT_HASH_ALG);
    assert_eq!(first.sig_alg, ACCOUNTABILITY_RECEIPT_SIG_ALG);
    assert_eq!(first.signature.sig_alg, ACCOUNTABILITY_RECEIPT_SIG_ALG);
    assert_eq!(first.signature.signature_hex.len(), 128);
    assert_eq!(first.signature.public_key_hex.len(), 64);
    assert_eq!(first.signature.key_id, "receipt-key-a");
    assert_eq!(first.audit_chain_head, TEST_AUDIT_CHAIN_HEAD);

    let unsigned = canonical_accountability_receipt_header_bytes(&first);
    let unsigned_text = std::str::from_utf8(&unsigned).unwrap();
    assert!(unsigned_text.contains("cortexdb.accountability_receipt.sign.v1"));
    assert!(!unsigned_text.contains("signature_hex"));

    let signed_value = accountability_receipt_header_value(&first);
    assert_eq!(
        signed_value["signature"]["signature_hex"],
        first.signature.signature_hex
    );
}

#[test]
fn accountability_receipt_header_is_replica_invariant_for_same_committed_inputs() {
    let body = sample_body();
    let key = ReceiptSigningKey::from_seed_hex("receipt-key-a", TEST_SEED_A).unwrap();

    let replica_a =
        sign_accountability_receipt_header(&body, "fixture-db", 42, TEST_AUDIT_CHAIN_HEAD, &key)
            .unwrap();
    let replica_b =
        sign_accountability_receipt_header(&body, "fixture-db", 42, TEST_AUDIT_CHAIN_HEAD, &key)
            .unwrap();

    assert_eq!(replica_a, replica_b);
    assert_eq!(
        canonical_accountability_receipt_header_bytes(&replica_a),
        canonical_accountability_receipt_header_bytes(&replica_b)
    );
    assert_eq!(
        accountability_receipt_header_value(&replica_a),
        accountability_receipt_header_value(&replica_b)
    );
    assert_eq!(replica_a.pack_root, body.pack_root);
    assert_eq!(replica_a.determinism_hash, body.determinism_hash);
    assert_eq!(replica_a.audit_chain_head, TEST_AUDIT_CHAIN_HEAD);
}

#[test]
fn accountability_receipt_header_signature_changes_when_root_changes() {
    let mut body = sample_body();
    let key = ReceiptSigningKey::from_seed_hex("receipt-key-a", TEST_SEED_A).unwrap();
    let original =
        sign_accountability_receipt_header(&body, "fixture-db", 0, TEST_AUDIT_CHAIN_HEAD, &key)
            .unwrap();

    body.pack_root = "f".repeat(64);
    let changed =
        sign_accountability_receipt_header(&body, "fixture-db", 0, TEST_AUDIT_CHAIN_HEAD, &key)
            .unwrap();

    assert_ne!(original.pack_root, changed.pack_root);
    assert_ne!(
        original.signature.signature_hex,
        changed.signature.signature_hex
    );
}

#[test]
fn accountability_receipt_header_changes_when_audit_chain_head_changes() {
    let body = sample_body();
    let key = ReceiptSigningKey::from_seed_hex("receipt-key-a", TEST_SEED_A).unwrap();
    let original =
        sign_accountability_receipt_header(&body, "fixture-db", 0, TEST_AUDIT_CHAIN_HEAD, &key)
            .unwrap();
    let changed =
        sign_accountability_receipt_header(&body, "fixture-db", 0, &"b".repeat(64), &key).unwrap();

    assert_ne!(original.audit_chain_head, changed.audit_chain_head);
    assert_ne!(
        original.signature.signature_hex,
        changed.signature.signature_hex
    );
}

#[test]
fn accountability_receipt_header_verifies_with_trusted_keyring() {
    let body = sample_body();
    let key = ReceiptSigningKey::from_seed_hex("receipt-key-a", TEST_SEED_A).unwrap();
    let header =
        sign_accountability_receipt_header(&body, "fixture-db", 0, TEST_AUDIT_CHAIN_HEAD, &key)
            .unwrap();
    let keyring = ReceiptKeyRing::new(vec![key.public_key()]).unwrap();

    verify_accountability_receipt_header(&header, &keyring).unwrap();
}

#[test]
fn accountability_receipt_header_accepts_external_signer_trait() {
    let body = sample_body();
    let key = ReceiptSigningKey::from_seed_hex("receipt-key-external", TEST_SEED_A).unwrap();
    let signer = DelegatingSigner { key };
    let header = sign_accountability_receipt_header_with_signer(
        &body,
        "fixture-db",
        0,
        TEST_AUDIT_CHAIN_HEAD,
        &signer,
    )
    .unwrap();
    let keyring = ReceiptKeyRing::new(vec![signer.key.public_key()]).unwrap();

    assert_eq!(header.key_id, "receipt-key-external");
    assert_eq!(
        header.signature.public_key_hex,
        signer.key.public_key().to_hex()
    );
    verify_accountability_receipt_header(&header, &keyring).unwrap();
}

#[test]
fn accountability_receipt_header_rejects_external_signer_invalid_signature() {
    let body = sample_body();
    let key = ReceiptSigningKey::from_seed_hex("receipt-key-external", TEST_SEED_A).unwrap();
    let signer = InvalidSignatureSigner { key };
    let error = sign_accountability_receipt_header_with_signer(
        &body,
        "fixture-db",
        0,
        TEST_AUDIT_CHAIN_HEAD,
        &signer,
    )
    .unwrap_err();

    assert!(matches!(error, EngineError::InvalidOperation));
}

#[test]
fn accountability_receipt_header_rejects_rotated_key_id_and_public_key_mismatch() {
    let body = sample_body();
    let key_a = ReceiptSigningKey::from_seed_hex("receipt-key-a", TEST_SEED_A).unwrap();
    let key_b = ReceiptSigningKey::from_seed_hex("receipt-key-b", TEST_SEED_B).unwrap();
    let header =
        sign_accountability_receipt_header(&body, "fixture-db", 0, TEST_AUDIT_CHAIN_HEAD, &key_a)
            .unwrap();
    let keyring_b = ReceiptKeyRing::new(vec![key_b.public_key()]).unwrap();

    let error = verify_accountability_receipt_header(&header, &keyring_b).unwrap_err();
    assert!(matches!(error, EngineError::InvalidOperation));

    let mut mismatched_key_id = header.clone();
    mismatched_key_id.signature.key_id = "receipt-key-b".to_owned();
    let keyring_both = ReceiptKeyRing::new(vec![key_a.public_key(), key_b.public_key()]).unwrap();
    assert!(verify_accountability_receipt_header(&mismatched_key_id, &keyring_both).is_err());

    let mut mismatched_public_key = header;
    mismatched_public_key.signature.public_key_hex = key_b.public_key().to_hex();
    assert!(verify_accountability_receipt_header(&mismatched_public_key, &keyring_both).is_err());
}

struct DelegatingSigner {
    key: ReceiptSigningKey,
}

impl AccountabilityReceiptHeaderSigner for DelegatingSigner {
    fn key_id(&self) -> &str {
        self.key.key_id()
    }

    fn public_key_hex(&self) -> String {
        self.key.public_key().to_hex()
    }

    fn sign_receipt_header(&self, unsigned_header_bytes: &[u8]) -> Result<String, String> {
        Ok(self.key.sign(unsigned_header_bytes).to_hex())
    }
}

struct InvalidSignatureSigner {
    key: ReceiptSigningKey,
}

impl AccountabilityReceiptHeaderSigner for InvalidSignatureSigner {
    fn key_id(&self) -> &str {
        self.key.key_id()
    }

    fn public_key_hex(&self) -> String {
        self.key.public_key().to_hex()
    }

    fn sign_receipt_header(&self, _unsigned_header_bytes: &[u8]) -> Result<String, String> {
        Ok("0".repeat(128))
    }
}

fn sample_body() -> AccountabilityReceiptBody {
    AccountabilityReceiptBody {
        schema_version: ACCOUNTABILITY_RECEIPT_BODY_SCHEMA,
        access_root: "0".repeat(64),
        provenance_root: "1".repeat(64),
        cell_set_root: "2".repeat(64),
        verification_root: "3".repeat(64),
        budget_commitment: "4".repeat(64),
        conflict_commitment: "5".repeat(64),
        pack_root: "6".repeat(64),
        determinism_hash: "7".repeat(64),
        leaves: AccountabilityReceiptLeaves {
            access: Vec::<Value>::new(),
            provenance: Vec::<Value>::new(),
            cell_set: Vec::<Value>::new(),
            verification: Vec::<Value>::new(),
            budget: Vec::<Value>::new(),
            conflict: Vec::<Value>::new(),
        },
    }
}

#[test]
fn receipt_emission_p99_is_within_budget() {
    // C4-1: the receipt emission hot path (canonicalize + Ed25519 sign the
    // header) is measured and frozen under a budget. If emission ever became
    // expensive, operators would be tempted to flag the receipt off — and the
    // moat evaporates. The budget is deliberately generous (never flakes on CI
    // timing noise); the measured p50/p99 are recorded for tracking.
    let body = sample_body();
    let key = ReceiptSigningKey::from_seed_hex("receipt-key-a", TEST_SEED_A).unwrap();
    for _ in 0..64 {
        let signed =
            sign_accountability_receipt_header(&body, "fixture-db", 0, TEST_AUDIT_CHAIN_HEAD, &key);
        std::hint::black_box(&signed);
    }
    const ITERS: usize = 400;
    let mut nanos = Vec::with_capacity(ITERS);
    for _ in 0..ITERS {
        let start = std::time::Instant::now();
        let signed =
            sign_accountability_receipt_header(&body, "fixture-db", 0, TEST_AUDIT_CHAIN_HEAD, &key);
        nanos.push(u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX));
        std::hint::black_box(&signed);
    }
    nanos.sort_unstable();
    let p50 = nanos[ITERS / 2];
    let p99 = nanos[(ITERS * 99 / 100).min(ITERS - 1)];
    const BUDGET_NANOS: u64 = 50_000_000; // 50 ms/emit — catches catastrophic regressions only.

    let report = serde_json::json!({
        "schema_version": "cortexdb.receipt_emission_budget.report.v1",
        "iterations": ITERS,
        "p50_nanos": p50,
        "p99_nanos": p99,
        "budget_nanos": BUDGET_NANOS,
        "within_budget": p99 <= BUDGET_NANOS,
    });
    if let Ok(path) = std::env::var("CORTEX_RECEIPT_EMISSION_BUDGET_REPORT") {
        let path = std::path::PathBuf::from(path);
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&path, serde_json::to_string_pretty(&report).unwrap());
    }
    assert!(
        p99 <= BUDGET_NANOS,
        "receipt emission p99 {p99}ns exceeds budget {BUDGET_NANOS}ns (p50 {p50}ns)"
    );
}
