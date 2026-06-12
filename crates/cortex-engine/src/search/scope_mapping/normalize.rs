use super::super::tokenize;

pub(super) fn clean_scope_value(value: &str) -> String {
    compact_whitespace(value.trim_matches(|ch: char| {
        matches!(
            ch,
            ',' | ';' | ':' | ')' | '(' | '[' | ']' | '{' | '}' | '"' | '\'' | '?' | '!'
        )
    }))
}

pub(super) fn clean_token(raw: &str) -> String {
    raw.trim_matches(|ch: char| {
        matches!(
            ch,
            ',' | ';' | ':' | ')' | '(' | '[' | ']' | '{' | '}' | '"' | '\'' | '?' | '!'
        )
    })
    .to_owned()
}

pub(super) fn normalize_for_match(value: &str) -> String {
    tokenize(value).join(" ")
}

pub(super) fn compact_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(super) fn contains_word_or_phrase(lower: &str, needle: &str) -> bool {
    if needle.contains(' ') || needle.contains('-') {
        return lower.contains(needle);
    }
    lower
        .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
        .any(|word| word == needle)
}

pub(super) fn is_name_like(token: &str) -> bool {
    let mut chars = token.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    token.len() >= 3
        && first.is_ascii_uppercase()
        && chars.any(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit())
}

pub(super) fn is_scope_stopword(term: &str) -> bool {
    matches!(
        term,
        "a" | "an"
            | "and"
            | "are"
            | "can"
            | "did"
            | "do"
            | "does"
            | "for"
            | "from"
            | "give"
            | "how"
            | "is"
            | "list"
            | "me"
            | "of"
            | "on"
            | "or"
            | "show"
            | "tell"
            | "the"
            | "this"
            | "to"
            | "was"
            | "what"
            | "when"
            | "where"
            | "which"
            | "who"
            | "why"
    )
}
