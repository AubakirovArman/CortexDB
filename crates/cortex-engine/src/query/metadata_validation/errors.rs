/// Validation error for cell metadata fields.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MetadataValidationError {
    EmptyScope,
    EmptyStatus,
    InvalidScopeCharacters(String),
    InvalidTtlSeconds(u64),
    InvalidCellType(String),
}

/// Decode error for strict payload parsing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MetadataDecodeError {
    MissingBodySeparator,
    EmptyScope,
    EmptyStatus,
    InvalidScopeCharacters(String),
    InvalidCellType(String),
    InvalidNumericField { field: String, value: String },
}

impl core::fmt::Display for MetadataDecodeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::MissingBodySeparator => write!(
                f,
                "payload missing body separator (blank line after headers)"
            ),
            Self::EmptyScope => write!(f, "scope must not be empty"),
            Self::EmptyStatus => write!(f, "status must not be empty"),
            Self::InvalidScopeCharacters(s) => write!(f, "scope contains invalid characters: {s}"),
            Self::InvalidCellType(v) => write!(f, "unknown cell type: {v}"),
            Self::InvalidNumericField { field, value } => {
                write!(f, "invalid numeric value for {field}: {value}")
            }
        }
    }
}

impl core::fmt::Display for MetadataValidationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            MetadataValidationError::EmptyScope => write!(f, "scope must not be empty"),
            MetadataValidationError::EmptyStatus => write!(f, "status must not be empty"),
            MetadataValidationError::InvalidScopeCharacters(s) => {
                write!(f, "scope contains invalid characters: {s}")
            }
            MetadataValidationError::InvalidTtlSeconds(v) => {
                write!(f, "ttl_seconds must be > 0: {v}")
            }
            MetadataValidationError::InvalidCellType(v) => {
                write!(f, "unknown cell type: {v}")
            }
        }
    }
}
