use std::collections::BTreeSet;

use crate::query::CellMetadata;

pub(super) fn search_parent_lookup_keys(metadata: &CellMetadata) -> Vec<String> {
    let mut keys = BTreeSet::new();
    if let Some(parent_id) = &metadata.parent_id {
        keys.insert(parent_id.clone());
    }
    if !is_search_parent_context_metadata(metadata) {
        if let Some(document_id) = &metadata.document_id {
            keys.insert(document_id.clone());
        }
    }
    keys.into_iter().collect()
}

pub(super) fn is_search_parent_context_metadata(metadata: &CellMetadata) -> bool {
    metadata
        .chunk_role
        .as_deref()
        .map(|role| {
            role.eq_ignore_ascii_case("parent")
                || role.eq_ignore_ascii_case("document")
                || role.eq_ignore_ascii_case("summary")
        })
        .unwrap_or(false)
}

pub(super) fn high_level_anchor_score(metadata: &CellMetadata) -> u64 {
    let mut score = 0u64;
    if is_search_parent_context_metadata(metadata) {
        score = score.saturating_add(8_000);
    }
    for value in [
        metadata.title.as_deref(),
        metadata.path.as_deref(),
        metadata.document_id.as_deref(),
        metadata.source.as_deref(),
        Some(metadata.body_text.as_str()),
    ]
    .into_iter()
    .flatten()
    {
        let value = value.to_ascii_lowercase();
        for term in [
            "overview", "summary", "mission", "charter", "about", "strategy", "vision", "company",
        ] {
            if value.contains(term) {
                score = score.saturating_add(2_000);
            }
        }
    }
    score
}

pub(super) fn project_context_score(metadata: &CellMetadata) -> u64 {
    let mut score = 1_000u64;
    if metadata.owner.is_some() {
        score = score.saturating_add(2_000);
    }
    if metadata.status_tag.is_some() {
        score = score.saturating_add(1_500);
    }
    if metadata.event_date.is_some() {
        score = score.saturating_add(1_000);
    }
    if metadata.title.is_some() {
        score = score.saturating_add(500);
    }
    score
}
