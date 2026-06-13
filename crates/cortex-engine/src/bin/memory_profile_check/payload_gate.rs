use std::fs;

use serde_json::{json, Value};

pub(super) fn payload_clone_gate_report() -> Value {
    let checks = [
        require_check(
            "crates/cortex-core/src/memtable/mod.rs",
            "pub fn visible_iter",
            "borrowed visible iterator",
        ),
        require_check(
            "crates/cortex-core/src/memtable/mod.rs",
            "pub fn visible_created_after_iter",
            "borrowed delta iterator",
        ),
        require_check(
            "crates/cortex-storage/src/segment.rs",
            "pub struct SegmentCellRef",
            "borrowed segment cell view",
        ),
        forbid_check(
            "crates/cortex-engine/src/checkpoint.rs",
            "self.snapshot_versions()",
            "checkpoint snapshot clone path",
        ),
        forbid_check(
            "crates/cortex-engine/src/verification.rs",
            "self.snapshot_versions()",
            "VERIFY FACT full clone scan",
        ),
        forbid_check(
            "crates/cortex-engine/src/verification.rs",
            "bind_aql_cached",
            "VERIFY FACT retrieval-index bind path",
        ),
        forbid_check(
            "crates/cortex-engine/src/verification/graph.rs",
            "conflicts_for_fact",
            "VERIFY graph enrichment full conflict-index scan",
        ),
    ];
    let checks = checks.into_iter().collect::<Vec<_>>();
    let passed = checks
        .iter()
        .all(|check| check["ok"].as_bool().unwrap_or(false));
    json!({
        "passed": passed,
        "method": "static_source_gate",
        "checks": checks,
    })
}

fn require_check(path: &str, needle: &str, label: &str) -> Value {
    source_check(path, needle, label, true)
}

fn forbid_check(path: &str, needle: &str, label: &str) -> Value {
    source_check(path, needle, label, false)
}

fn source_check(path: &str, needle: &str, label: &str, require: bool) -> Value {
    let text = fs::read_to_string(path).unwrap_or_default();
    let contains = text.contains(needle);
    let ok = if require { contains } else { !contains };
    json!({
        "ok": ok,
        "path": path,
        "label": label,
        "kind": if require { "require" } else { "forbid" },
        "needle": needle,
    })
}

pub(super) fn clone_gate_errors(report: &Value) -> Vec<String> {
    report["checks"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|check| !check["ok"].as_bool().unwrap_or(false))
        .filter_map(|check| check["label"].as_str())
        .map(|label| format!("payload clone gate failed: {label}"))
        .collect()
}
