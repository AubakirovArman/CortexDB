use std::collections::BTreeSet;

use crate::query::metadata::CellMetadata;

use super::super::tokenize;
use super::normalize::normalize_for_match;
use super::types::{QueryScopeDirective, QueryScopeField, QueryScopeMapping};

pub fn scope_mapping_payload_bonus(mapping: &QueryScopeMapping, payload: &[u8]) -> u64 {
    if mapping.directives.is_empty() {
        return 0;
    }
    let metadata = CellMetadata::from_payload(payload);
    scope_mapping_metadata_bonus(mapping, &metadata)
}

pub fn scope_mapping_metadata_bonus(mapping: &QueryScopeMapping, metadata: &CellMetadata) -> u64 {
    if mapping.directives.is_empty() {
        return 0;
    }
    mapping
        .directives
        .iter()
        .filter(|directive| directive_matches_metadata(directive, metadata))
        .map(|directive| u64::from(directive.confidence_q16) / 6)
        .sum()
}

fn directive_matches_metadata(directive: &QueryScopeDirective, metadata: &CellMetadata) -> bool {
    let values = metadata_values_for_field(directive.field, metadata);
    values.iter().any(|value| {
        value_matches_directive(value, directive)
            || directive
                .terms
                .iter()
                .filter(|term| term.len() >= 3)
                .any(|term| tokenize(value).contains(term))
    })
}

fn metadata_values_for_field(field: QueryScopeField, metadata: &CellMetadata) -> Vec<&str> {
    match field {
        QueryScopeField::Source => vec![
            metadata.source.as_deref(),
            metadata
                .source_ref
                .as_ref()
                .map(|source| source.source_id.as_str()),
            metadata.path.as_deref(),
            metadata.title.as_deref(),
            Some(metadata.body_text.as_str()),
        ],
        QueryScopeField::Scope => vec![
            Some(metadata.scope.as_str()),
            metadata.topic.as_deref(),
            metadata.path.as_deref(),
            metadata.title.as_deref(),
            Some(metadata.body_text.as_str()),
        ],
        QueryScopeField::Project => vec![
            metadata.project.as_deref(),
            metadata.document_id.as_deref(),
            metadata.title.as_deref(),
            metadata.path.as_deref(),
            Some(metadata.body_text.as_str()),
        ],
        QueryScopeField::Team => vec![
            metadata.entity.as_deref(),
            metadata.owner.as_deref(),
            metadata.project.as_deref(),
            metadata.path.as_deref(),
            metadata.title.as_deref(),
            Some(metadata.body_text.as_str()),
        ],
        QueryScopeField::Owner => vec![
            metadata.owner.as_deref(),
            metadata.entity.as_deref(),
            metadata.title.as_deref(),
            Some(metadata.body_text.as_str()),
        ],
        QueryScopeField::Topic => vec![
            metadata.topic.as_deref(),
            metadata.section.as_deref(),
            metadata.title.as_deref(),
            metadata.path.as_deref(),
            Some(metadata.body_text.as_str()),
        ],
        QueryScopeField::Entity => vec![
            metadata.entity.as_deref(),
            metadata.project.as_deref(),
            metadata.owner.as_deref(),
            metadata.title.as_deref(),
            Some(metadata.body_text.as_str()),
        ],
    }
    .into_iter()
    .flatten()
    .collect()
}

fn value_matches_directive(value: &str, directive: &QueryScopeDirective) -> bool {
    let haystack = normalize_for_match(value);
    let needle = normalize_for_match(&directive.value);
    if needle.len() >= 3 && haystack.contains(&needle) {
        return true;
    }
    let value_terms = tokenize(value).into_iter().collect::<BTreeSet<_>>();
    let hits = directive
        .terms
        .iter()
        .filter(|term| value_terms.contains(*term))
        .count();
    hits >= directive.terms.len().clamp(1, 2)
}
