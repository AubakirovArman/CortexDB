use crate::search::tokenize;

pub fn generate_selection_reason(
    _score: u32,
    base_bm25: u32,
    source_trust: u32,
    penalty: u32,
) -> String {
    if penalty > 10000 {
        "Selected due to matching terms, but penalized for high redundancy with other context"
            .to_owned()
    } else if source_trust > 40000 {
        "Selected due to high provenance source trust and relevant query terms"
            .to_owned()
    } else if base_bm25 > 8000 {
        "Selected due to strong keyword relevance match"
            .to_owned()
    } else {
        "Selected as relevant background context"
            .to_owned()
    }
}

pub(crate) fn extract_query_terms(aql: &str) -> Vec<String> {
    let mut terms = Vec::new();
    let mut in_quotes = false;
    let mut current = String::new();
    for character in aql.chars() {
        if character == '"' {
            if in_quotes {
                terms.extend(tokenize(&current));
                current.clear();
            }
            in_quotes = !in_quotes;
        } else if in_quotes {
            current.push(character);
        }
    }
    terms
}
