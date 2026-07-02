use super::{
    build_transparency_slo_evidence, verify_transparency_slo_evidence, TransparencySloPolicy,
    TransparencySloWindow, TRANSPARENCY_SLO_EVIDENCE_SCHEMA, TRANSPARENCY_SLO_WINDOW_SCHEMA,
};

#[test]
fn transparency_slo_accepts_continuous_operational_windows() {
    let evidence = build_transparency_slo_evidence(policy(), windows("a")).unwrap();

    assert_eq!(evidence.schema_version, TRANSPARENCY_SLO_EVIDENCE_SCHEMA);
    assert_eq!(evidence.window_count, 3);
    assert_eq!(evidence.available_window_count, 3);
    assert_eq!(evidence.log_record_count, 12);
    assert_eq!(evidence.log_head_hash, hash("c"));
    verify_transparency_slo_evidence(&evidence).unwrap();
}

#[test]
fn transparency_slo_rejects_gap_between_windows() {
    let mut windows = windows("a");
    windows[1].window_start_unix_seconds = 1200;

    let error = build_transparency_slo_evidence(policy(), windows)
        .expect_err("gap must be rejected")
        .to_string();
    assert!(error.contains("transparency slo window gap"));
}

#[test]
fn transparency_slo_rejects_below_availability_slo() {
    let mut windows = windows("a");
    windows[1].availability_status = "unavailable".to_owned();

    let error = build_transparency_slo_evidence(policy(), windows)
        .expect_err("insufficient availability must be rejected")
        .to_string();
    assert!(error.contains("transparency slo availability target not met"));
}

#[test]
fn transparency_slo_rejects_log_count_regression() {
    let mut windows = windows("a");
    windows[2].log_record_count = 9;

    let error = build_transparency_slo_evidence(policy(), windows)
        .expect_err("log count regression must be rejected")
        .to_string();
    assert!(error.contains("transparency slo log count regressed"));
}

#[test]
fn transparency_slo_rejects_split_root_for_same_log_count() {
    let mut windows = windows("a");
    windows[1].log_record_count = windows[0].log_record_count;
    windows[1].log_head_hash = hash("d");

    let error = build_transparency_slo_evidence(policy(), windows)
        .expect_err("same-count split head must be rejected")
        .to_string();
    assert!(error.contains("split transparency slo log head"));
}

fn policy() -> TransparencySloPolicy {
    TransparencySloPolicy {
        service_id: "cortexdb-public-transparency".to_owned(),
        service_url: "https://transparency.cortexdb.example".to_owned(),
        period_start_unix_seconds: 1000,
        period_end_unix_seconds: 1179,
        required_window_count: 3,
        min_available_window_percentage: 100,
        max_window_gap_seconds: 0,
        required_monitor_count: 3,
        required_gossip_fanout: 2,
    }
}

fn windows(prefix: &str) -> Vec<TransparencySloWindow> {
    vec![
        window("w1", 1000, 1059, 10, &hash(prefix)),
        window("w2", 1060, 1119, 11, &hash("b")),
        window("w3", 1120, 1179, 12, &hash("c")),
    ]
}

fn window(
    id: &str,
    start: u64,
    end: u64,
    log_record_count: u64,
    log_head_hash: &str,
) -> TransparencySloWindow {
    TransparencySloWindow {
        schema_version: TRANSPARENCY_SLO_WINDOW_SCHEMA.to_owned(),
        window_id: id.to_owned(),
        service_url: "https://transparency.cortexdb.example".to_owned(),
        window_start_unix_seconds: start,
        window_end_unix_seconds: end,
        availability_status: "available".to_owned(),
        monitor_count: 3,
        gossip_fanout: 2,
        consistency_status: "append_only".to_owned(),
        log_record_count,
        log_head_hash: log_head_hash.to_owned(),
        merkle_root_hash: log_head_hash.to_owned(),
    }
}

fn hash(prefix: &str) -> String {
    format!("{prefix:0<64}")
}
