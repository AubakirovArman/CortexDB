use cortex_core::CellId;
use cortex_engine::{ContextPack, ContextPackAnomalyCode, ContextPackOptions, RetrievedCell};

#[test]
fn normalized_equal_currency_and_magnitude_values_do_not_flag_conflict() {
    let pack = pack_from_cells(vec![
        retrieved(
            1,
            "Project: Solar\nMetric: budget\nValue: $1.2M\n\nSolar budget is $1.2M.",
        ),
        retrieved(
            2,
            "project=Solar\nmetric=budget\nvalue=1,200,000\ncurrency=USD\n\nSolar budget is 1,200,000 USD.",
        ),
        retrieved(
            3,
            "project=Solar\nmetric=budget\nvalue=1.2 million\n\nSolar budget is 1.2 million.",
        ),
    ]);

    assert_eq!(pack.visible_conflict_count, 0);
    assert_eq!(pack.conflict_visibility_q16, 0);
    assert!(!pack
        .anomalies
        .iter()
        .any(|anomaly| anomaly.code == ContextPackAnomalyCode::VisibleConflict));
}

#[test]
fn true_numeric_conflicts_flag_across_currency_formats() {
    let pack = pack_from_cells(vec![
        retrieved(
            1,
            "project=Solar\nmetric=budget\nvalue=$1.2M\n\nSolar budget is $1.2M.",
        ),
        retrieved(
            2,
            "project=Solar\nmetric=budget\nvalue=1.4 million\ncurrency=USD\n\nSolar budget is 1.4 million USD.",
        ),
    ]);

    assert_eq!(pack.visible_conflict_count, 1);
    assert_eq!(pack.conflict_visibility_q16, 32_767);
    assert!(pack
        .anomalies
        .iter()
        .any(|anomaly| anomaly.code == ContextPackAnomalyCode::VisibleConflict));
}

#[test]
fn compatible_unit_values_normalize_before_conflict_detection() {
    let equal = pack_from_cells(vec![
        retrieved(
            1,
            "project=Launch\nmetric=duration\nvalue=60 min\n\nLaunch duration is 60 min.",
        ),
        retrieved(
            2,
            "project=Launch\nmetric=duration\nvalue=1h\n\nLaunch duration is 1h.",
        ),
    ]);
    assert_eq!(equal.visible_conflict_count, 0);

    let conflicting = pack_from_cells(vec![
        retrieved(
            1,
            "project=Launch\nmetric=duration\nvalue=60 min\n\nLaunch duration is 60 min.",
        ),
        retrieved(
            2,
            "project=Launch\nmetric=duration\nvalue=2 h\n\nLaunch duration is 2 h.",
        ),
    ]);
    assert_eq!(conflicting.visible_conflict_count, 1);
}

#[test]
fn non_numeric_values_keep_string_fallback() {
    let pack = pack_from_cells(vec![
        retrieved(
            1,
            "project=Solar\nmetric=risk\nvalue=low\n\nSolar risk is low.",
        ),
        retrieved(
            2,
            "project=Solar\nmetric=risk\nvalue=high\n\nSolar risk is high.",
        ),
    ]);

    assert_eq!(pack.visible_conflict_count, 1);
}

#[test]
fn conflict_normalization_is_deterministic() {
    let cells = vec![
        retrieved(
            1,
            "project=Solar\nmetric=budget\nvalue=$1.2M\n\nSolar budget is $1.2M.",
        ),
        retrieved(
            2,
            "project=Solar\nmetric=budget\nvalue=1.4M USD\n\nSolar budget is 1.4M USD.",
        ),
    ];
    let first = pack_from_cells(cells.clone());
    let second = pack_from_cells(cells);

    assert_eq!(first.visible_conflict_count, second.visible_conflict_count);
    assert_eq!(
        first.conflict_visibility_q16,
        second.conflict_visibility_q16
    );
    assert_eq!(first.anomalies, second.anomalies);
}

fn pack_from_cells(cells: Vec<RetrievedCell>) -> ContextPack {
    ContextPack::from_retrieved_with_options(
        cells,
        2_000,
        false,
        &ContextPackOptions::default(),
        "",
    )
}

fn retrieved(cell_id: u64, payload: &str) -> RetrievedCell {
    RetrievedCell::from_payload(CellId(cell_id), payload.as_bytes().to_vec())
}
