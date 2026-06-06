use std::str::FromStr;

use super::value::{Magnitude, NumericValue};

pub fn parse_magnitude_suffix(value: &str) -> Option<Magnitude> {
    match value.to_ascii_lowercase().as_str() {
        "b" | "bn" | "billion" | "млрд" => Some(Magnitude::Billion),
        "m" | "mn" | "million" | "млн" => Some(Magnitude::Million),
        "k" | "thousand" | "тыс" => Some(Magnitude::Thousand),
        "%" | "percent" | "percentage" | "процент" => Some(Magnitude::Percent),
        _ => None,
    }
}

pub fn parse_currency_code(value: &str) -> Option<String> {
    let code = value.to_ascii_uppercase();
    matches!(
        code.as_str(),
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
    .then_some(code)
}

pub fn parse_unit_code(value: &str) -> Option<String> {
    match value.to_ascii_lowercase().as_str() {
        "m" | "km" | "cm" | "mm" | "kg" | "g" | "t" | "l" | "ml" | "h" | "min" | "s" | "ms"
        | "day" | "days" | "week" | "weeks" | "month" | "months" | "year" | "years" => {
            Some(value.to_ascii_lowercase())
        }
        "hr" | "hrs" => Some("h".to_owned()),
        "sec" => Some("s".to_owned()),
        "%" | "percent" | "percentage" => Some("%".to_owned()),
        _ => None,
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
    let cleaned = word.replace(',', "");
    let base_value = parse_decimal(&cleaned)?;

    let mut magnitude: Option<Magnitude> = None;
    let mut currency: Option<String> = None;
    let mut unit: Option<String> = None;

    for offset in 1..=2 {
        let next_index = index + offset;
        if next_index >= words.len() {
            break;
        }
        let next_word = words[next_index].trim_matches(|c: char| !c.is_alphabetic() && c != '%');
        if next_word.is_empty() {
            continue;
        }

        if unit.is_none() {
            if let Some(parsed_unit) = parse_unit_code(next_word) {
                unit = Some(parsed_unit);
                continue;
            }
        }

        if magnitude.is_none() {
            if let Some(parsed_magnitude) = parse_magnitude_suffix(next_word) {
                magnitude = Some(parsed_magnitude);
                continue;
            }
        }

        if currency.is_none() {
            if let Some(parsed_currency) = parse_currency_code(next_word) {
                currency = Some(parsed_currency);
                continue;
            }
        }
    }

    if magnitude.is_none() {
        if let Some(last_char) = word.chars().last() {
            let mag_str = last_char.to_string();
            if let Some(parsed_magnitude) = parse_magnitude_suffix(&mag_str) {
                magnitude = Some(parsed_magnitude);
            }
        }
    }

    if unit.is_none() && word.trim_end_matches(['.', ',']).ends_with('%') {
        unit = Some("%".to_owned());
    }

    let scaled_value = match magnitude {
        Some(mag) => {
            let multiplier = mag.multiplier();
            let whole = base_value.whole;
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
