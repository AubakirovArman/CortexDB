use super::*;

#[test]
fn parse_temporal_date_accepts_iso_dates_and_years() {
    assert_eq!(
        parse_temporal_date("2025-02-28").unwrap().as_iso_date(),
        "2025-02-28"
    );
    assert_eq!(
        parse_temporal_date("2024-02-29").unwrap().as_iso_date(),
        "2024-02-29"
    );
    assert_eq!(
        parse_temporal_date("2025").unwrap().as_iso_date(),
        "2025-01-01"
    );
}

#[test]
fn parse_temporal_date_rejects_invalid_dates() {
    assert!(parse_temporal_date("2025-02-29").is_none());
    assert!(parse_temporal_date("2025-13-01").is_none());
    assert!(parse_temporal_date("12000").is_none());
}

#[test]
fn extract_temporal_query_range_prefers_exact_date_then_year() {
    let exact = extract_temporal_query_range("budget valid on 2025-05-02").unwrap();
    assert_eq!(exact.start.as_iso_date(), "2025-05-02");
    assert_eq!(exact.end.as_iso_date(), "2025-05-02");

    let year = extract_temporal_query_range("budget valid in 2025").unwrap();
    assert_eq!(year.start.as_iso_date(), "2025-01-01");
    assert_eq!(year.end.as_iso_date(), "2025-12-31");
}

#[test]
fn temporal_validity_reports_expired_and_not_yet_valid() {
    let query = extract_temporal_query_range("budget on 2025-01-10").unwrap();
    let expired = TemporalValidity {
        valid_from: None,
        valid_to: parse_temporal_date("2024-12-31"),
    };
    let future = TemporalValidity {
        valid_from: parse_temporal_date("2026-01-01"),
        valid_to: None,
    };
    assert!(matches!(
        expired.stale_reason(query),
        Some(TemporalStaleReason::Expired { .. })
    ));
    assert!(matches!(
        future.stale_reason(query),
        Some(TemporalStaleReason::NotYetValid { .. })
    ));
}
