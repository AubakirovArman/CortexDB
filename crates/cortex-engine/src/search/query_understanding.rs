use std::collections::{BTreeMap, BTreeSet};

use super::{frozen_weights, tokenize, TextAnalyzer};

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
    analyze_search_query_with_analyzer(query, &TextAnalyzer::default())
}

pub fn analyze_search_query_with_analyzer(
    query: &str,
    analyzer: &TextAnalyzer,
) -> SearchQueryUnderstanding {
    let anchors = extract_anchors(query);
    let source_hints = extract_source_hints(query, analyzer);
    let weighted_terms = weighted_query_terms_with_analyzer(query, &anchors, analyzer);
    SearchQueryUnderstanding {
        anchors,
        source_hints,
        weighted_terms,
    }
}

pub(crate) fn weighted_query_terms_with_analyzer(
    query: &str,
    anchors: &[QueryAnchor],
    analyzer: &TextAnalyzer,
) -> BTreeMap<String, u32> {
    let mut terms = BTreeMap::new();
    for term in analyzer.tokenize(query) {
        add_weighted_term(&mut terms, &term, frozen_weights::QUERY_BASE_TERM_WEIGHT);
        for expansion in query_expansions(&term) {
            add_weighted_term(
                &mut terms,
                expansion,
                frozen_weights::QUERY_EXPANSION_WEIGHT,
            );
        }
    }
    for expansion in phrase_query_expansions(query) {
        add_weighted_term(
            &mut terms,
            expansion,
            frozen_weights::QUERY_PHRASE_EXPANSION_WEIGHT,
        );
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

fn extract_source_hints(query: &str, analyzer: &TextAnalyzer) -> Vec<String> {
    let mut hints = BTreeSet::new();
    for term in analyzer.tokenize(query) {
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
        "blocked" | "blocker" | "blockers" | "stuck" | "slipped" | "slipping" => &[
            "blocked",
            "blocker",
            "risk",
            "dependency",
            "delayed",
            "waiting",
            "issue",
        ],
        "delay" | "delays" | "delayed" | "late" => {
            &["blocked", "blocker", "risk", "dependency", "waiting"]
        }
        "dependency" | "dependencies" | "waiting" => &["blocked", "blocker", "risk"],
        "owner" | "owns" | "owned" => &["assignee", "assigned", "responsible", "dri", "lead"],
        "assignee" | "assigned" | "responsible" | "dri" | "lead" => &["owner", "owns"],
        "deadline" | "due" => &["eta", "date", "timeline", "target"],
        "eta" | "timeline" | "target" => &["deadline", "due", "date"],
        "policy" => &["guideline", "rule", "requirement", "procedure", "standard"],
        "guideline" | "rule" | "requirement" | "procedure" | "standard" => &["policy"],
        "revenue" => &["income", "sales", "arr", "mrr", "billing"],
        "income" | "sales" | "billing" => &["revenue"],
        "customer" => &["client", "account", "tenant"],
        "client" | "account" | "tenant" => &["customer"],
        "incident" => &["outage", "postmortem", "root", "cause", "sev"],
        "outage" | "postmortem" | "sev" => &["incident"],
        "migration" => &["upgrade", "rollout", "transition"],
        "upgrade" | "transition" => &["migration"],
        "security" => &["auth", "permission", "rbac", "risk", "access"],
        "auth" | "permission" | "permissions" | "rbac" | "access" => &["security"],
        "launch" => &["release", "rollout", "ga"],
        "release" | "ga" => &["launch", "rollout"],
        "overview" | "summary" => &["mission", "charter", "about", "strategy"],
        "mission" | "charter" | "strategy" | "vision" => &["overview", "summary", "about"],
        _ => &[],
    }
}

fn phrase_query_expansions(query: &str) -> Vec<&'static str> {
    let query = query.to_ascii_lowercase();
    let mut expansions = BTreeSet::new();
    if query.contains("high level")
        || query.contains("big picture")
        || query.contains("company overview")
        || query.contains("what does")
    {
        expansions.extend(["overview", "summary", "mission", "charter", "about"]);
    }
    if query.contains("single sign") || query.contains("sign on") {
        expansions.extend(["sso", "auth", "authentication", "security"]);
    }
    if query.contains("role based") || query.contains("access control") {
        expansions.extend(["rbac", "permission", "permissions", "security"]);
    }
    if query.contains("go live") || query.contains("general availability") {
        expansions.extend(["launch", "release", "rollout", "ga"]);
    }
    expansions.into_iter().collect()
}

fn add_weighted_term(terms: &mut BTreeMap<String, u32>, term: &str, weight: u32) {
    if term.is_empty() {
        return;
    }
    *terms.entry(term.to_owned()).or_default() += weight.max(1);
}

fn anchor_weight(kind: QueryAnchorKind) -> u32 {
    match kind {
        QueryAnchorKind::TicketId | QueryAnchorKind::PullRequest | QueryAnchorKind::FilePath => {
            frozen_weights::QUERY_ANCHOR_STRONG_WEIGHT
        }
        QueryAnchorKind::QuotedPhrase => frozen_weights::QUERY_ANCHOR_PHRASE_WEIGHT,
        QueryAnchorKind::Version | QueryAnchorKind::Date | QueryAnchorKind::Number => {
            frozen_weights::QUERY_ANCHOR_NUMERIC_WEIGHT
        }
    }
}

fn clean_token(raw: &str) -> String {
    raw.trim_matches(|ch: char| {
        matches!(
            ch,
            ',' | ';' | ':' | ')' | '(' | '[' | ']' | '{' | '}' | '"' | '\'' | '?' | '!'
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
mod tests;
