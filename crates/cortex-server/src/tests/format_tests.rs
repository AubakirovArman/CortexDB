use cortex_engine::format_scaled_value;

#[test]
fn test_numeric_scale_currency_formatting() {
    // Value = 1.2 with KZT should not format as 1.2B KZT, but remain 1.2 KZT
    assert_eq!(format_scaled_value(1, Some("KZT"), None), "1 KZT");

    // Value = 1200000000 with KZT should format as 1.2B KZT
    assert_eq!(
        format_scaled_value(1_200_000_000, Some("KZT"), None),
        "1.2B KZT"
    );

    // Value = 1500000 with USD should format as 1.5M USD
    assert_eq!(
        format_scaled_value(1_500_000, Some("USD"), None),
        "1.5M USD"
    );
}
