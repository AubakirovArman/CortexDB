use std::collections::BTreeSet;

use super::normalize::{clean_token, contains_word_or_phrase, is_name_like, is_scope_stopword};
use super::types::QueryScopeField;

pub(super) fn lexicon_scope_hints(
    query: &str,
) -> Vec<(QueryScopeField, String, u16, &'static str)> {
    let lower = query.to_ascii_lowercase();
    let mut out = Vec::new();
    collect_department_hints(&lower, &mut out);
    collect_topic_hints(&lower, &mut out);
    out
}

pub(super) fn project_name_hints(query: &str) -> Vec<String> {
    let mut values = BTreeSet::new();
    let tokens = query
        .split_whitespace()
        .map(clean_token)
        .collect::<Vec<_>>();
    for (index, token) in tokens.iter().enumerate() {
        let lower = token.to_ascii_lowercase();
        if matches!(
            lower.as_str(),
            "project" | "initiative" | "program" | "rollout" | "launch" | "migration"
        ) {
            push_neighbor_name(&tokens, index + 1, &mut values);
            if index > 0 {
                push_neighbor_name(&tokens, index - 1, &mut values);
            }
        }
    }
    for phrase in capitalized_phrases(query) {
        if !is_scope_stopword(&phrase.to_ascii_lowercase()) {
            values.insert(phrase);
        }
    }
    values.into_iter().collect()
}

pub(super) fn team_name_hints(query: &str) -> Vec<String> {
    let mut values = BTreeSet::new();
    let tokens = query
        .split_whitespace()
        .map(clean_token)
        .collect::<Vec<_>>();
    for (index, token) in tokens.iter().enumerate() {
        let lower = token.to_ascii_lowercase();
        if matches!(lower.as_str(), "team" | "squad" | "group" | "department") {
            if index > 0 {
                let previous = &tokens[index - 1];
                if is_name_like(previous) || previous.chars().all(|ch| ch.is_ascii_uppercase()) {
                    values.insert(previous.clone());
                }
            }
            push_neighbor_name(&tokens, index + 1, &mut values);
        }
    }
    values.into_iter().collect()
}

fn collect_department_hints(
    lower: &str,
    out: &mut Vec<(QueryScopeField, String, u16, &'static str)>,
) {
    for (needle, value) in [
        ("engineering", "engineering"),
        ("platform", "platform"),
        ("infrastructure", "infrastructure"),
        ("infra", "infrastructure"),
        ("security", "security"),
        ("sales", "sales"),
        ("revenue", "revenue"),
        ("revops", "revops"),
        ("customer success", "customer_success"),
        ("support", "support"),
        ("finance", "finance"),
        ("billing", "billing"),
        ("legal", "legal"),
        ("marketing", "marketing"),
        ("go-to-market", "gtm"),
        ("gtm", "gtm"),
        ("product", "product"),
        ("design", "design"),
        ("operations", "operations"),
        ("ops", "operations"),
        ("data", "data"),
        ("analytics", "analytics"),
    ] {
        if contains_word_or_phrase(lower, needle) {
            out.push((
                QueryScopeField::Scope,
                value.to_owned(),
                44_000,
                "department_scope_hint",
            ));
        }
    }
}

fn collect_topic_hints(lower: &str, out: &mut Vec<(QueryScopeField, String, u16, &'static str)>) {
    for (needle, value) in [
        ("onboarding", "onboarding"),
        ("launch", "launch"),
        ("rollout", "rollout"),
        ("migration", "migration"),
        ("incident", "incident"),
        ("postmortem", "postmortem"),
        ("pricing", "pricing"),
        ("sso", "sso"),
        ("single sign", "sso"),
        ("auth", "auth"),
        ("authentication", "auth"),
        ("rbac", "rbac"),
        ("permissions", "permissions"),
        ("retention", "retention"),
        ("renewal", "renewal"),
        ("roadmap", "roadmap"),
        ("policy", "policy"),
    ] {
        if contains_word_or_phrase(lower, needle) {
            out.push((
                QueryScopeField::Topic,
                value.to_owned(),
                42_000,
                "topic_scope_hint",
            ));
        }
    }
}

fn push_neighbor_name(tokens: &[String], index: usize, values: &mut BTreeSet<String>) {
    if let Some(token) = tokens.get(index) {
        if is_name_like(token) {
            values.insert(token.clone());
        }
    }
}

fn capitalized_phrases(query: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut current = Vec::new();
    for token in query.split_whitespace().map(clean_token) {
        if is_name_like(&token) {
            current.push(token);
        } else {
            push_capitalized_phrase(&mut values, &mut current);
        }
    }
    push_capitalized_phrase(&mut values, &mut current);
    values
}

fn push_capitalized_phrase(values: &mut Vec<String>, current: &mut Vec<String>) {
    if current.is_empty() {
        return;
    }
    let phrase = current.join(" ");
    current.clear();
    if phrase
        .split_whitespace()
        .all(|term| is_scope_stopword(&term.to_ascii_lowercase()))
    {
        return;
    }
    values.push(phrase);
}
