use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;

use super::{
    append_transparency_log_record, transparency_witness_quorum_hash,
    verify_transparency_witness_quorum, witness_transparency_log, TransparencyWitnessSigningKey,
    TRANSPARENCY_WITNESS_QUORUM_SCHEMA,
};

const WITNESS_SEED_A: &str = "1f1e1d1c1b1a191817161514131211100f0e0d0c0b0a09080706050403020100";
const WITNESS_SEED_B: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";

#[test]
fn transparency_witness_quorum_accepts_independent_log_head_witnesses() {
    let path = transparency_log("quorum-ok", "a", "b");
    let witness_a = witness_for(&path, "mirror-a", "witness-key-a", WITNESS_SEED_A, 1);
    let witness_b = witness_for(&path, "mirror-b", "witness-key-b", WITNESS_SEED_B, 2);

    let evidence =
        verify_transparency_witness_quorum(&[witness_b.clone(), witness_a.clone()], "alpha", 2)
            .unwrap();
    let reordered =
        verify_transparency_witness_quorum(&[witness_a, witness_b], "alpha", 2).unwrap();

    assert_eq!(evidence.schema_version, TRANSPARENCY_WITNESS_QUORUM_SCHEMA);
    assert_eq!(evidence.min_witnesses, 2);
    assert_eq!(evidence.witness_count, 2);
    assert_eq!(evidence.witness_ids, vec!["mirror-a", "mirror-b"]);
    assert_eq!(
        evidence.witness_key_ids,
        vec!["witness-key-a", "witness-key-b"]
    );
    assert_eq!(evidence.witnessed_record_count, 2);
    assert_eq!(evidence.first_sequence, 0);
    assert_eq!(evidence.last_sequence, 1);
    assert_eq!(evidence.quorum_hash, reordered.quorum_hash);
    assert_eq!(
        evidence.quorum_hash,
        transparency_witness_quorum_hash(&evidence)
    );
}

#[test]
fn transparency_witness_quorum_rejects_duplicate_public_key() {
    let path = transparency_log("quorum-duplicate", "a", "b");
    let witness_a = witness_for(&path, "mirror-a", "witness-key-a", WITNESS_SEED_A, 1);
    let witness_b = witness_for(&path, "mirror-b", "witness-key-b", WITNESS_SEED_A, 2);

    let error = verify_transparency_witness_quorum(&[witness_a, witness_b], "alpha", 2)
        .unwrap_err()
        .to_string();

    assert!(error.contains("duplicate witness_public_key_hex"));
}

#[test]
fn transparency_witness_quorum_rejects_split_log_heads() {
    let path_a = transparency_log("quorum-split-a", "a", "b");
    let path_b = transparency_log("quorum-split-b", "a", "c");
    let witness_a = witness_for(&path_a, "mirror-a", "witness-key-a", WITNESS_SEED_A, 1);
    let witness_b = witness_for(&path_b, "mirror-b", "witness-key-b", WITNESS_SEED_B, 2);

    let error = verify_transparency_witness_quorum(&[witness_a, witness_b], "alpha", 2)
        .unwrap_err()
        .to_string();

    assert!(error.contains("mismatched witnessed log heads"));
}

fn transparency_log(label: &str, first_pack_prefix: &str, second_pack_prefix: &str) -> PathBuf {
    let path = unique_path(label);
    append_transparency_log_record(
        &path,
        &receipt(
            &hex64(first_pack_prefix),
            &hex64("d"),
            &format!("{}{}", "1".repeat(64), "2".repeat(64)),
        ),
    )
    .unwrap();
    append_transparency_log_record(
        &path,
        &receipt(
            &hex64(second_pack_prefix),
            &hex64("e"),
            &format!("{}{}", "3".repeat(64), "4".repeat(64)),
        ),
    )
    .unwrap();
    path
}

fn witness_for(
    path: &Path,
    witness_id: &str,
    key_id: &str,
    seed: &str,
    timestamp_offset: u64,
) -> super::TransparencyWitnessRecord {
    let key = TransparencyWitnessSigningKey::from_seed_hex(key_id, seed).unwrap();
    witness_transparency_log(path, witness_id, 1_800_000_000 + timestamp_offset, &key).unwrap()
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

fn hex64(prefix: &str) -> String {
    prefix.repeat(64)
}

fn unique_path(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "cortexdb-transparency-quorum-{label}-{}-{nanos}.jsonl",
        std::process::id()
    ));
    let _ = fs::remove_file(&path);
    path
}
