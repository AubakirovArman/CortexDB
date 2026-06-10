use std::collections::{BTreeMap, BTreeSet};

use super::tokenize;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchQueryUnderstanding {
    pub anchors: Vec<QueryAnchor>,
    pub source_hints: Vec<String>,
    pub weighted_terms: BTreeMap<String, u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryAnchor {
    pub kind: QueryAnchorKind,
    pub text: String,
    pub terms: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueryAnchorKind {
    TicketId,
    PullRequest,
    FilePath,
    Version,
    Date,
    Number,
    QuotedPhrase,
}

pub fn analyze_search_query(query: &str) -> SearchQueryUnderstanding {
    let anchors = extract_anchors(query);
    let source_hints = extract_source_hints(query);
    let weighted_terms = weighted_query_terms(query, &anchors);
    SearchQueryUnderstanding {
        anchors,
        source_hints,
        weighted_terms,
    }
}

pub(crate) fn weighted_query_terms(query: &str, anchors: &[QueryAnchor]) -> BTreeMap<String, u32> {
    let mut terms = BTreeMap::new();
    for term in tokenize(query) {
        add_weighted_term(&mut terms, &term, 4);
        for expansion in query_expansions(&term) {
            add_weighted_term(&mut terms, expansion, 1);
        }
    }
    for anchor in anchors {
        for term in &anchor.terms {
            add_weighted_term(&mut terms, term, anchor_weight(anchor.kind));
        }
    }
    terms
}

fn extract_anchors(query: &str) -> Vec<QueryAnchor> {
    let mut anchors = Vec::new();
    for phrase in quoted_phrases(query) {
        push_anchor(&mut anchors, QueryAnchorKind::QuotedPhrase, phrase);
    }
    for raw in query.split_whitespace() {
        let token = clean_token(raw);
        if token.is_empty() {
            continue;
        }
        if is_ticket_id(&token) {
            push_anchor(&mut anchors, QueryAnchorKind::TicketId, token);
        } else if is_pull_request(&token) {
            push_anchor(&mut anchors, QueryAnchorKind::PullRequest, token);
        } else if is_date(&token) {
            push_anchor(&mut anchors, QueryAnchorKind::Date, token);
        } else if is_version(&token) {
            push_anchor(&mut anchors, QueryAnchorKind::Version, token);
        } else if is_file_path(&token) {
            push_anchor(&mut anchors, QueryAnchorKind::FilePath, token);
        } else if is_number(&token) {
            push_anchor(&mut anchors, QueryAnchorKind::Number, token);
        }
    }
    anchors
}

fn push_anchor(anchors: &mut Vec<QueryAnchor>, kind: QueryAnchorKind, text: String) {
    let terms = tokenize(&text);
    if terms.is_empty()
        || anchors
            .iter()
            .any(|anchor| anchor.kind == kind && anchor.text == text)
    {
        return;
    }
    anchors.push(QueryAnchor { kind, text, terms });
}

fn quoted_phrases(query: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut quote: Option<char> = None;
    let mut current = String::new();
    for ch in query.chars() {
        if matches!(ch, '"' | '\'') {
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

fn extract_source_hints(query: &str) -> Vec<String> {
    let mut hints = BTreeSet::new();
    for term in tokenize(query) {
        if let Some(source) = source_hint(&term) {
            hints.insert(source.to_owned());
        }
    }
    hints.into_iter().collect()
}

fn source_hint(term: &str) -> Option<&'static str> {
    match term {
        "slack" | "channel" | "thread" => Some("slack"),
        "gmail" | "email" | "mail" | "inbox" => Some("gmail"),
        "jira" | "ticket" | "issue" => Some("jira"),
        "github" | "repo" | "repository" | "pull" | "pr" => Some("github"),
        "confluence" | "wiki" | "page" => Some("confluence"),
        "drive" | "file" | "spreadsheet" | "sheet" => Some("drive"),
        "hubspot" | "crm" => Some("hubspot"),
        "fireflies" | "meeting" | "transcript" => Some("fireflies"),
        "linear" => Some("linear"),
        _ => None,
    }
}

fn query_expansions(term: &str) -> &'static [&'static str] {
    match term {
        "blocked" | "blocker" | "blockers" => &["risk", "dependency", "delayed", "waiting"],
        "owner" | "owns" => &["assignee", "assigned", "responsible", "dri", "lead"],
        "deadline" | "due" => &["eta", "date", "timeline"],
        "policy" => &["guideline", "rule", "requirement", "procedure"],
        "revenue" => &["income", "sales", "arr", "mrr"],
        "customer" => &["client", "account"],
        "incident" => &["outage", "postmortem", "root", "cause"],
        "migration" => &["upgrade", "rollout"],
        "security" => &["auth", "permission", "rbac", "risk"],
        "launch" => &["release", "rollout", "ga"],
        _ => &[],
    }
}

fn add_weighted_term(terms: &mut BTreeMap<String, u32>, term: &str, weight: u32) {
    if term.is_empty() {
        return;
    }
    *terms.entry(term.to_owned()).or_default() += weight.max(1);
}

fn anchor_weight(kind: QueryAnchorKind) -> u32 {
    match kind {
        QueryAnchorKind::TicketId | QueryAnchorKind::PullRequest | QueryAnchorKind::FilePath => 12,
        QueryAnchorKind::QuotedPhrase => 8,
        QueryAnchorKind::Version | QueryAnchorKind::Date | QueryAnchorKind::Number => 6,
    }
}

fn clean_token(raw: &str) -> String {
    raw.trim_matches(|ch: char| {
        matches!(
            ch,
            ',' | ';' | ':' | ')' | '(' | '[' | ']' | '{' | '}' | '"' | '\''
        )
    })
    .to_owned()
}

fn is_ticket_id(token: &str) -> bool {
    let Some((prefix, suffix)) = token.split_once('-') else {
        return false;
    };
    prefix.len() >= 2
        && prefix.len() <= 16
        && prefix
            .chars()
            .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit())
        && suffix.chars().all(|ch| ch.is_ascii_digit())
}

fn is_pull_request(token: &str) -> bool {
    token
        .strip_prefix('#')
        .is_some_and(|suffix| !suffix.is_empty() && suffix.chars().all(|ch| ch.is_ascii_digit()))
}

fn is_file_path(token: &str) -> bool {
    let has_path_separator = token.contains('/') || token.contains('\\');
    let has_extension = token.rsplit_once('.').is_some_and(|(_, ext)| {
        (1..=8).contains(&ext.len()) && ext.chars().all(char::is_alphanumeric)
    });
    (has_path_separator || has_extension) && token.chars().any(char::is_alphabetic)
}

fn is_version(token: &str) -> bool {
    let value = token.strip_prefix('v').or_else(|| token.strip_prefix('V'));
    value.is_some_and(|rest| {
        rest.contains('.')
            && rest.chars().any(|ch| ch.is_ascii_digit())
            && rest
                .chars()
                .all(|ch| ch.is_ascii_digit() || matches!(ch, '.' | '-' | '_'))
    })
}

fn is_date(token: &str) -> bool {
    let bytes = token.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, value)| matches!(index, 4 | 7) || value.is_ascii_digit())
}

