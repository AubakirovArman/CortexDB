use std::collections::{BTreeMap, BTreeSet};

use super::overview::is_overview_candidate_path;

pub(super) fn source_allowed_map(
    candidate_to_source_type: &BTreeMap<u32, String>,
) -> BTreeMap<String, BTreeSet<u32>> {
    let mut values = BTreeMap::<String, BTreeSet<u32>>::new();
    for (candidate, source_type) in candidate_to_source_type {
        values
            .entry(source_type.clone())
            .or_default()
            .insert(*candidate);
    }
    values
}

pub(super) fn candidate_doc_maps(
    uuid_index: &BTreeMap<String, String>,
    doc_lengths: &BTreeMap<u32, u32>,
) -> (
    BTreeMap<u32, String>,
    BTreeMap<u32, String>,
    BTreeMap<u32, String>,
) {
    let documents = uuid_index
        .iter()
        .map(|(doc_id, path)| (doc_id.clone(), source_type(path)))
        .collect::<Vec<_>>();
    let document_paths = uuid_index
        .iter()
        .map(|(doc_id, path)| (doc_id.clone(), path.clone()))
        .collect::<Vec<_>>();
    let mut candidate_to_doc_id = BTreeMap::new();
    let mut candidate_to_source_type = BTreeMap::new();
    let mut candidate_to_rel_path = BTreeMap::new();
    for candidate in doc_lengths.keys() {
        if let Some((doc_id, source_type)) = (|| {
            let ordinal = usize::try_from(*candidate).ok()?.checked_sub(1)?;
            documents.get(ordinal)
        })() {
            candidate_to_doc_id.insert(*candidate, doc_id.clone());
            candidate_to_source_type.insert(*candidate, source_type.clone());
        }
        if let Some((_, rel_path)) = (|| {
            let ordinal = usize::try_from(*candidate).ok()?.checked_sub(1)?;
            document_paths.get(ordinal)
        })() {
            let rel_path = rel_path.to_lowercase();
            if is_overview_candidate_path(&rel_path) {
                candidate_to_rel_path.insert(*candidate, rel_path);
            }
        }
    }
    (
        candidate_to_doc_id,
        candidate_to_source_type,
        candidate_to_rel_path,
    )
}

fn source_type(path: &str) -> String {
    path.split('/').next().unwrap_or("unknown").to_owned()
}

pub(super) fn average_len_q16(doc_lengths: &BTreeMap<u32, u32>, allowed: &BTreeSet<u32>) -> u64 {
    let mut count = 0u64;
    let mut total = 0u64;
    for candidate in allowed {
        if let Some(length) = doc_lengths.get(candidate) {
            count += 1;
            total += u64::from(*length);
        }
    }
    total
        .saturating_mul(65_536)
        .checked_div(count)
        .unwrap_or(65_536)
}

pub(super) fn average_field_len_q16(
    field_doc_lengths: &BTreeMap<String, BTreeMap<u32, u32>>,
    field: &str,
    allowed: &BTreeSet<u32>,
) -> u64 {
    let Some(lengths) = field_doc_lengths.get(field) else {
        return 65_536;
    };
    let mut count = 0u64;
    let mut total = 0u64;
    for (candidate, length) in lengths {
        if allowed.contains(candidate) {
            count += 1;
            total = total.saturating_add(u64::from((*length).max(1)));
        }
    }
    total
        .saturating_mul(65_536)
        .checked_div(count)
        .unwrap_or(65_536)
}

pub(super) fn lexical_field_weight(field: &str) -> u32 {
    match field {
        "title" => 8,
        "table" => 6,
        "path" => 5,
        "entity" => 4,
        "chunk" => 2,
        _ => 1,
    }
}
