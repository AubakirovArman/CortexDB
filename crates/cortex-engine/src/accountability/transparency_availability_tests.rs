use super::{
    build_transparency_availability_evidence, verify_transparency_availability_evidence,
    TransparencyAvailabilityObservation, TransparencyAvailabilityPolicy,
    TRANSPARENCY_AVAILABILITY_EVIDENCE_SCHEMA, TRANSPARENCY_AVAILABILITY_OBSERVATION_SCHEMA,
};

#[test]
fn transparency_availability_accepts_fresh_independent_monitors() {
    let evidence = build_transparency_availability_evidence(
        policy(),
        vec![
            observation(
                "monitor-a",
                "https://monitor-a.example/probe",
                1_090,
                7_200,
                "a",
            ),
            observation(
                "monitor-b",
                "https://monitor-b.example/probe",
                1_085,
                7_500,
                "a",
            ),
        ],
    )
    .unwrap();

    assert_eq!(
        evidence.schema_version,
        TRANSPARENCY_AVAILABILITY_EVIDENCE_SCHEMA
    );
    assert_eq!(evidence.observations.len(), 2);
    assert_eq!(evidence.log_record_count, 3);
    assert_eq!(evidence.log_head_hash, hex64("a"));
    assert_eq!(evidence.merkle_root_hash, hex64("b"));
    assert_eq!(evidence.availability_hash.len(), 64);
    verify_transparency_availability_evidence(&evidence).unwrap();
}

#[test]
fn transparency_availability_rejects_stale_observation() {
    let error = build_transparency_availability_evidence(
        policy(),
        vec![
            observation(
                "monitor-a",
                "https://monitor-a.example/probe",
                1_030,
                7_200,
                "a",
            ),
            observation(
                "monitor-b",
                "https://monitor-b.example/probe",
                1_085,
                7_500,
                "a",
            ),
        ],
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("stale transparency availability observation"));
}

#[test]
fn transparency_availability_rejects_duplicate_monitor_identity() {
    let error = build_transparency_availability_evidence(
        policy(),
        vec![
            observation(
                "monitor-a",
                "https://monitor-a.example/probe",
                1_090,
                7_200,
                "a",
            ),
            observation(
                "monitor-a",
                "https://monitor-b.example/probe",
                1_085,
                7_500,
                "a",
            ),
        ],
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("duplicate transparency monitor id"));
}

#[test]
fn transparency_availability_rejects_low_monitor_uptime() {
    let error = build_transparency_availability_evidence(
        policy(),
        vec![
            observation(
                "monitor-a",
                "https://monitor-a.example/probe",
                1_090,
                100,
                "a",
            ),
            observation(
                "monitor-b",
                "https://monitor-b.example/probe",
                1_085,
                7_500,
                "a",
            ),
        ],
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("transparency monitor uptime below policy"));
}

#[test]
fn transparency_availability_rejects_split_log_heads() {
    let error = build_transparency_availability_evidence(
        policy(),
        vec![
            observation(
                "monitor-a",
                "https://monitor-a.example/probe",
                1_090,
                7_200,
                "a",
            ),
            observation(
                "monitor-b",
                "https://monitor-b.example/probe",
                1_085,
                7_500,
                "c",
            ),
        ],
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("split transparency availability log head"));
}

fn policy() -> TransparencyAvailabilityPolicy {
    TransparencyAvailabilityPolicy {
        service_id: "public-transparency-mainnet".to_owned(),
        service_url: "https://transparency.example/log".to_owned(),
        window_start_unix_seconds: 1_000,
        window_end_unix_seconds: 1_100,
        required_monitor_count: 2,
        required_monitor_uptime_seconds: 3_600,
        max_observation_age_seconds: 60,
    }
}

fn observation(
    monitor_id: &str,
    monitor_url: &str,
    observed_unix_seconds: u64,
    monitor_uptime_seconds: u64,
    log_head_prefix: &str,
) -> TransparencyAvailabilityObservation {
    TransparencyAvailabilityObservation {
        schema_version: TRANSPARENCY_AVAILABILITY_OBSERVATION_SCHEMA.to_owned(),
        monitor_id: monitor_id.to_owned(),
        monitor_url: monitor_url.to_owned(),
        service_url: "https://transparency.example/log".to_owned(),
        observed_unix_seconds,
        response_http_status: 200,
        monitor_uptime_seconds,
        log_record_count: 3,
        log_head_hash: hex64(log_head_prefix),
        merkle_root_hash: hex64("b"),
        availability_status: "available".to_owned(),
    }
}

fn hex64(prefix: &str) -> String {
    let mut value = prefix.repeat(64);
    value.truncate(64);
    value
}
