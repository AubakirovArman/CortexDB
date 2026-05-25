use crate::query::CellMetadata;
use crate::search::tokenize;

pub(crate) fn contradiction_match(payload: &[u8], fact_terms: &[String]) -> Option<u32> {
    if fact_terms.is_empty() {
        return None;
    }
    structured_contradiction_match(payload, fact_terms)
        .or_else(|| natural_language_contradiction_match(payload, fact_terms))
}

pub(crate) fn contradiction_text_matches(value: &str, fact_terms: &[String]) -> bool {
    if fact_terms.is_empty() {
        return false;
    }
    let contradicts_terms = tokenize(value);
    let matched_terms = fact_terms
        .iter()
        .filter(|term| contradicts_terms.contains(term))
        .count();
    matched_terms == fact_terms.len()
}

pub(crate) fn contradiction_facts(payload: &[u8]) -> Vec<String> {
    body_text(payload)
        .lines()
        .filter_map(|line| line.trim().strip_prefix("contradicts="))
        .map(str::trim)
        .filter(|fact| !fact.is_empty())
        .map(str::to_owned)
        .collect()
}

pub(crate) fn tokenize_support_text(payload: &[u8]) -> Vec<String> {
    tokenize(&body_text_without_markers(payload))
}

fn structured_contradiction_match(payload: &[u8], fact_terms: &[String]) -> Option<u32> {
    contradiction_facts(payload).into_iter().find_map(|fact| {
        contradiction_text_matches(&fact, fact_terms).then_some(fact_terms.len() as u32)
    })
}

fn natural_language_contradiction_match(payload: &[u8], fact_terms: &[String]) -> Option<u32> {
    let body = body_text_without_markers(payload);
    body.split(is_clause_boundary).find_map(|clause| {
        let tokens = tokenize(clause);
        negated_fact_score(&tokens, fact_terms).or_else(|| antonym_fact_score(&tokens, fact_terms))
    })
}

fn negated_fact_score(tokens: &[String], fact_terms: &[String]) -> Option<u32> {
    let matched_terms = fact_terms
        .iter()
        .filter(|term| tokens.contains(term))
        .count();
    if matched_terms != fact_terms.len() {
        return None;
    }
    tokens
        .iter()
        .enumerate()
        .any(|(index, token)| negates_fact_term(tokens, index, token, fact_terms))
        .then_some(matched_terms as u32)
}

fn negates_fact_term(
    tokens: &[String],
    negation_index: usize,
    token: &str,
    fact_terms: &[String],
) -> bool {
    if is_prefix_negation(token) && is_not_only(tokens, negation_index) {
        return false;
    }
    fact_terms.iter().any(|term| {
        tokens.iter().enumerate().any(|(term_index, value)| {
            value == term
                && ((is_prefix_negation(token)
                    && negation_index < term_index
                    && term_index - negation_index <= 3)
                    || (is_postfix_negation(token)
                        && term_index < negation_index
                        && negation_index - term_index <= 4))
        })
    })
}

fn antonym_fact_score(tokens: &[String], fact_terms: &[String]) -> Option<u32> {
    fact_terms.iter().find_map(|term| {
        let antonym_hit = antonyms(term)
            .iter()
            .any(|antonym| tokens.iter().any(|token| token.as_str() == *antonym));
        if !antonym_hit {
            return None;
        }
        let matched_other_terms = fact_terms
            .iter()
            .filter(|candidate| *candidate != term && tokens.contains(candidate))
            .count();
        (matched_other_terms + 1 == fact_terms.len()).then_some(fact_terms.len() as u32)
    })
}

fn body_text_without_markers(payload: &[u8]) -> String {
    body_text(payload)
        .lines()
        .filter(|line| !line.trim().starts_with("contradicts="))
        .collect::<Vec<_>>()
        .join("\n")
}

fn body_text(payload: &[u8]) -> String {
    CellMetadata::from_payload(payload).body_text
}

fn is_clause_boundary(value: char) -> bool {
    matches!(value, '.' | '!' | '?' | ';' | '\n')
}

fn is_prefix_negation(value: &str) -> bool {
    matches!(
        value,
        "not" | "no" | "never" | "without" | "denies" | "denied" | "не" | "нет" | "жоқ" | "емес"
    )
}

fn is_postfix_negation(value: &str) -> bool {
    matches!(
        value,
        "false" | "incorrect" | "wrong" | "ложно" | "неверно" | "неправда"
    )
}

fn is_not_only(tokens: &[String], index: usize) -> bool {
    tokens.get(index).is_some_and(|term| term == "not")
        && tokens.get(index + 1).is_some_and(|term| term == "only")
}

fn antonyms(term: &str) -> &'static [&'static str] {
    match term {
        "approved" | "approve" | "accepted" => &[
            "rejected",
            "rejects",
            "denied",
            "declined",
            "refused",
            "cancelled",
            "canceled",
            "revoked",
            "отклонено",
            "отклонен",
            "отклонена",
        ],
        "ready" => &["blocked", "failed", "unready", "notready"],
        "valid" | "true" => &["invalid", "false", "incorrect"],
        "increased" | "increase" => &["decreased", "reduced", "lowered"],
        "decreased" | "reduced" => &["increased", "raised", "grew"],
        "open" => &["closed", "shut"],
        "active" => &["inactive", "disabled"],
        _ => &[],
    }
}
