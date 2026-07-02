use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;

use super::{
    append_transparency_log_record, read_transparency_log_records,
    transparency_witness_record_hash, verify_transparency_witness_record, witness_transparency_log,
    TransparencyWitnessSigningKey, TRANSPARENCY_LOG_ZERO_HASH, TRANSPARENCY_WITNESS_RECORD_SCHEMA,
};

const WITNESS_SEED: &str = "1f1e1d1c1b1a191817161514131211100f0e0d0c0b0a09080706050403020100";

#[test]
fn transparency_log_appends_pack_root_chain() {
    let path = unique_path("chain");
    let first = append_transparency_log_record(
        &path,
        &receipt("pack-root-a", "determinism-a", "signature-a"),
    )
    .unwrap();
    let second = append_transparency_log_record(
        &path,
        &receipt("pack-root-a", "determinism-a", "signature-b"),
    )
    .unwrap();

    let records = read_transparency_log_records(&path).unwrap();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0], first);
    assert_eq!(records[1], second);
    assert_eq!(records[0].sequence, 0);
    assert_eq!(records[0].previous_record_hash, TRANSPARENCY_LOG_ZERO_HASH);
    assert_eq!(records[1].sequence, 1);
    assert_eq!(records[1].previous_record_hash, records[0].record_hash);
    assert_eq!(records[0].pack_root, "pack-root-a");
    assert_eq!(records[1].pack_root, "pack-root-a");
}

#[test]
fn transparency_log_rejects_equivocation_for_same_determinism_hash() {
    let path = unique_path("equivocation");
    append_transparency_log_record(
        &path,
        &receipt("pack-root-a", "determinism-a", "signature-a"),
    )
    .unwrap();

    let error = append_transparency_log_record(
        &path,
        &receipt("pack-root-b", "determinism-a", "signature-b"),
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("equivocation"));
    assert_eq!(read_transparency_log_records(&path).unwrap().len(), 1);
}

#[test]
fn transparency_log_detects_record_tampering() {
    let path = unique_path("tamper");
    append_transparency_log_record(
        &path,
        &receipt("pack-root-a", "determinism-a", "signature-a"),
    )
    .unwrap();
    let tampered = fs::read_to_string(&path)
        .unwrap()
        .replace("pack-root-a", "pack-root-x");
    fs::write(&path, tampered).unwrap();

    let error = read_transparency_log_records(&path)
        .unwrap_err()
        .to_string();
    assert!(error.contains("record hash mismatch"));
}

#[test]
fn transparency_witness_signs_log_head_and_verifies() {
    let path = unique_path("witness");
    let first_pack_root = "a".repeat(64);
    let second_pack_root = "b".repeat(64);
    append_transparency_log_record(
        &path,
        &receipt(&first_pack_root, &"c".repeat(64), &"d".repeat(128)),
    )
    .unwrap();
    append_transparency_log_record(
        &path,
        &receipt(&second_pack_root, &"e".repeat(64), &"f".repeat(128)),
    )
    .unwrap();
    let records = read_transparency_log_records(&path).unwrap();
    let witness_key =
        TransparencyWitnessSigningKey::from_seed_hex("witness-key-a", WITNESS_SEED).unwrap();

    let witness = witness_transparency_log(&path, "mirror-a", 1_800_000_001, &witness_key).unwrap();

    assert_eq!(witness.schema_version, TRANSPARENCY_WITNESS_RECORD_SCHEMA);
    assert_eq!(witness.witnessed_record_count, 2);
    assert_eq!(witness.first_sequence, 0);
    assert_eq!(witness.last_sequence, 1);
    assert_eq!(witness.first_record_hash, records[0].record_hash);
    assert_eq!(witness.log_head_hash, records[1].record_hash);
    assert_eq!(witness.first_pack_root, first_pack_root);
    assert_eq!(witness.last_pack_root, second_pack_root);
    assert_eq!(witness.witness_key_id, "witness-key-a");
    verify_transparency_witness_record(&witness).unwrap();
}

#[test]
fn transparency_witness_detects_tampered_head_after_hash_recompute() {
    let path = unique_path("witness-tamper");
    append_transparency_log_record(
        &path,
        &receipt(&"a".repeat(64), &"c".repeat(64), &"d".repeat(128)),
    )
    .unwrap();
    let witness_key =
        TransparencyWitnessSigningKey::from_seed_hex("witness-key-a", WITNESS_SEED).unwrap();
    let mut witness =
        witness_transparency_log(&path, "mirror-a", 1_800_000_001, &witness_key).unwrap();

    witness.log_head_hash = "f".repeat(64);
    witness.witness_record_hash = transparency_witness_record_hash(&witness);

    let error = verify_transparency_witness_record(&witness)
        .unwrap_err()
        .to_string();
    assert!(error.contains("signature mismatch"));
}

fn receipt(pack_root: &str, determinism_hash: &str, signature_hex: &str) -> serde_json::Value {
    json!({
        "schema_version": "accountability_receipt.v1",
        "header": {
            "schema_version": "accountability_receipt.v1",
            "hash_alg": "blake3-256",
            "sig_alg": "ed25519",
            "db_instance_id": "local:test",
            "key_id": "receipt-key-test",
            "created_unix_seconds": 1_777_777_777_u64,
            "access_root": "access-root",
            "provenance_root": "provenance-root",
            "cell_set_root": "cell-set-root",
            "verification_root": "verification-root",
            "budget_commitment": "budget-root",
            "conflict_commitment": "conflict-root",
            "pack_root": pack_root,
            "determinism_hash": determinism_hash,
            "signature": {
                "key_id": "receipt-key-test",
                "sig_alg": "ed25519",
                "public_key_hex": "public-key",
                "signature_hex": signature_hex,
            },
        },
        "leaves": {
            "access": [],
            "provenance": [],
            "cell_set": [],
            "verification": [],
            "budget": [],
            "conflict": [],
        },
    })
}

fn unique_path(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "cortexdb-transparency-{label}-{}-{nanos}.jsonl",
        std::process::id()
    ))
}
