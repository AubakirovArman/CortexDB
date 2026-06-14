use std::collections::BTreeMap;

use crate::search::{tokenize, TextAnalyzer};

pub(crate) fn non_empty(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

pub(crate) fn lexical_field_weight(field: &str) -> u32 {
    match field {
        "title" => 8,
        "table" => 6,
        "path" => 5,
        "entity" => 4,
        "chunk" => 2,
        _ => 1,
    }
}

pub(super) fn add_field_terms(
    fields: &mut BTreeMap<String, BTreeMap<String, u32>>,
    field: &str,
    text: &str,
) {
    for term in tokenize(text) {
        *fields
            .entry(field.to_owned())
            .or_default()
            .entry(term)
            .or_default() += 1;
    }
}

pub(super) fn add_field_terms_with_analyzer(
    fields: &mut BTreeMap<String, BTreeMap<String, u32>>,
    field: &str,
    text: &str,
    analyzer: &TextAnalyzer,
) {
    for term in analyzer.tokenize(text) {
        *fields
            .entry(field.to_owned())
            .or_default()
            .entry(term)
            .or_default() += 1;
    }
}
