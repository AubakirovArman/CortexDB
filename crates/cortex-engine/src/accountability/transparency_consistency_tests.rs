use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;

use super::{
    append_transparency_log_record, build_transparency_consistency_evidence,
    read_transparency_log_records, verify_transparency_consistency_evidence,
    TRANSPARENCY_CONSISTENCY_SCHEMA,
};

#[test]
fn transparency_consistency_accepts_append_only_snapshot() {
    let old_records = transparency_log("consistency-old", &["a", "b"]);
    let new_records = transparency_log("consistency-new", &["a", "b", "c"]);

    let evidence =
        build_transparency_consistency_evidence(&old_records, &new_records, "monitor-a").unwrap();

    assert_eq!(evidence.schema_version, TRANSPARENCY_CONSISTENCY_SCHEMA);
    assert_eq!(evidence.monitor_id, "monitor-a");
    assert_eq!(evidence.old_log_record_count, 2);
    assert_eq!(evidence.new_log_record_count, 3);
    assert_eq!(evidence.old_log_head_hash, old_records[1].record_hash);
    assert_eq!(evidence.new_log_head_hash, new_records[2].record_hash);
    assert_eq!(
        evidence.old_record_hashes.as_slice(),
        &evidence.new_record_hashes[..2]
    );
    verify_transparency_consistency_evidence(&evidence).unwrap();
}

#[test]
fn transparency_consistency_rejects_divergent_prefix() {
    let old_records = transparency_log("consistency-divergent-old", &["a", "b"]);
    let new_records = transparency_log("consistency-divergent-new", &["a", "x", "c"]);

    let error = build_transparency_consistency_evidence(&old_records, &new_records, "monitor-a")
        .unwrap_err()
        .to_string();

    assert!(error.contains("transparency consistency divergent prefix"));
}

#[test]
fn transparency_consistency_rejects_truncated_snapshot() {
    let old_records = transparency_log("consistency-truncated-old", &["a", "b"]);
    let new_records = transparency_log("consistency-truncated-new", &["a"]);

    let error = build_transparency_consistency_evidence(&old_records, &new_records, "monitor-a")
        .unwrap_err()
        .to_string();

    assert!(error.contains("transparency consistency new snapshot is shorter"));
}

fn transparency_log(label: &str, pack_prefixes: &[&str]) -> Vec<super::TransparencyLogRecord> {
    let path = unique_path(label);
    for (index, prefix) in pack_prefixes.iter().enumerate() {
        append_transparency_log_record(
            &path,
            &receipt(
                &hex64(prefix),
                &hex64(&(index + 20).to_string()),
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
        "cortexdb-transparency-consistency-{label}-{}-{nanos}.jsonl",
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
