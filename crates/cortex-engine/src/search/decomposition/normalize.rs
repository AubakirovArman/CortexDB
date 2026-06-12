use std::collections::BTreeSet;

use super::super::tokenize;

pub(super) fn clean_token(raw: &str) -> String {
    raw.trim_matches(|ch: char| {
        matches!(
            ch,
            ',' | ';' | ':' | ')' | '(' | '[' | ']' | '{' | '}' | '"' | '\'' | '?' | '!'
        )
    })
    .to_owned()
}

pub(super) fn clean_part(raw: &str) -> String {
    raw.trim_matches(|ch: char| matches!(ch, ',' | ';' | ':' | '.' | '?' | '!' | ' '))
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn compact_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(super) fn normalize_for_key(text: &str) -> String {
    tokenize(text).join(" ")
}

pub(super) fn normalize_for_substring(text: &str) -> String {
    text.chars()
        .flat_map(char::to_lowercase)
        .map(|ch| {
            if ch.is_alphanumeric() || matches!(ch, '_' | '.' | '/' | ':' | '%' | '-') {
                ch
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn contains_any_word_or_phrase(text: &str, needles: &[&str]) -> bool {
    let terms = tokenize(text).into_iter().collect::<BTreeSet<_>>();
    needles.iter().any(|needle| {
        if needle.contains(' ') {
            text.contains(needle)
        } else {
            terms.contains(*needle)
        }
    })
}

pub(super) fn unique_texts(values: Vec<String>, limit: usize) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for value in values {
        let key = normalize_for_key(&value);
        if key.is_empty() || !seen.insert(key) {
            continue;
        }
        out.push(value);
        if out.len() >= limit {
            break;
        }
    }
    out
}

pub(super) fn is_question_word(token: &str) -> bool {
    matches!(
        token.to_ascii_lowercase().as_str(),
        "who"
            | "what"
            | "which"
            | "where"
            | "when"
            | "why"
            | "how"
            | "does"
            | "did"
            | "is"
            | "are"
            | "was"
            | "were"
            | "in"
            | "for"
            | "the"
            | "and"
            | "or"
    )
}
