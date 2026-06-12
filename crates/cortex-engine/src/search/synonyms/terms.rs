use std::collections::BTreeSet;

use super::super::tokenize;

pub(super) fn document_terms(document: &str, max_terms: usize) -> Vec<String> {
    let document = document_body_for_synonyms(document);
    let mut seen = BTreeSet::new();
    let mut terms = Vec::new();
    for term in tokenize(document)
        .into_iter()
        .map(|term| normalize_term(&term))
        .filter(|term| is_dictionary_term(term))
    {
        if !seen.insert(term.clone()) {
            continue;
        }
        terms.push(term);
        if terms.len() >= max_terms.max(1) {
            break;
        }
    }
    terms
}

fn document_body_for_synonyms(document: &str) -> &str {
    document
        .split_once("\n\n")
        .map(|(_, body)| body)
        .unwrap_or(document)
}

pub(super) fn document_abbreviation_pairs(document: &str) -> BTreeSet<(String, String)> {
    let document = document_body_for_synonyms(document);
    let mut pairs = BTreeSet::new();
    let mut search_start = 0;
    while let Some(open_offset) = document[search_start..].find('(') {
        let open = search_start + open_offset;
        let Some(close_offset) = document[open + 1..].find(')') else {
            break;
        };
        let close = open + 1 + close_offset;
        let before = &document[..open];
        let inside = &document[open + 1..close];
        collect_parenthetical_abbreviation_pairs(before, inside, &mut pairs);
        search_start = close + 1;
    }
    pairs
}

fn collect_parenthetical_abbreviation_pairs(
    before: &str,
    inside: &str,
    pairs: &mut BTreeSet<(String, String)>,
) {
    let inside_words = raw_words(inside);
    if inside_words.len() == 1 {
        if let Some(abbreviation) = normalized_abbreviation(&inside_words[0]) {
            if let Some(phrase_terms) =
                matching_phrase_suffix_terms(&raw_words(before), &abbreviation)
            {
                insert_abbreviation_pairs(pairs, &abbreviation, phrase_terms);
            }
        }
    }

    let before_words = raw_words(before);
    let Some(previous_word) = before_words.last() else {
        return;
    };
    let Some(abbreviation) = normalized_abbreviation(previous_word) else {
        return;
    };
    if let Some(phrase_terms) = matching_phrase_terms(&inside_words, &abbreviation) {
        insert_abbreviation_pairs(pairs, &abbreviation, phrase_terms);
    }
}

fn insert_abbreviation_pairs(
    pairs: &mut BTreeSet<(String, String)>,
    abbreviation: &str,
    phrase_terms: Vec<String>,
) {
    for term in phrase_terms {
        if term != abbreviation {
            pairs.insert((abbreviation.to_owned(), term));
        }
    }
}

fn raw_words(text: &str) -> Vec<String> {
    text.split(|value: char| !value.is_alphanumeric())
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn normalized_abbreviation(word: &str) -> Option<String> {
    let compact = word
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>();
    let letters = compact
        .chars()
        .filter(|ch| ch.is_ascii_alphabetic())
        .count();
    if compact.len() < 2
        || compact.len() > 12
        || letters < 2
        || compact.chars().any(|ch| ch.is_ascii_lowercase())
    {
        return None;
    }
    Some(compact.to_ascii_lowercase())
}

fn matching_phrase_suffix_terms(words: &[String], abbreviation: &str) -> Option<Vec<String>> {
    let letters = abbreviation_letters(abbreviation);
    if words.len() < letters.len() {
        return None;
    }
    let start = words.len() - letters.len();
    matching_phrase_terms(&words[start..], abbreviation)
}

fn matching_phrase_terms(words: &[String], abbreviation: &str) -> Option<Vec<String>> {
    let letters = abbreviation_letters(abbreviation);
    if words.len() < letters.len() {
        return None;
    }
    for window in words.windows(letters.len()) {
        if window
            .iter()
            .filter_map(|word| word.chars().next())
            .map(|ch| ch.to_ascii_lowercase())
            .eq(letters.iter().copied())
        {
            let terms = window
                .iter()
                .map(|word| normalize_term(word))
                .filter(|term| is_dictionary_term(term))
                .collect::<Vec<_>>();
            if !terms.is_empty() {
                return Some(terms);
            }
        }
    }
    None
}

fn abbreviation_letters(abbreviation: &str) -> Vec<char> {
    abbreviation
        .chars()
        .filter(|ch| ch.is_ascii_alphabetic())
        .map(|ch| ch.to_ascii_lowercase())
        .collect()
}

pub(super) fn normalize_term(term: &str) -> String {
    term.to_ascii_lowercase()
}

fn is_dictionary_term(term: &str) -> bool {
    term.len() >= 3
        && term.len() <= 40
        && term.chars().any(|ch| ch.is_ascii_alphabetic())
        && !matches!(
            term,
            "and"
                | "are"
                | "but"
                | "can"
                | "did"
                | "does"
                | "for"
                | "from"
                | "has"
                | "have"
                | "how"
                | "into"
                | "not"
                | "our"
                | "the"
                | "their"
                | "this"
                | "that"
                | "was"
                | "were"
                | "what"
                | "when"
                | "where"
                | "which"
                | "who"
                | "why"
                | "with"
        )
}

pub(super) fn association_score_q16(
    cooccurrence_count: u32,
    left_docs: u32,
    right_docs: u32,
) -> u16 {
    let denominator = left_docs.min(right_docs).max(1);
    ((u64::from(cooccurrence_count) * 65_535) / u64::from(denominator)).min(65_535) as u16
}
