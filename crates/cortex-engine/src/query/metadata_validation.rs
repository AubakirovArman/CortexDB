//! Metadata validation for stable decode and graceful degradation.

use crate::query::metadata::CellMetadata;

/// Validation error for cell metadata fields.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MetadataValidationError {
    EmptyScope,
    EmptyStatus,
    InvalidScopeCharacters(String),
    InvalidTtlSeconds(u64),
    InvalidCellType(String),
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

impl CellMetadata {
    /// Validate metadata fields and return the first error encountered.
    pub fn validate(&self) -> Result<(), MetadataValidationError> {
        if self.scope.is_empty() {
            return Err(MetadataValidationError::EmptyScope);
        }
        if self.scope.contains("..") || self.scope.contains('/') {
            return Err(MetadataValidationError::InvalidScopeCharacters(
                self.scope.clone(),
            ));
        }
        if self.status.is_empty() {
            return Err(MetadataValidationError::EmptyStatus);
        }
        if let Some(ttl) = self.ttl_seconds {
            if ttl == 0 {
                return Err(MetadataValidationError::InvalidTtlSeconds(ttl));
            }
        }
        if self
            .cell_type
            .parse::<cortex_core::KnowledgeCellType>()
            .is_err()
        {
            return Err(MetadataValidationError::InvalidCellType(
                self.cell_type.clone(),
            ));
        }
        Ok(())
    }

    /// Sanitize metadata into a guaranteed-valid form, applying safe defaults.
    pub fn sanitized(self) -> Self {
        let mut m = self;
        if m.scope.is_empty() || m.scope.contains("..") || m.scope.contains('/') {
            m.scope = "default".to_owned();
        }
        if m.status.is_empty() {
            m.status = "ready".to_owned();
        }
        if let Some(ttl) = m.ttl_seconds {
            if ttl == 0 {
                m.ttl_seconds = None;
            }
        }
        m
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::metadata::CellMetadata;

    #[test]
    fn valid_metadata_passes() {
        let m = CellMetadata::from_payload(b"scope=project:test\nstatus=ready\n\nhello world");
        assert!(m.validate().is_ok());
    }

    #[test]
    fn empty_scope_fails() {
        let m = CellMetadata::from_payload(b"scope=\nstatus=ready\n\nhello");
        assert_eq!(m.validate(), Err(MetadataValidationError::EmptyScope));
    }

    #[test]
    fn path_traversal_scope_fails() {
        let m = CellMetadata::from_payload(b"scope=../etc\nstatus=ready\n\nhello");
        assert_eq!(
            m.validate(),
            Err(MetadataValidationError::InvalidScopeCharacters(
                "../etc".to_owned()
            ))
        );
    }

    #[test]
    fn slash_in_scope_fails() {
        let m = CellMetadata::from_payload(b"scope=a/b\nstatus=ready\n\nhello");
        assert_eq!(
            m.validate(),
            Err(MetadataValidationError::InvalidScopeCharacters(
                "a/b".to_owned()
            ))
        );
    }

    #[test]
    fn empty_status_fails() {
        let m = CellMetadata::from_payload(b"scope=project:test\nstatus=\n\nhello");
        assert_eq!(m.validate(), Err(MetadataValidationError::EmptyStatus));
    }

    #[test]
    fn zero_ttl_fails() {
        let m =
            CellMetadata::from_payload(b"scope=project:test\nstatus=ready\nttl_seconds=0\n\nhello");
        assert_eq!(
            m.validate(),
            Err(MetadataValidationError::InvalidTtlSeconds(0))
        );
    }

    #[test]
    fn out_of_range_source_trust_is_gracefully_dropped() {
        let m = CellMetadata::from_payload(
            b"scope=project:test\nstatus=ready\nsource_trust_q16=99999\n\nhello",
        );
        assert_eq!(m.source_trust_q16, None);
    }

    #[test]
    fn sanitize_fixes_invalid_scope() {
        let m = CellMetadata::from_payload(b"scope=../etc\nstatus=ready\n\nhello");
        let fixed = m.sanitized();
        assert_eq!(fixed.scope, "default");
        assert_eq!(fixed.status, "ready");
    }

    #[test]
    fn sanitize_fixes_invalid_status() {
        let m = CellMetadata::from_payload(b"scope=project:test\nstatus=\n\nhello");
        let fixed = m.sanitized();
        assert_eq!(fixed.status, "ready");
    }

    #[test]
    fn sanitize_keeps_valid_source_trust() {
        let m = CellMetadata::from_payload(
            b"scope=project:test\nstatus=ready\nsource_trust_q16=60000\n\nhello",
        );
        let fixed = m.sanitized();
        assert_eq!(fixed.source_trust_q16, Some(60000));
    }

    #[test]
    fn sanitize_clears_zero_ttl() {
        let m =
            CellMetadata::from_payload(b"scope=project:test\nstatus=ready\nttl_seconds=0\n\nhello");
        let fixed = m.sanitized();
        assert_eq!(fixed.ttl_seconds, None);
    }
}
