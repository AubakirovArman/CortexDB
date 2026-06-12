use super::builder::requirement_tokens;
use super::normalize::{clean_part, compact_whitespace, unique_texts};

pub fn split_subquestions(question: &str) -> Vec<String> {
    let cleaned = compact_whitespace(question);
    let separators = [
        " and what ",
        " and when ",
        " and where ",
        " and which ",
        " and how ",
        " and why ",
        " including ",
        " including the ",
        " along with ",
        " plus ",
    ];
    let mut parts = Vec::new();
    for part in split_on_phrases(&cleaned, &separators) {
        push_part(&mut parts, &part);
    }
    for part in cleaned.split([',', ';']) {
        push_part(&mut parts, part);
    }
    unique_texts(parts, 8)
}

fn split_on_phrases(text: &str, separators: &[&str]) -> Vec<String> {
    let mut parts = vec![text.to_owned()];
    for separator in separators {
        let mut next = Vec::new();
        for part in parts {
            next.extend(split_once_repeated(&part, separator));
        }
        parts = next;
    }
    parts
}

fn split_once_repeated(text: &str, separator: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut rest = text.to_owned();
    loop {
        let lower = rest.to_ascii_lowercase();
        let Some(index) = lower.find(separator) else {
            break;
        };
        let before = rest[..index].to_owned();
        let after_index = index + separator.len();
        let after = rest[after_index..].to_owned();
        parts.push(before);
        rest = after;
    }
    parts.push(rest);
    parts
}

fn push_part(parts: &mut Vec<String>, part: &str) {
    let cleaned = clean_part(part);
    let token_count = requirement_tokens(&cleaned).len();
    if token_count >= 2 {
        parts.push(cleaned);
    }
}