fn is_number(token: &str) -> bool {
    token.chars().any(|ch| ch.is_ascii_digit())
        && token
            .chars()
            .all(|ch| ch.is_ascii_digit() || matches!(ch, '.' | ',' | '%' | '$' | '€' | '₸'))
}

#[cfg(test)]
mod tests {
    use super::{analyze_search_query, QueryAnchorKind};

    #[test]
    fn extracts_enterprise_anchors_from_question_text_only() {
        let analyzed = analyze_search_query(
            "Which GitHub PR #42 fixed AUTH-123 in src/auth/login.rs for v2.3.0 on 2026-04-12?",
        );

        assert!(analyzed
            .anchors
            .iter()
            .any(|anchor| anchor.kind == QueryAnchorKind::PullRequest && anchor.text == "#42"));
        assert!(analyzed
            .anchors
            .iter()
            .any(|anchor| anchor.kind == QueryAnchorKind::TicketId && anchor.text == "AUTH-123"));
        assert!(analyzed
            .anchors
            .iter()
            .any(|anchor| anchor.kind == QueryAnchorKind::FilePath));
        assert!(analyzed.source_hints.contains(&"github".to_owned()));
    }

    #[test]
    fn expands_enterprise_synonyms_without_gold_labels() {
        let analyzed = analyze_search_query("Who owns the blocked launch?");

        assert!(analyzed.weighted_terms.contains_key("assignee"));
        assert!(analyzed.weighted_terms.contains_key("dependency"));
        assert!(analyzed.weighted_terms.contains_key("release"));
    }
}
