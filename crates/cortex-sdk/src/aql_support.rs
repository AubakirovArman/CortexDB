use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AqlBuildError {
    field: &'static str,
    value: String,
    expected: &'static str,
}

impl AqlBuildError {
    pub(crate) fn invalid(field: &'static str, value: &str, expected: &'static str) -> Self {
        Self {
            field,
            value: value.to_owned(),
            expected,
        }
    }
}

impl fmt::Display for AqlBuildError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid AQL {} {:?}; expected {}",
            self.field, self.value, self.expected
        )
    }
}

impl std::error::Error for AqlBuildError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AqlRetrievalMode {
    Fast,
    Balanced,
    Semantic,
    Audit,
}

impl AqlRetrievalMode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Fast => "fast",
            Self::Balanced => "balanced",
            Self::Semantic => "semantic",
            Self::Audit => "audit",
        }
    }
}

pub(crate) fn quote_string(value: &str) -> String {
    let mut quoted = String::with_capacity(value.len() + 2);
    quoted.push('"');
    for character in value.chars() {
        match character {
            '"' => quoted.push_str("\\\""),
            '\\' => quoted.push_str("\\\\"),
            '\n' => quoted.push_str("\\n"),
            '\r' => quoted.push_str("\\r"),
            '\t' => quoted.push_str("\\t"),
            _ => quoted.push(character),
        }
    }
    quoted.push('"');
    quoted
}

pub(crate) fn validate_identifier(field: &'static str, value: &str) -> Result<(), AqlBuildError> {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return Err(AqlBuildError::invalid(field, value, "non-empty identifier"));
    };
    if !(first == '_' || first.is_ascii_alphabetic()) {
        return Err(AqlBuildError::invalid(
            field,
            value,
            "identifier starting with '_' or an ASCII letter",
        ));
    }
    if chars.all(|ch| ch == '_' || ch == '-' || ch == ':' || ch.is_ascii_alphanumeric()) {
        Ok(())
    } else {
        Err(AqlBuildError::invalid(
            field,
            value,
            "identifier characters [A-Za-z0-9_:-]",
        ))
    }
}

pub(crate) fn validate_optional_fragment(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), AqlBuildError> {
    match value {
        Some(raw) if raw.trim().is_empty() => {
            Err(AqlBuildError::invalid(field, raw, "non-empty AQL fragment"))
        }
        _ => Ok(()),
    }
}

pub(crate) fn validate_optional_decimal(
    field: &'static str,
    value: Option<&str>,
) -> Result<(), AqlBuildError> {
    let Some(value) = value else {
        return Ok(());
    };
    let Some((left, right)) = value.split_once('.') else {
        return Err(AqlBuildError::invalid(field, value, "decimal literal"));
    };
    if !left.is_empty()
        && !right.is_empty()
        && left.chars().all(|ch| ch.is_ascii_digit())
        && right.chars().all(|ch| ch.is_ascii_digit())
    {
        Ok(())
    } else {
        Err(AqlBuildError::invalid(field, value, "decimal literal"))
    }
}
