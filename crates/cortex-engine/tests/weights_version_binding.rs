use std::fs;
use std::path::PathBuf;

use cortex_engine::determinism_hash::{
    determinism_hash, frozen_ranking_weights_artifact_hash_for_bytes,
    frozen_ranking_weights_identity, DeterminismHashInput,
};
use serde_json::{json, Value};

const FIXTURE_BYTES: &[u8] = include_bytes!("../fixtures/ranking_frozen_weights_v1.json");

#[test]
fn determinism_hash_changes_when_frozen_weight_changes_and_restores_on_revert() {
    let identity = frozen_ranking_weights_identity();
    let original_hash = frozen_ranking_weights_artifact_hash_for_bytes(FIXTURE_BYTES);
    assert_eq!(identity.artifact_hash, original_hash);

    let mutated_bytes = mutate_one_q16_weight(FIXTURE_BYTES);
    let mutated_hash = frozen_ranking_weights_artifact_hash_for_bytes(&mutated_bytes);
    let original_determinism_hash = hash_for(&identity.version, &original_hash);
    let mutated_determinism_hash = hash_for(&identity.version, &mutated_hash);
    let reverted_hash = frozen_ranking_weights_artifact_hash_for_bytes(FIXTURE_BYTES);
    let reverted_determinism_hash = hash_for(&identity.version, &reverted_hash);
    let passed = original_hash != mutated_hash
        && original_determinism_hash != mutated_determinism_hash
        && original_hash == reverted_hash
        && original_determinism_hash == reverted_determinism_hash;

    write_report(json!({
        "schema_version": "cortexdb.weights_version_binding.report.v1",
        "status": if passed { "passed" } else { "failed" },
        "frozen_weights_version": identity.version,
        "original_artifact_hash": original_hash,
        "mutated_artifact_hash": mutated_hash,
        "reverted_artifact_hash": reverted_hash,
        "original_determinism_hash": original_determinism_hash,
        "mutated_determinism_hash": mutated_determinism_hash,
        "reverted_determinism_hash": reverted_determinism_hash,
        "mutated_path": "/calibration/basic/0",
    }));

    assert!(passed, "determinism_hash must bind frozen weights bytes");
}

fn hash_for<'a>(version: &'a str, artifact_hash: &'a str) -> String {
    determinism_hash(&DeterminismHashInput {
        query: "RETRIEVE CONTEXT FOR TASK \"release readiness\"",
        agent_view_digest: Some("agent-view-digest-fixture"),
        context_options_digest: Some("context-options-digest-fixture"),
        bitmap_program_digest: Some("bitmap-program-digest-fixture"),
        frozen_weights_version: version,
        frozen_weights_artifact_hash: artifact_hash,
        serving_epoch: None,
        embedding_ref: None,
    })
}

fn mutate_one_q16_weight(bytes: &[u8]) -> Vec<u8> {
    let mut artifact: Value = serde_json::from_slice(bytes).expect("fixture JSON must decode");
    let weight = artifact
        .pointer_mut("/calibration/basic/0")
        .expect("fixture must expose basic lexical weight");
    let next = weight
        .as_u64()
        .expect("basic lexical weight must be an integer")
        .saturating_add(1);
    *weight = json!(next);
    let mut encoded = serde_json::to_vec_pretty(&artifact).expect("mutated fixture must encode");
    encoded.push(b'\n');
    encoded
}

fn write_report(report: Value) {
    let Some(path) = std::env::var_os("CORTEX_WEIGHTS_VERSION_BINDING_REPORT") else {
        return;
    };
    let path = PathBuf::from(path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create weights-version-binding report parent");
    }
    fs::write(
        path,
        serde_json::to_string_pretty(&report).expect("serialize weights-version-binding report")
            + "\n",
    )
    .expect("write weights-version-binding report");
}
