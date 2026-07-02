use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;

use super::{
    append_transparency_log_record, build_transparency_inclusion_proof,
    read_transparency_log_records, transparency_inclusion_root_hash,
    verify_transparency_inclusion_proof, TRANSPARENCY_INCLUSION_PROOF_SCHEMA,
};

#[test]
fn transparency_inclusion_proof_accepts_middle_record() {
    let records = transparency_log("inclusion-ok", &["a", "b", "c"]);

    let proof = build_transparency_inclusion_proof(&records, 1).unwrap();

    assert_eq!(proof.schema_version, TRANSPARENCY_INCLUSION_PROOF_SCHEMA);
    assert_eq!(proof.sequence, 1);
    assert_eq!(proof.record_hash, records[1].record_hash);
    assert_eq!(proof.log_record_count, 3);
    assert_eq!(proof.log_head_hash, records[2].record_hash);
    assert_eq!(proof.proof_path.len(), 2);
    assert_eq!(
        proof.merkle_root_hash,
        transparency_inclusion_root_hash(&records).unwrap()
    );
    verify_transparency_inclusion_proof(&proof).unwrap();
}

#[test]
fn transparency_inclusion_proof_rejects_wrong_record_hash() {
    let records = transparency_log("inclusion-wrong-record", &["a", "b", "c"]);
    let mut proof = build_transparency_inclusion_proof(&records, 1).unwrap();
    proof.record_hash = hex64("f");

    let error = verify_transparency_inclusion_proof(&proof)
        .unwrap_err()
        .to_string();

    assert!(error.contains("transparency inclusion leaf hash mismatch"));
}

#[test]
fn transparency_inclusion_proof_rejects_wrong_path_hash() {
    let records = transparency_log("inclusion-wrong-path", &["a", "b", "c"]);
    let mut proof = build_transparency_inclusion_proof(&records, 1).unwrap();
    proof.proof_path[0].hash = hex64("9");

    let error = verify_transparency_inclusion_proof(&proof)
        .unwrap_err()
        .to_string();

    assert!(error.contains("transparency inclusion root hash mismatch"));
}

fn transparency_log(label: &str, pack_prefixes: &[&str]) -> Vec<super::TransparencyLogRecord> {
    let path = unique_path(label);
    for (index, prefix) in pack_prefixes.iter().enumerate() {
        append_transparency_log_record(
            &path,
            &receipt(
                &hex64(prefix),
                &hex64(&(index + 10).to_string()),
                &format!("{}{}", index + 1, "0".repeat(127)),
            ),
        )
        .unwrap();
    }
    read_transparency_log_records(&path).unwrap()
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
    let path = std::env::temp_dir().join(format!(
        "cortexdb-transparency-inclusion-{label}-{}-{nanos}.jsonl",
        std::process::id()
    ));
    let _ = fs::remove_file(&path);
    path
}

fn hex64(prefix: &str) -> String {
    let mut value = prefix.repeat(64);
    value.truncate(64);
    value
}
