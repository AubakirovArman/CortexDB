use crate::query::CellMetadata;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TemporalDate {
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

impl TemporalDate {
    pub fn new(year: u16, month: u8, day: u8) -> Option<Self> {
        if !(1..=12).contains(&month) {
            return None;
        }
        let max_day = days_in_month(year, month);
        if day == 0 || day > max_day {
            return None;
        }
        Some(Self { year, month, day })
    }

    pub fn as_iso_date(self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TemporalQueryRange {
    pub start: TemporalDate,
    pub end: TemporalDate,
}

impl TemporalQueryRange {
    fn exact(date: TemporalDate) -> Self {
        Self {
            start: date,
            end: date,
        }
    }

    fn year(year: u16) -> Self {
        Self {
            start: TemporalDate {
                year,
                month: 1,
                day: 1,
            },
            end: TemporalDate {
                year,
                month: 12,
                day: 31,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TemporalValidity {
    pub valid_from: Option<TemporalDate>,
    pub valid_to: Option<TemporalDate>,
}

impl TemporalValidity {
    pub fn is_empty(self) -> bool {
        self.valid_from.is_none() && self.valid_to.is_none()
    }

    pub fn stale_reason(self, query: TemporalQueryRange) -> Option<TemporalStaleReason> {
        if let Some(valid_to) = self.valid_to {
            if query.start > valid_to {
                return Some(TemporalStaleReason::Expired { valid_to });
            }
        }
        if let Some(valid_from) = self.valid_from {
            if query.end < valid_from {
                return Some(TemporalStaleReason::NotYetValid { valid_from });
            }
        }
        None
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TemporalStaleReason {
    Expired { valid_to: TemporalDate },
    NotYetValid { valid_from: TemporalDate },
}

pub fn parse_temporal_date(value: &str) -> Option<TemporalDate> {
    parse_temporal_date_parts(value).and_then(|(year, month, day)| {
        TemporalDate::new(year, month.unwrap_or(1), day.unwrap_or(1))
    })
}

pub fn extract_temporal_query_range(text: &str) -> Option<TemporalQueryRange> {
    let tokens = text.split_whitespace().collect::<Vec<_>>();
    for token in &tokens {
        let cleaned = clean_temporal_token(token);
        if let Some((year, Some(month), Some(day))) = parse_temporal_date_parts(cleaned) {
            if let Some(date) = TemporalDate::new(year, month, day) {
                return Some(TemporalQueryRange::exact(date));
            }
        }
    }
    tokens.into_iter().find_map(|token| {
        let cleaned = clean_temporal_token(token);
        parse_year(cleaned).map(TemporalQueryRange::year)
    })
}

pub(super) fn temporal_validity_from_payload(payload: &[u8]) -> TemporalValidity {
    let mut valid_from = None;
    let mut valid_to = None;
    for line in String::from_utf8_lossy(payload).lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            break;
        }
        if let Some(value) = trimmed.strip_prefix("valid_from=") {
            valid_from = parse_temporal_bound(value.trim(), Boundary::Start);
        } else if let Some(value) = trimmed.strip_prefix("valid_to=") {
            valid_to = parse_temporal_bound(value.trim(), Boundary::End);
        }
    }
    TemporalValidity {
        valid_from,
        valid_to,
    }
}

pub(super) fn temporal_validity_from_metadata(metadata: &CellMetadata) -> TemporalValidity {
    TemporalValidity {
        valid_from: metadata
            .valid_from
            .as_deref()
            .and_then(|value| parse_temporal_bound(value, Boundary::Start)),
        valid_to: metadata
            .valid_to
            .as_deref()
            .and_then(|value| parse_temporal_bound(value, Boundary::End)),
    }
}

pub(super) fn temporal_stale_reason_with_metadata(
    fact: &str,
    metadata: &CellMetadata,
) -> Option<TemporalStaleReason> {
    let query = extract_temporal_query_range(fact)?;
    let validity = temporal_validity_from_metadata(metadata);
    if validity.is_empty() {
        return None;
    }
    temporal_stale_reason_from_metadata(fact, query, validity, metadata)
}

fn temporal_stale_reason_from_metadata(
    fact: &str,
    query: TemporalQueryRange,
    validity: TemporalValidity,
    metadata: &CellMetadata,
) -> Option<TemporalStaleReason> {
    (non_temporal_overlap(fact, &metadata.body_text) > 0).then(|| validity.stale_reason(query))?
}

fn parse_temporal_bound(value: &str, boundary: Boundary) -> Option<TemporalDate> {
    let (year, month, day) = parse_temporal_date_parts(value)?;
    match (month, day, boundary) {
        (Some(month), Some(day), _) => TemporalDate::new(year, month, day),
        (Some(month), None, Boundary::Start) => TemporalDate::new(year, month, 1),
        (Some(month), None, Boundary::End) => {
            TemporalDate::new(year, month, days_in_month(year, month))
        }
        (None, None, Boundary::Start) => TemporalDate::new(year, 1, 1),
        (None, None, Boundary::End) => TemporalDate::new(year, 12, 31),
        (None, Some(_), _) => None,
    }
}

fn parse_temporal_date_parts(value: &str) -> Option<(u16, Option<u8>, Option<u8>)> {
    let cleaned = clean_temporal_token(value);
    if cleaned.len() == 4 {
        return parse_year(cleaned).map(|year| (year, None, None));
    }
    let separator = if cleaned.contains('-') { '-' } else { '/' };
    let parts = cleaned.split(separator).collect::<Vec<_>>();
    match parts.as_slice() {
        [year] => parse_year(year).map(|year| (year, None, None)),
        [year, month] => Some((parse_year(year)?, Some(parse_two_digits(month)?), None)),
        [year, month, day] => Some((
            parse_year(year)?,
            Some(parse_two_digits(month)?),
            Some(parse_two_digits(day)?),
        )),
        _ => None,
    }
}

fn parse_year(value: &str) -> Option<u16> {
    if value.len() != 4 || !value.chars().all(|character| character.is_ascii_digit()) {
        return None;
    }
    let year = value.parse::<u16>().ok()?;
    (1900..=2199).contains(&year).then_some(year)
}

fn parse_two_digits(value: &str) -> Option<u8> {
    if value.len() != 2 || !value.chars().all(|character| character.is_ascii_digit()) {
        return None;
    }
    value.parse::<u8>().ok()
}

fn clean_temporal_token(value: &str) -> &str {
    value.trim_matches(|character: char| {
        !character.is_ascii_alphanumeric() && character != '-' && character != '/'
    })
}

fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: u16) -> bool {
    year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400))
}

fn non_temporal_overlap(left: &str, right: &str) -> usize {
    let right_terms = non_temporal_terms(right);
    non_temporal_terms(left)
        .into_iter()
        .filter(|term| right_terms.contains(term))
        .count()
}

fn non_temporal_terms(text: &str) -> Vec<String> {
    crate::search::tokenize(text)
        .into_iter()
        .filter(|term| extract_temporal_query_range(term).is_none())
        .filter(|term| crate::verification::numeric::extract_numeric_values(term).is_empty())
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Boundary {
    Start,
    End,
}

#[cfg(test)]
mod tests {
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
}
