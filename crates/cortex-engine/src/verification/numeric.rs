//! Numeric value parsing for deterministic fact verification.
//!
//! Parses numeric claims from text with support for:
//! - Magnitude suffixes: B (billion), M (million), K (thousand), % (percent)
//! - Currency codes: KZT, USD, EUR, RUB, etc.
//! - Units: m, km, kg, t, etc.
//! - Integer-only arithmetic (no f64 in core path).

use std::str::FromStr;

/// A parsed numeric claim with full context.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NumericValue {
    /// The raw original text that produced this value.
    pub raw: String,
    /// Normalized integer value scaled to base units.
    /// For 1.2B KZT this is 1_200_000_000.
    /// For 1.5M USD this is 1_500_000.
    /// For 1500 this is 1500.
    pub scaled_value: u64,
    /// Optional currency code (KZT, USD, EUR, etc.)
    pub currency: Option<String>,
    /// Optional unit (m, km, kg, t, %, etc.)
    pub unit: Option<String>,
    /// Whether the original used a magnitude suffix.
    pub magnitude: Option<Magnitude>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Magnitude {
    Billion,
    Million,
    Thousand,
    Percent,
}

impl Magnitude {
    pub fn multiplier(self) -> u64 {
        match self {
            Magnitude::Billion => 1_000_000_000,
            Magnitude::Million => 1_000_000,
            Magnitude::Thousand => 1_000,
            Magnitude::Percent => 1,
        }
    }

    pub fn from_suffix(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "b" | "bn" | "billion" | "млрд" => Some(Magnitude::Billion),
            "m" | "mn" | "million" | "млн" => Some(Magnitude::Million),
            "k" | "thousand" | "тыс" => Some(Magnitude::Thousand),
            "%" | "percent" | "percentage" | "процент" => Some(Magnitude::Percent),
            _ => None,
        }
    }
}

/// Extract all numeric values from a text string.
pub fn extract_numeric_values(text: &str) -> Vec<NumericValue> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut results = Vec::new();
    let mut i = 0;
    while i < words.len() {
        let word = words[i]
            .trim_matches(|c: char| !c.is_alphanumeric() && c != '.' && c != ',' && c != '%');
        if let Some(value) = try_parse_numeric_word(word, &words, i) {
            results.push(value);
        }
        i += 1;
    }
    results
}

