use super::*;

fn value(
    scaled_value: u64,
    currency: Option<&str>,
    unit: Option<&str>,
    magnitude: Option<Magnitude>,
) -> NumericValue {
    NumericValue {
        raw: scaled_value.to_string(),
        scaled_value,
        currency: currency.map(str::to_owned),
        unit: unit.map(str::to_owned),
        magnitude,
    }
}

#[test]
fn parse_plain_integer() {
    let vals = extract_numeric_values("budget is 1200000000");
    assert_eq!(vals.len(), 1);
    assert_eq!(vals[0].scaled_value, 1_200_000_000);
    assert_eq!(vals[0].magnitude, None);
}

#[test]
fn parse_billion_with_currency() {
    let vals = extract_numeric_values("budget is 1.2B KZT");
    assert_eq!(vals.len(), 1);
    assert_eq!(vals[0].scaled_value, 1_200_000_000);
    assert_eq!(vals[0].currency, Some("KZT".to_owned()));
    assert_eq!(vals[0].magnitude, Some(Magnitude::Billion));
}

#[test]
fn parse_million_with_currency() {
    let vals = extract_numeric_values("revenue 1.5M USD");
    assert_eq!(vals.len(), 1);
    assert_eq!(vals[0].scaled_value, 1_500_000);
    assert_eq!(vals[0].currency, Some("USD".to_owned()));
    assert_eq!(vals[0].magnitude, Some(Magnitude::Million));
}

#[test]
fn parse_currency_symbol_as_usd() {
    let vals = extract_numeric_values("budget is $1.2M");
    assert_eq!(vals.len(), 1);
    assert_eq!(vals[0].scaled_value, 1_200_000);
    assert_eq!(vals[0].currency, Some("USD".to_owned()));
    assert_eq!(vals[0].magnitude, Some(Magnitude::Million));
}

#[test]
fn parse_thousand_with_unit() {
    let vals = extract_numeric_values("distance 15K m");
    assert_eq!(vals.len(), 1);
    assert_eq!(vals[0].scaled_value, 15_000);
    assert_eq!(vals[0].unit, Some("m".to_owned()));
    assert_eq!(vals[0].magnitude, Some(Magnitude::Thousand));
}

#[test]
fn parse_suffixed_unit() {
    let vals = extract_numeric_values("duration 60min");
    assert_eq!(vals.len(), 1);
    assert_eq!(vals[0].scaled_value, 60);
    assert_eq!(vals[0].unit, Some("min".to_owned()));
}

#[test]
fn parse_percent_and_percent_word_normalize_to_same_unit() {
    let symbol = extract_numeric_values("growth 12.5%");
    let word = extract_numeric_values("growth 12 percent");
    assert_eq!(symbol[0].unit, Some("%".to_owned()));
    assert_eq!(word[0].unit, Some("%".to_owned()));
    assert!(normalized_numeric_equal(&symbol[0], &word[0]));
}

#[test]
fn explicit_parsers_normalize_supported_contexts() {
    assert_eq!(parse_currency_code("kzt"), Some("KZT".to_owned()));
    assert_eq!(parse_unit_code("hrs"), Some("h".to_owned()));
    assert_eq!(parse_unit_code("percentage"), Some("%".to_owned()));
    assert_eq!(parse_magnitude_suffix("млрд"), Some(Magnitude::Billion));
    assert_eq!(parse_magnitude_suffix("mn"), Some(Magnitude::Million));
}

#[test]
fn normalized_comparison_reports_equal_conflict_and_incomparable() {
    let a = value(1_200_000_000, Some("KZT"), None, Some(Magnitude::Billion));
    let b = value(1_200_000_000, Some("KZT"), None, None);
    let c = value(1_400_000_000, Some("KZT"), None, Some(Magnitude::Billion));
    let d = value(1_200_000_000, Some("USD"), None, Some(Magnitude::Billion));
    let e = value(1_200_000_000, None, None, None);

    assert_eq!(compare_numeric_values(&a, &b), NumericComparison::Equal);
    assert_eq!(compare_numeric_values(&a, &c), NumericComparison::Conflict);
    assert_eq!(
        compare_numeric_values(&a, &d),
        NumericComparison::CurrencyMismatch
    );
    assert_eq!(
        compare_numeric_values(&a, &e),
        NumericComparison::Incomparable
    );
}

#[test]
fn conflict_helpers_match_normalized_comparison() {
    let a = value(12_000, Some("USD"), None, None);
    let b = value(12_000, Some("KZT"), None, None);
    let c = value(12_000, Some("USD"), None, None);

    assert!(numeric_conflict(&a, &b));
    assert!(!numeric_conflict(&a, &c));
    assert!(a.conflicts_with(&b));
    assert!(a.normalized_eq(&c));
}

#[test]
fn compatible_units_compare_in_integer_base_units() {
    let hour = extract_numeric_values("duration 1 h").remove(0);
    let minutes = extract_numeric_values("duration 60 min").remove(0);
    let two_hours = extract_numeric_values("duration 2h").remove(0);
    let meters = extract_numeric_values("length 1000 m").remove(0);
    let kilometers = extract_numeric_values("length 1 km").remove(0);
    let kilograms = extract_numeric_values("mass 1 kg").remove(0);

    assert!(normalized_numeric_equal(&hour, &minutes));
    assert!(numeric_conflict(&hour, &two_hours));
    assert!(normalized_numeric_equal(&meters, &kilometers));
    assert_eq!(
        compare_numeric_values(&meters, &kilograms),
        NumericComparison::Incomparable
    );
}

#[test]
fn unit_aliases_normalize_to_same_base_units() {
    let one_hour = extract_numeric_values("duration 1 hour").remove(0);
    let sixty_minutes = extract_numeric_values("duration 60 minutes").remove(0);
    let one_kilometer = extract_numeric_values("distance 1 kilometer").remove(0);
    let thousand_meters = extract_numeric_values("distance 1000 meters").remove(0);
    let one_kilogram = extract_numeric_values("mass 1 kilogram").remove(0);
    let thousand_grams = extract_numeric_values("mass 1000 grams").remove(0);

    assert!(normalized_numeric_equal(&one_hour, &sixty_minutes));
    assert!(normalized_numeric_equal(&one_kilometer, &thousand_meters));
    assert!(normalized_numeric_equal(&one_kilogram, &thousand_grams));
}

#[test]
fn cross_currency_conflict_is_labeled() {
    let usd = value(12_000, Some("USD"), None, None);
    let eur = value(12_000, Some("EUR"), None, None);
    let comparison = compare_numeric_values(&usd, &eur);

    assert_eq!(comparison, NumericComparison::CurrencyMismatch);
    assert_eq!(comparison.reason(), "currency_mismatch");
    assert!(comparison.is_conflict());
    assert!(numeric_conflict(&usd, &eur));
}

#[test]
fn large_unit_conversion_uses_u128_without_overflow() {
    let large_km = value(u64::MAX, None, Some("km"), None);
    let large_m = value(u64::MAX, None, Some("m"), None);

    assert_eq!(
        compare_numeric_values(&large_km, &large_m),
        NumericComparison::Conflict
    );
}

#[test]
fn format_scaled_values_are_stable() {
    assert_eq!(
        format_scaled_value(1_200_000_000, Some("KZT"), None),
        "1.2B KZT"
    );
    assert_eq!(
        format_scaled_value(1_500_000, Some("USD"), None),
        "1.5M USD"
    );
    assert_eq!(format_scaled_value(1500, None, None), "1500");
}
