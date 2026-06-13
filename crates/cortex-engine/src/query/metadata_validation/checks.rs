use crate::query::metadata::CellMetadata;

use super::MetadataValidationError;

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
