use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use serde_json::json;

const KEYS: &[&str] = &[
    "before_context_pack",
    "before_context_determinism_hash",
    "before_verify_context_pack",
    "before_verify_report",
    "before_verify_determinism_hash",
    "after_context_pack",
    "after_context_determinism_hash",
    "after_verify_context_pack",
    "after_verify_report",
    "after_verify_determinism_hash",
];

#[test]
fn pack_and_verify_canonical_bytes_and_determinism_hash_are_cross_process_stable() {
    let first = run_fixture();
    let second = run_fixture();
    let same_process_stable = before_after_equal(&first);
    let cross_process_stable = first == second;
    let passed = same_process_stable && cross_process_stable;

    write_report(json!({
        "schema_version": "cortexdb.pack_determinism_hash.report.v1",
        "status": if passed { "passed" } else { "failed" },
        "same_process_checkpoint_reopen_stable": same_process_stable,
        "cross_process_stable": cross_process_stable,
        "first": first,
        "second": second,
    }));

    assert!(
        same_process_stable,
        "before/after checkpoint hashes must match"
    );
    assert!(
        cross_process_stable,
        "independent process hashes must match"
    );
}

fn before_after_equal(values: &BTreeMap<String, String>) -> bool {
    pairs_match(values, "context_pack")
        && pairs_match(values, "context_determinism_hash")
        && pairs_match(values, "verify_context_pack")
        && pairs_match(values, "verify_report")
        && pairs_match(values, "verify_determinism_hash")
}

fn pairs_match(values: &BTreeMap<String, String>, suffix: &str) -> bool {
    values.get(&format!("before_{suffix}")) == values.get(&format!("after_{suffix}"))
}

fn run_fixture() -> BTreeMap<String, String> {
    let output = Command::new(env!("CARGO_BIN_EXE_pack_determinism_hash_fixture"))
        .output()
        .expect("run pack determinism fixture");
    assert!(
        output.status.success(),
        "fixture failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("fixture output must be utf8");
    let mut values = BTreeMap::new();
    for line in stdout.lines() {
        if let Some((key, value)) = line.split_once('=') {
            values.insert(key.to_owned(), value.to_owned());
        }
    }
    for key in KEYS {
        assert_nonempty_hex(&values, key);
    }
    values
}

fn assert_nonempty_hex(values: &BTreeMap<String, String>, key: &str) {
    let value = values.get(key).unwrap_or_else(|| panic!("missing {key}"));
    assert!(!value.is_empty(), "{key} must not be empty");
    assert!(
        value.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "{key} must be hex"
    );
}

fn write_report(report: serde_json::Value) {
    let Some(path) = std::env::var_os("CORTEX_PACK_DETERMINISM_HASH_REPORT") else {
        return;
    };
    let path = PathBuf::from(path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create pack-determinism report parent");
    }
    fs::write(
        path,
        serde_json::to_string_pretty(&report).expect("serialize pack-determinism report") + "\n",
    )
    .expect("write pack-determinism report");
}
