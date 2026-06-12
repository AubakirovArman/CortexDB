use std::collections::BTreeSet;

use super::super::analyze_search_query;
use super::builder::requirement_tokens;
use super::normalize::{clean_part, clean_token, is_question_word};

pub(super) fn precise_anchors(question: &str) -> Vec<String> {
    let mut anchors = BTreeSet::new();
    for anchor in analyze_search_query(question).anchors {
        insert_anchor(&mut anchors, anchor.text);
    }
    for phrase in quoted_phrases(question) {
        insert_anchor(&mut anchors, phrase);
    }
    for phrase in capitalized_phrases(question) {
        insert_anchor(&mut anchors, phrase);
    }
    for token in question.split_whitespace().map(clean_token) {
        if is_capitalized_term(&token)
            || is_all_caps_anchor(&token)
            || is_numeric_anchor(&token)
            || is_path_like(&token)
        {
            insert_anchor(&mut anchors, token);
        }
    }
    let mut values = anchors.into_iter().collect::<Vec<_>>();
    values.sort_by_key(|value| (usize::MAX - value.len(), value.clone()));
    values
}

fn quoted_phrases(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut quote = None;
    let mut current = String::new();
    for ch in text.chars() {
        if matches!(ch, '"' | '`') {
            if quote == Some(ch) {
                let phrase = current.trim();
                if !phrase.is_empty() {
                    out.push(phrase.to_owned());
                }
                current.clear();
                quote = None;
            } else if quote.is_none() {
                quote = Some(ch);
                current.clear();
            } else {
                current.push(ch);
            }
        } else if quote.is_some() {
            current.push(ch);
        }
    }
    out
}

fn capitalized_phrases(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = Vec::new();
    for raw in text.split_whitespace() {
        let token = clean_token(raw);
        if token.is_empty() {
            flush_capitalized_phrase(&mut out, &mut current);
            continue;
        }
        if is_capitalized_term(&token) && !is_question_word(&token) {
            current.push(token);
            if current.len() >= 5 {
                flush_capitalized_phrase(&mut out, &mut current);
            }
        } else {
            flush_capitalized_phrase(&mut out, &mut current);
        }
    }
    flush_capitalized_phrase(&mut out, &mut current);
    out
}

fn flush_capitalized_phrase(out: &mut Vec<String>, current: &mut Vec<String>) {
    if current.len() >= 2 {
        out.push(current.join(" "));
    }
    current.clear();
}

fn insert_anchor(anchors: &mut BTreeSet<String>, value: String) {
    let cleaned = clean_part(&value);
    if cleaned.len() < 2 || is_question_word(&cleaned) {
        return;
    }
    if requirement_tokens(&cleaned).is_empty() {
        return;
    }
    anchors.insert(cleaned);
}

fn is_capitalized_term(token: &str) -> bool {
    token
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_uppercase())
        || is_all_caps_anchor(token)
}

fn is_all_caps_anchor(token: &str) -> bool {
    let letters = token.chars().filter(|ch| ch.is_ascii_alphabetic()).count();
    letters >= 2
        && token
            .chars()
            .filter(|ch| ch.is_ascii_alphabetic())
            .all(|ch| ch.is_ascii_uppercase())
}

fn is_numeric_anchor(token: &str) -> bool {
    token.chars().any(|ch| ch.is_ascii_digit())
        && token
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '%' | '-' | ':' | '_'))
}

fn is_path_like(token: &str) -> bool {
    token.contains('/')
        || token
            .rsplit_once('.')
            .is_some_and(|(_, ext)| ext.len() <= 8)
}
