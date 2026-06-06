/// A parsed numeric claim with full context.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NumericValue {
    /// The raw original text that produced this value.
    pub raw: String,
    /// Normalized integer value scaled to base units.
    pub scaled_value: u64,
    /// Optional currency code such as KZT, USD, or EUR.
    pub currency: Option<String>,
    /// Optional unit such as m, kg, %, or year.
    pub unit: Option<String>,
    /// Whether the original used a magnitude suffix.
    pub magnitude: Option<Magnitude>,
}

impl NumericValue {
    pub fn compare_normalized(&self, other: &Self) -> NumericComparison {
        compare_numeric_values(self, other)
    }

    pub fn normalized_eq(&self, other: &Self) -> bool {
        self.compare_normalized(other) == NumericComparison::Equal
    }

    pub fn conflicts_with(&self, other: &Self) -> bool {
        self.compare_normalized(other) == NumericComparison::Conflict
    }
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumericComparison {
    Equal,
    Conflict,
    Incomparable,
}

pub fn normalized_numeric_equal(left: &NumericValue, right: &NumericValue) -> bool {
    compare_numeric_values(left, right) == NumericComparison::Equal
}

pub fn numeric_conflict(left: &NumericValue, right: &NumericValue) -> bool {
    compare_numeric_values(left, right) == NumericComparison::Conflict
}

pub fn compare_numeric_values(left: &NumericValue, right: &NumericValue) -> NumericComparison {
    match (&left.currency, &right.currency) {
        (Some(left_currency), Some(right_currency)) => {
            if left_currency != right_currency {
                return NumericComparison::Conflict;
            }
            return compare_scaled(left, right);
        }
        (Some(_), None) | (None, Some(_)) => return NumericComparison::Incomparable,
        (None, None) => {}
    }

    match (&left.unit, &right.unit) {
        (Some(left_unit), Some(right_unit)) => {
            if left_unit != right_unit {
                return NumericComparison::Incomparable;
            }
            compare_scaled(left, right)
        }
        (Some(_), None) | (None, Some(_)) => NumericComparison::Incomparable,
        (None, None) => compare_scaled(left, right),
    }
}

fn compare_scaled(left: &NumericValue, right: &NumericValue) -> NumericComparison {
    if left.scaled_value == right.scaled_value {
        NumericComparison::Equal
    } else {
        NumericComparison::Conflict
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
