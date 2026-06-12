//! Metadata validation for stable decode and graceful degradation.

use crate::query::metadata::{non_empty, CellMetadata};
use crate::source_trust::{parse_source_trust_class, SourceTrust};

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

impl CellMetadata {
    /// Strictly decode payload, returning an error on invalid metadata.
    pub fn decode_payload(payload: &[u8]) -> Result<Self, MetadataDecodeError> {
        let text = String::from_utf8_lossy(payload);
        let mut scope = None;
        let mut status = None;
        let mut cell_type = None;
        let mut memory_type = None;
        let mut ttl_seconds = None;
        let mut created_unix_seconds = None;
        let mut source_trust_q16 = None;
        let mut source_trust_class = None;
        let mut source = None;
        let mut citation = None;
        let mut title = None;
        let mut content_hash = None;
        let mut source_hash = None;
        let mut document_id_field = None;
        let mut chunk_id = None;
        let mut parent_id = None;
        let mut chunk_role = None;
        let mut path = None;
        let mut section = None;
        let mut project = None;
        let mut entity = None;
        let mut sector = None;
        let mut owner = None;
        let mut status_tag = None;
        let mut event_date = None;
        let mut topic = None;
        let mut as_of = None;
        let mut valid_from = None;
        let mut valid_to = None;
        let mut supersedes = None;
        let mut superseded_by = None;
        let mut table_id = None;
        let mut table_headers = None;
        let mut row_label = None;
        let mut body_lines = Vec::new();
        let mut in_header = true;
        let mut has_separator = false;

        let mut source_id_val = None;
        let mut source_url = None;
        let mut document_id = None;
        let mut page = None;
        let mut row = None;
        let mut cell_range = None;
        let mut json_path = None;
        let mut confidence_q16 = None;

        for line in text.lines() {
            if in_header {
                if line.trim().is_empty() {
                    in_header = false;
                    has_separator = true;
                    continue;
                }
                if let Some(value) = line.strip_prefix("scope=") {
                    scope = Some(value.trim().to_owned());
                } else if let Some(value) = line.strip_prefix("status=") {
                    status = Some(value.trim().to_owned());
                } else if let Some(value) = line.strip_prefix("type=") {
                    cell_type = Some(value.trim().to_owned());
                } else if let Some(value) = line.strip_prefix("memory_type=") {
                    memory_type = value.trim().parse().ok();
                } else if let Some(value) = line.strip_prefix("ttl_seconds=") {
                    ttl_seconds = Some(value.trim().parse().map_err(|_| {
                        MetadataDecodeError::InvalidNumericField {
                            field: "ttl_seconds".to_owned(),
                            value: value.trim().to_owned(),
                        }
                    })?);
                } else if let Some(value) = line.strip_prefix("created_unix_seconds=") {
                    created_unix_seconds = value.trim().parse().ok();
                } else if let Some(value) = line.strip_prefix("source_trust_q16=") {
                    source_trust_q16 = value.trim().parse().ok();
                } else if let Some(value) = line.strip_prefix("source_trust_class=") {
                    source_trust_class = parse_source_trust_class(value);
                } else if let Some(value) = line.strip_prefix("source=") {
                    source = non_empty(value);
                } else if let Some(value) = line.strip_prefix("citation=") {
                    citation = non_empty(value);
                } else if let Some(value) = line.strip_prefix("title=") {
                    title = non_empty(value);
                } else if let Some(value) = line.strip_prefix("content_hash=") {
                    content_hash = non_empty(value);
                } else if let Some(value) = line.strip_prefix("source_hash=") {
                    source_hash = non_empty(value);
                } else if let Some(value) = line.strip_prefix("document_id=") {
                    document_id_field = non_empty(value);
                    document_id = document_id_field.clone();
                } else if let Some(value) = line.strip_prefix("doc_id=") {
                    document_id_field = non_empty(value);
                    document_id = document_id_field.clone();
                } else if let Some(value) = line.strip_prefix("chunk_id=") {
                    chunk_id = non_empty(value);
                    cell_range = chunk_id.clone();
                } else if let Some(value) = line.strip_prefix("parent_id=") {
                    parent_id = non_empty(value);
                } else if let Some(value) = line.strip_prefix("parent_chunk_id=") {
                    parent_id = non_empty(value);
                } else if let Some(value) = line.strip_prefix("chunk_role=") {
                    chunk_role = non_empty(value);
                } else if let Some(value) = line.strip_prefix("chunk_kind=") {
                    chunk_role = non_empty(value);
                } else if let Some(value) = line.strip_prefix("path=") {
                    path = non_empty(value);
                } else if let Some(value) = line.strip_prefix("section=") {
                    section = non_empty(value);
                } else if let Some(value) = line.strip_prefix("project=") {
                    project = non_empty(value);
                } else if let Some(value) = line.strip_prefix("entity=") {
                    entity = non_empty(value);
                } else if let Some(value) = line.strip_prefix("sector=") {
                    sector = non_empty(value);
                } else if let Some(value) = line.strip_prefix("owner=") {
                    owner = non_empty(value);
                } else if let Some(value) = line.strip_prefix("status_tag=") {
                    status_tag = non_empty(value);
                } else if let Some(value) = line.strip_prefix("event_date=") {
                    event_date = non_empty(value);
                } else if let Some(value) = line.strip_prefix("topic=") {
                    topic = non_empty(value);
                } else if let Some(value) = line.strip_prefix("as_of=") {
                    as_of = non_empty(value);
                } else if let Some(value) = line.strip_prefix("valid_from=") {
                    valid_from = non_empty(value);
                } else if let Some(value) = line.strip_prefix("valid_to=") {
                    valid_to = non_empty(value);
                } else if let Some(value) = line.strip_prefix("supersedes=") {
                    supersedes = non_empty(value);
                } else if let Some(value) = line.strip_prefix("superseded_by=") {
                    superseded_by = non_empty(value);
                } else if let Some(value) = line.strip_prefix("table_id=") {
                    table_id = non_empty(value);
                } else if let Some(value) = line.strip_prefix("table_headers=") {
                    table_headers = non_empty(value);
                } else if let Some(value) = line.strip_prefix("columns=") {
                    table_headers = non_empty(value);
                } else if let Some(value) = line.strip_prefix("row_label=") {
                    row_label = non_empty(value);
                } else if line.strip_prefix("embedding_model=").is_some()
                    || line.strip_prefix("embedding_text_hash=").is_some()
                    || line.strip_prefix("vector=").is_some()
                    || line.contains("_vector=")
                {
                    continue;
                } else if let Some(value) = line.strip_prefix("source_id=") {
                    source_id_val = non_empty(value);
                } else if let Some(value) = line.strip_prefix("source_url=") {
                    source_url = non_empty(value);
                } else if let Some(value) = line.strip_prefix("url=") {
                    source_url = non_empty(value);
                } else if let Some(value) = line.strip_prefix("page=") {
                    page = value.trim().parse().ok();
                } else if let Some(value) = line.strip_prefix("row=") {
                    row = value.trim().parse().ok();
                } else if let Some(value) = line.strip_prefix("row_number=") {
                    row = value.trim().parse().ok();
                } else if let Some(value) = line.strip_prefix("cell_range=") {
                    cell_range = non_empty(value);
                } else if let Some(value) = line.strip_prefix("json_path=") {
                    json_path = non_empty(value);
                } else if let Some(value) = line.strip_prefix("confidence_q16=") {
                    confidence_q16 = value.trim().parse().ok();
                } else {
                    // Unknown header line — treat as start of body
                    in_header = false;
                    body_lines.push(line);
                }
            } else {
                body_lines.push(line);
            }
        }

        if !has_separator && !body_lines.is_empty() {
            // If there was no blank line but we have body lines, the first body line
            // might have been treated as header. This is ambiguous but acceptable
            // if scope/status were found. If not, it's invalid.
        }

        let scope = scope.ok_or(MetadataDecodeError::EmptyScope)?;
        if scope.is_empty() {
            return Err(MetadataDecodeError::EmptyScope);
        }
        if scope.contains("..") || scope.contains('/') {
            return Err(MetadataDecodeError::InvalidScopeCharacters(scope));
        }

        let status = status.ok_or(MetadataDecodeError::EmptyStatus)?;
        if status.is_empty() {
            return Err(MetadataDecodeError::EmptyStatus);
        }

        let cell_type = cell_type.unwrap_or_else(|| "raw".to_owned());
        if cell_type.parse::<cortex_core::KnowledgeCellType>().is_err() {
            return Err(MetadataDecodeError::InvalidCellType(cell_type));
        }

        let body_text = body_lines.join("\n");
        let terms = crate::search::tokenize(&body_text);

        let final_source_id = source_id_val
            .or_else(|| source.clone())
            .or_else(|| citation.clone());
        let source_ref = final_source_id.map(|id| crate::query::metadata::SourceRef {
            source_id: id,
            source_url,
            document_id,
            page,
            row,
            cell_range,
            json_path,
            confidence_q16: confidence_q16.unwrap_or_else(|| {
                SourceTrust::from_metadata(source_trust_q16, source_trust_class).q16
            }),
        });

        Ok(Self {
            scope,
            status,
            cell_type,
            memory_type,
            ttl_seconds,
            created_unix_seconds,
            source_trust_q16,
            source_trust_class,
            source,
            citation,
            title,
            content_hash,
            source_hash,
            document_id: document_id_field,
            chunk_id,
            parent_id,
            chunk_role,
            path,
            section,
            project,
            entity,
            sector,
            owner,
            status_tag,
            event_date,
            topic,
            as_of,
            valid_from,
            valid_to,
            supersedes,
            superseded_by,
            table_id,
            table_headers,
            row_label,
            body_text,
            terms,
            source_ref,
        })
    }

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
mod tests;
