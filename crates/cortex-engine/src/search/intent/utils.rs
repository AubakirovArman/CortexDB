pub(super) fn has_date_or_version_signal(query: &str) -> bool {
    query
        .split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ';' | ')' | '('))
        .any(|token| is_iso_date(token) || is_version_like(token))
        || contains_any(
            query,
            &[
                "jan ",
                "january",
                "feb ",
                "february",
                "march",
                "april",
                "may 20",
                "june",
                "july",
                "august",
                "september",
                "october",
                "november",
                "december",
                "h1 2025",
                "runtime 1.",
            ],
        )
}

fn is_iso_date(token: &str) -> bool {
    let bytes = token.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, value)| matches!(index, 4 | 7) || value.is_ascii_digit())
}

fn is_version_like(token: &str) -> bool {
    let value = token.strip_prefix('v').unwrap_or(token);
    value.contains('.')
        && value.chars().any(|ch| ch.is_ascii_digit())
        && value
            .chars()
            .all(|ch| ch.is_ascii_digit() || matches!(ch, '.' | '-' | '_'))
}

pub(super) fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

pub(super) fn normalize_label(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
        .collect()
}
