use crate::memory::format_scale_currency;

#[test]
fn test_numeric_scale_currency_formatting() {
    // Value = 1.2 with KZT should not format as 1.2B KZT, but remain 1.2 KZT
    assert_eq!(format_scale_currency("1.2", "KZT"), "1.2 KZT");

    // Value = 1200000000 with KZT should format as 1.2B KZT
    assert_eq!(format_scale_currency("1200000000", "KZT"), "1.2B KZT");

    // Value = 1500000 with USD should format as 1.5M USD
    assert_eq!(format_scale_currency("1500000", "USD"), "1.5M USD");
}
