use std::collections::BTreeMap;
use std::process::Command;

#[test]
fn canonical_bytes_match_across_processes() {
    let first = run_fixture();
    let second = run_fixture();

    assert_eq!(first, second);
    assert_nonempty_hex(&first, "context_pack");
    assert_nonempty_hex(&first, "verification_report");
}

fn run_fixture() -> BTreeMap<String, String> {
    let output = Command::new(env!("CARGO_BIN_EXE_accountability_canonical_fixture"))
        .output()
        .expect("run accountability canonical fixture");
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