fn try_parse_numeric_word(word: &str, words: &[&str], index: usize) -> Option<NumericValue> {
    // Try to parse a decimal number like 1.2, 1,200, 1400000000
    let cleaned = word.replace(',', "");
    let base_value = parse_decimal(&cleaned)?;

    let mut magnitude: Option<Magnitude> = None;
    let mut currency: Option<String> = None;
    let mut unit: Option<String> = None;

    // Look ahead for magnitude/currency/unit in the next 2 words
    for offset in 1..=2 {
        let next_index = index + offset;
        if next_index >= words.len() {
            break;
        }
        let next_word = words[next_index].trim_matches(|c: char| !c.is_alphabetic() && c != '%');
        if next_word.is_empty() {
            continue;
        }

        if unit.is_none() && is_unit(next_word) {
            unit = Some(next_word.to_ascii_lowercase());
            continue;
        }

        if magnitude.is_none() {
            if let Some(mag) = Magnitude::from_suffix(next_word) {
                magnitude = Some(mag);
                continue;
            }
        }

        if currency.is_none() && is_currency(next_word) {
            currency = Some(next_word.to_ascii_uppercase());
            continue;
        }
    }

    // Also check if the number itself ends with a magnitude (e.g. "1.2B")
    if magnitude.is_none() {
        if let Some(last_char) = word.chars().last() {
            let mag_str = last_char.to_string();
            if let Some(mag) = Magnitude::from_suffix(&mag_str) {
                magnitude = Some(mag);
            }
        }
    }

    // Treat trailing % as unit if not already captured
    if unit.is_none()
        && word
            .trim_end_matches(['.', ','])
            .ends_with('%')
    {
        unit = Some("%".to_owned());
    }

    let scaled_value = match magnitude {
        Some(mag) => {
            let multiplier = mag.multiplier();
            let whole = base_value.whole;
            // For decimal values like 1.2B:
            // scaled = whole * multiplier + (fraction * multiplier / 10^fraction_digits)
            if base_value.fraction_digits > 0 {
                let frac_multiplier = multiplier / 10_u64.pow(base_value.fraction_digits);
                whole * multiplier + base_value.fraction * frac_multiplier
            } else {
                whole * multiplier
            }
        }
        None => base_value.whole,
    };

    Some(NumericValue {
        raw: word.to_owned(),
        scaled_value,
        currency,
        unit,
        magnitude,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DecimalValue {
    whole: u64,
    fraction: u64,
    fraction_digits: u32,
}

fn parse_decimal(s: &str) -> Option<DecimalValue> {
    if s.is_empty() {
        return None;
    }
    // Remove trailing non-digit characters (like B, M, %, . attached to number)
    let s = s.trim_end_matches(|c: char| c.is_alphabetic() || c == '%' || c == '.');
    if s.is_empty() {
        return None;
    }

    if let Some((whole_str, frac_str)) = s.split_once('.') {
        let whole = parse_u64_str(whole_str)?;
        let frac_str = frac_str.trim_end_matches('0');
        if frac_str.is_empty() {
            return Some(DecimalValue {
                whole,
                fraction: 0,
                fraction_digits: 0,
            });
        }
        let fraction = parse_u64_str(frac_str)?;
        let fraction_digits = frac_str.len() as u32;
        Some(DecimalValue {
            whole,
            fraction,
            fraction_digits,
        })
    } else {
        let whole = parse_u64_str(s)?;
        Some(DecimalValue {
            whole,
            fraction: 0,
            fraction_digits: 0,
        })
    }
}

fn parse_u64_str(s: &str) -> Option<u64> {
    if s.is_empty() {
        return None;
    }
    u64::from_str(s).ok()
}

fn is_currency(word: &str) -> bool {
    matches!(
        word.to_ascii_uppercase().as_str(),
        "KZT"
            | "USD"
            | "EUR"
            | "RUB"
            | "GBP"
            | "JPY"
            | "CNY"
            | "KGS"
            | "UZS"
            | "TJS"
            | "TMT"
            | "BYN"
            | "AMD"
            | "GEL"
            | "AZN"
            | "TRY"
    )
}

fn is_unit(word: &str) -> bool {
    matches!(
        word.to_ascii_lowercase().as_str(),
        "m" | "km"
            | "cm"
            | "mm"
            | "kg"
            | "g"
            | "t"
            | "l"
            | "ml"
            | "h"
            | "hr"
            | "hrs"
            | "min"
            | "sec"
            | "s"
            | "ms"
            | "day"
            | "days"
            | "week"
            | "weeks"
            | "month"
            | "months"
            | "year"
            | "years"
            | "%"
            | "percent"
            | "percentage"
    )
}

/// Detect if two numeric values contradict each other.
/// They contradict if they have the same currency/unit context but different scaled values.
pub fn numeric_conflict(left: &NumericValue, right: &NumericValue) -> bool {
    if left.scaled_value == right.scaled_value {
        return false;
    }
    // Same currency or same unit or both plain numbers
    match (&left.currency, &right.currency) {
        (Some(l), Some(r)) => l == r,
        (None, None) => match (&left.unit, &right.unit) {
            (Some(l), Some(r)) => l == r,
            (None, None) => true,
            _ => false,
        },
        _ => false,
    }
}

/// Format a scaled value back to human-readable display string (integer-only).
pub fn format_scaled_value(value: u64, currency: Option<&str>, unit: Option<&str>) -> String {
    let suffix = currency.or(unit).unwrap_or("");
    if value >= 1_000_000_000 && value.is_multiple_of(100_000_000) {
        let whole = value / 1_000_000_000;
        let tenths = (value % 1_000_000_000) / 100_000_000;
        if tenths == 0 {
            return format!("{whole}B {suffix}").trim().to_owned();
        }
        return format!("{whole}.{tenths}B {suffix}").trim().to_owned();
    }
    if value >= 1_000_000 && value.is_multiple_of(100_000) {
        let whole = value / 1_000_000;
        let tenths = (value % 1_000_000) / 100_000;
        if tenths == 0 {
            return format!("{whole}M {suffix}").trim().to_owned();
        }
        return format!("{whole}.{tenths}M {suffix}").trim().to_owned();
    }
    if value >= 1_000 && value.is_multiple_of(1_000) {
        let whole = value / 1_000;
        return format!("{whole}K {suffix}").trim().to_owned();
    }
    if suffix.is_empty() {
        value.to_string()
    } else {
        format!("{value} {suffix}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn parse_thousand() {
        let vals = extract_numeric_values("distance 15K m");
        assert_eq!(vals.len(), 1);
        assert_eq!(vals[0].scaled_value, 15_000);
        assert_eq!(vals[0].unit, Some("m".to_owned()));
        assert_eq!(vals[0].magnitude, Some(Magnitude::Thousand));
    }

    #[test]
    fn parse_percent() {
        let vals = extract_numeric_values("growth 12.5%");
        assert_eq!(vals.len(), 1);
        assert_eq!(vals[0].scaled_value, 12);
        assert_eq!(vals[0].magnitude, Some(Magnitude::Percent));
        assert_eq!(vals[0].unit, Some("%".to_owned()));
    }

    #[test]
    fn detect_conflict_same_currency() {
        let a = NumericValue {
            raw: "1.2B".to_owned(),
            scaled_value: 1_200_000_000,
            currency: Some("KZT".to_owned()),
            unit: None,
            magnitude: Some(Magnitude::Billion),
        };
        let b = NumericValue {
            raw: "1.4B".to_owned(),
            scaled_value: 1_400_000_000,
            currency: Some("KZT".to_owned()),
            unit: None,
            magnitude: Some(Magnitude::Billion),
        };
        assert!(numeric_conflict(&a, &b));
    }

    #[test]
    fn no_conflict_same_value() {
        let a = NumericValue {
            raw: "1.2B".to_owned(),
            scaled_value: 1_200_000_000,
            currency: Some("KZT".to_owned()),
            unit: None,
            magnitude: Some(Magnitude::Billion),
        };
        let b = NumericValue {
            raw: "1200000000".to_owned(),
            scaled_value: 1_200_000_000,
            currency: Some("KZT".to_owned()),
            unit: None,
            magnitude: None,
        };
        assert!(!numeric_conflict(&a, &b));
    }

    #[test]
    fn format_scaled_billion() {
        assert_eq!(
            format_scaled_value(1_200_000_000, Some("KZT"), None),
            "1.2B KZT"
        );
    }

    #[test]
    fn format_scaled_million() {
        assert_eq!(
            format_scaled_value(1_500_000, Some("USD"), None),
            "1.5M USD"
        );
    }

    #[test]
    fn format_scaled_plain() {
        assert_eq!(format_scaled_value(1500, None, None), "1500");
    }
}
