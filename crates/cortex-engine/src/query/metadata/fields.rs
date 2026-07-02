use std::collections::BTreeMap;

use crate::search::{tokenize, TextAnalyzer};

pub(crate) fn non_empty(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

/// Frozen lexical field-weight table (A7.1): the single source of truth for how
/// much each field contributes to BM25 in both the engine lexical path and the
/// benchmark retrieval index (which imports this instead of keeping a private
/// copy). Changing a weight is a ranking change and must go through the C3-1
/// frozen-weights protocol.
pub fn lexical_field_weight(field: &str) -> u32 {
    match field {
        "title" => 8,
        "table" => 6,
        "path" => 5,
        "entity" => 4,
        "chunk" => 2,
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::lexical_field_weight;

    #[test]
    fn frozen_lexical_field_weight_table() {
        // These weights are the benchmark parity reference; a change is a ranking
        // change and must go through the C3-1 protocol. The bench crate imports
        // this exact function, so bench/engine parity is by construction.
        assert_eq!(lexical_field_weight("title"), 8);
        assert_eq!(lexical_field_weight("table"), 6);
        assert_eq!(lexical_field_weight("path"), 5);
        assert_eq!(lexical_field_weight("entity"), 4);
        assert_eq!(lexical_field_weight("chunk"), 2);
        assert_eq!(lexical_field_weight("body"), 1);
        assert_eq!(lexical_field_weight("anything-else"), 1);
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
