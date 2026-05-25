use crate::ast::SourceSpan;

use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AqlParseErrorKind {
    Unexpected,
    ExpectedKeyword,
    Incomplete,
    InvalidInteger,
    InvalidMode,
    InvalidStringEscape,
    WhereDepthExceeded,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AqlParseError {
    pub kind: AqlParseErrorKind,
    pub span: SourceSpan,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AqlParseDiagnostic {
    pub kind: AqlParseErrorKind,
    pub span: SourceSpan,
    pub message: String,
}

impl AqlParseError {
    pub fn new(kind: AqlParseErrorKind, span: SourceSpan) -> Self {
        let message = format!(
            "{} at line {}, column {}",
            kind.safe_message(),
            span.line,
            span.column
        );
        Self {
            kind,
            span,
            message,
        }
    }

    pub fn diagnostic(&self) -> AqlParseDiagnostic {
        AqlParseDiagnostic {
            kind: self.kind.clone(),
            span: self.span,
            message: self.message.clone(),
        }
    }
}

impl From<AqlParseError> for AqlParseDiagnostic {
    fn from(error: AqlParseError) -> Self {
        error.diagnostic()
    }
}

impl fmt::Display for AqlParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl fmt::Display for AqlParseDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl AqlParseErrorKind {
    pub fn safe_message(&self) -> &'static str {
        match self {
            Self::Unexpected => "unexpected AQL syntax",
            Self::ExpectedKeyword => "expected AQL keyword",
            Self::Incomplete => "incomplete AQL input",
            Self::InvalidInteger => "invalid integer literal",
            Self::InvalidMode => "invalid retrieval mode",
            Self::InvalidStringEscape => "invalid string escape",
            Self::WhereDepthExceeded => "WHERE expression depth exceeded",
        }
    }
}
