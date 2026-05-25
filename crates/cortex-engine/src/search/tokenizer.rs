pub fn tokenize(text: &str) -> Vec<String> {
    text.split(|value: char| !value.is_alphanumeric())
        .filter_map(normalize_term)
        .filter(|term| !is_stopword(term))
        .collect()
}

fn normalize_term(term: &str) -> Option<String> {
    let normalized = term
        .chars()
        .flat_map(char::to_lowercase)
        .collect::<String>();
    (!normalized.is_empty()).then_some(normalized)
}

fn is_stopword(term: &str) -> bool {
    matches!(
        term,
        "a" | "an"
            | "and"
            | "the"
            | "or"
            | "of"
            | "to"
            | "in"
            | "и"
            | "в"
            | "на"
            | "для"
            | "мен"
            | "және"
    )
}
