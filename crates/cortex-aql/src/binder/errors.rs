use thiserror::Error;

use crate::policy::PolicyError;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum BindError {
    #[error("unknown brain")]
    UnknownBrain,
    #[error("unknown scope")]
    UnknownScope,
    #[error("unknown bitmap")]
    UnknownBitmap,
    #[error("field is not filterable: {0}")]
    FieldNotFilterable(String),
    #[error("unsupported comparator")]
    UnsupportedComparator,
    #[error("unsupported literal")]
    UnsupportedLiteral,
    #[error("invalid memory type")]
    InvalidMemoryType,
    #[error("invalid decimal literal")]
    InvalidDecimal,
    #[error("invalid date literal")]
    InvalidDate,
    #[error("invalid bitmap program")]
    InvalidBitmapProgram,
    #[error("statement is not supported by binder")]
    UnsupportedStatement,
    #[error("policy denied: {0:?}")]
    PolicyDenied(PolicyError),
}

impl BindError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::UnknownBrain => "UnknownBrain",
            Self::UnknownScope => "UnknownScope",
            Self::UnknownBitmap => "UnknownBitmap",
            Self::FieldNotFilterable(_) => "FieldNotFilterable",
            Self::UnsupportedComparator => "UnsupportedComparator",
            Self::UnsupportedLiteral => "UnsupportedLiteral",
            Self::InvalidMemoryType => "InvalidMemoryType",
            Self::InvalidDecimal => "InvalidDecimal",
            Self::InvalidDate => "InvalidDate",
            Self::InvalidBitmapProgram => "InvalidBitmapProgram",
            Self::UnsupportedStatement => "UnsupportedStatement",
            Self::PolicyDenied(_) => "PolicyDenied",
        }
    }

    pub fn safe_message(&self) -> &'static str {
        match self {
            Self::UnknownBrain => "requested brain is unavailable",
            Self::UnknownScope => "requested scope is unavailable",
            Self::UnknownBitmap => "required bitmap is unavailable",
            Self::FieldNotFilterable(_) => "field is not filterable",
            Self::UnsupportedComparator => "comparator is not supported for this field",
            Self::UnsupportedLiteral => "literal is not supported for this field",
            Self::InvalidMemoryType => "memory type is invalid",
            Self::InvalidDecimal => "decimal literal is invalid",
            Self::InvalidDate => "date literal is invalid",
            Self::InvalidBitmapProgram => "bitmap program is invalid",
            Self::UnsupportedStatement => "statement is not supported by binder",
            Self::PolicyDenied(error) => error.safe_message(),
        }
    }
}
