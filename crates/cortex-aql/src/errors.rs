use crate::ast::SourceSpan;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AqlParseErrorKind {
    Unexpected,
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

impl AqlParseError {
    pub fn new(kind: AqlParseErrorKind, span: SourceSpan) -> Self {
        let message = match kind {
            AqlParseErrorKind::Unexpected => "unexpected AQL syntax",
            AqlParseErrorKind::Incomplete => "incomplete AQL input",
            AqlParseErrorKind::InvalidInteger => "invalid integer literal",
            AqlParseErrorKind::InvalidMode => "invalid retrieval mode",
            AqlParseErrorKind::InvalidStringEscape => "invalid string escape",
            AqlParseErrorKind::WhereDepthExceeded => "WHERE expression depth exceeded",
        };
        Self {
            kind,
            span,
            message: message.to_owned(),
        }
    }
}
