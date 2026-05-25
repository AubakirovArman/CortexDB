use crate::search::tokenize;

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
