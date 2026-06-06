//! Numeric value parsing for deterministic fact verification.
//!
//! Numeric handling is intentionally integer-only. It parses amounts with
//! optional magnitude, currency, and unit context into `NumericValue`, then
//! compares normalized values without `f32`/`f64`.

mod parse;
mod value;

pub use parse::{
    extract_numeric_values, parse_currency_code, parse_magnitude_suffix, parse_unit_code,
};
pub use value::{
    compare_numeric_values, format_scaled_value, normalized_numeric_equal, numeric_conflict,
    Magnitude, NumericComparison, NumericValue,
};

#[cfg(test)]
mod tests;
