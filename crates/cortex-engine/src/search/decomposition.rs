use std::collections::BTreeSet;

use super::{analyze_search_query, tokenize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum QuestionRequirementKind {
    Anchor,
    Slot,
    Subquestion,
    Question,
}

impl QuestionRequirementKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Anchor => "anchor",
            Self::Slot => "slot",
            Self::Subquestion => "subquestion",
            Self::Question => "question",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuestionRequirement {
    pub id: String,
    pub kind: QuestionRequirementKind,
    pub text: String,
    pub tokens: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuestionDecomposition {
    pub question: String,
    pub requirements: Vec<QuestionRequirement>,
    pub anchors: Vec<String>,
    pub slots: Vec<String>,
    pub subquestions: Vec<String>,
    pub multi_requirement: bool,
}

pub fn decompose_enterprise_rag_question(question: &str) -> QuestionDecomposition {
    let question = compact_whitespace(question);
    let mut builder = RequirementBuilder::default();

    for anchor in precise_anchors(&question).into_iter().take(12) {
        builder.push(QuestionRequirementKind::Anchor, anchor);
    }
    for slot in expected_slots(&question) {
        builder.push(QuestionRequirementKind::Slot, slot);
    }
    for subquestion in split_subquestions(&question).into_iter().take(8) {
        builder.push(QuestionRequirementKind::Subquestion, subquestion);
    }
    if builder.is_empty() {
        builder.push(QuestionRequirementKind::Question, question.clone());
    }

    let requirements = builder.finish();
    let anchors = texts_by_kind(&requirements, QuestionRequirementKind::Anchor);
    let slots = texts_by_kind(&requirements, QuestionRequirementKind::Slot);
    let subquestions = texts_by_kind(&requirements, QuestionRequirementKind::Subquestion);
    let non_anchor_count = requirements
        .iter()
        .filter(|item| item.kind != QuestionRequirementKind::Anchor)
        .count();
    let multi_requirement = subquestions.len() > 1 || non_anchor_count > 1;
    QuestionDecomposition {
        question,
        requirements,
        anchors,
        slots,
        subquestions,
        multi_requirement,
    }
}

pub fn covered_requirement_ids(decomposition: &QuestionDecomposition, text: &str) -> Vec<String> {
    let normalized_text = normalize_for_substring(text);
    let doc_terms = tokenize(text).into_iter().collect::<BTreeSet<_>>();
    let mut covered = Vec::new();
    for requirement in &decomposition.requirements {
        if requirement.tokens.is_empty() {
            continue;
        }
        if requirement.kind == QuestionRequirementKind::Anchor {
            let needle = normalize_for_substring(&requirement.text);
            if !needle.is_empty() && normalized_text.contains(&needle) {
                covered.push(requirement.id.clone());
                continue;
            }
        }
        let hits = requirement
            .tokens
            .iter()
            .filter(|term| doc_terms.contains(*term))
            .count();
        let required = if requirement.tokens.len() <= 2 {
            1
        } else {
            (requirement.tokens.len() * 45).div_ceil(100).max(2)
        };
        if hits >= required {
            covered.push(requirement.id.clone());
        }
    }
    covered
}

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

fn precise_anchors(question: &str) -> Vec<String> {
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

fn expected_slots(question: &str) -> Vec<String> {
    let lower = question.to_ascii_lowercase();
    let mut slots = Vec::new();
    if contains_any_word_or_phrase(
        &lower,
        &["when", "scheduled", "schedule", "time window", "timezone"],
    ) {
        slots.push("date time schedule window timezone".to_owned());
    }
    if contains_any_word_or_phrase(
        &lower,
        &[
            "threshold",
            "limit",
            "pass rate",
            "gate",
            "cutoff",
            "size",
            "budget",
        ],
    ) {
        slots.push("threshold limit default pass rate gate size budget".to_owned());
    }
    if contains_any_word_or_phrase(
        &lower,
        &["latency", "p95", "p99", "ms", "rtt", "sla", "slo"],
    ) {
        slots.push("latency p95 p99 ms rtt sla slo target".to_owned());
    }
    if contains_any_word_or_phrase(
        &lower,
        &["cost", "price", "credits", "billing", "invoice", "cheapest"],
    ) {
        slots.push("cost price credits billing invoice".to_owned());
    }
    if contains_any_word_or_phrase(&lower, &["cause", "root cause", "caused", "trigger"]) {
        slots.push("root cause trigger reason".to_owned());
    }
    if contains_any_word_or_phrase(
        &lower,
        &[
            "location",
            "region",
            "edge",
            "cluster",
            "route",
            "environment",
        ],
    ) {
        slots.push("location region edge cluster route environment".to_owned());
    }
    if contains_any_word_or_phrase(
        &lower,
        &[
            "role", "owner", "owns", "dri", "review", "approver", "approval",
        ],
    ) {
        slots.push("role owner reviewer approver dri".to_owned());
    }
    if contains_any_word_or_phrase(
        &lower,
        &[
            "status",
            "state",
            "blocker",
            "blocked",
            "mitigation",
            "rollback",
            "risk",
        ],
    ) {
        slots.push("status blocker mitigation rollback risk".to_owned());
    }
    if contains_any_word_or_phrase(&lower, &["all", "every", "list", "complete", "procedure"]) {
        slots.push("complete checklist of requested subparts".to_owned());
    }
    unique_texts(slots, 12)
}

#[derive(Default)]
struct RequirementBuilder {
    requirements: Vec<QuestionRequirement>,
    seen: BTreeSet<String>,
}

impl RequirementBuilder {
    fn is_empty(&self) -> bool {
        self.requirements.is_empty()
    }

    fn push(&mut self, kind: QuestionRequirementKind, text: String) {
        let text = clean_part(&text);
        let tokens = requirement_tokens(&text);
        if text.is_empty() || tokens.is_empty() {
            return;
        }
        let key = format!("{}:{}", kind.as_str(), normalize_for_key(&text));
        if !self.seen.insert(key) {
            return;
        }
        self.requirements.push(QuestionRequirement {
            id: format!("u{:02}", self.requirements.len() + 1),
            kind,
            text,
            tokens,
        });
    }

    fn finish(self) -> Vec<QuestionRequirement> {
        self.requirements
    }
}

fn texts_by_kind(
    requirements: &[QuestionRequirement],
    kind: QuestionRequirementKind,
) -> Vec<String> {
    requirements
        .iter()
        .filter(|item| item.kind == kind)
        .map(|item| item.text.clone())
        .collect()
}

fn requirement_tokens(text: &str) -> Vec<String> {
    tokenize(text)
        .into_iter()
        .filter(|term| !is_requirement_stopword(term))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn is_requirement_stopword(term: &str) -> bool {
    matches!(
        term,
        "who"
            | "what"
            | "which"
            | "where"
            | "when"
            | "why"
            | "how"
            | "give"
            | "tell"
            | "show"
            | "find"
            | "include"
            | "including"
            | "does"
            | "did"
            | "was"
            | "were"
            | "are"
            | "for"
            | "with"
            | "from"
            | "into"
            | "about"
            | "is"
            | "be"
            | "can"
            | "on"
            | "by"
            | "if"
            | "has"
            | "have"
            | "i"
            | "s"
            | "t"
            | "e"
            | "g"
            | "any"
            | "each"
            | "those"
            | "up"
            | "so"
            | "get"
            | "this"
            | "that"
            | "their"
            | "our"
    )
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

fn unique_texts(values: Vec<String>, limit: usize) -> Vec<String> {
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

fn clean_token(raw: &str) -> String {
    raw.trim_matches(|ch: char| {
        matches!(
            ch,
            ',' | ';' | ':' | ')' | '(' | '[' | ']' | '{' | '}' | '"' | '\'' | '?' | '!'
        )
    })
    .to_owned()
}

fn clean_part(raw: &str) -> String {
    raw.trim_matches(|ch: char| matches!(ch, ',' | ';' | ':' | '.' | '?' | '!' | ' '))
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn compact_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn normalize_for_key(text: &str) -> String {
    tokenize(text).join(" ")
}

fn normalize_for_substring(text: &str) -> String {
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

fn contains_any_word_or_phrase(text: &str, needles: &[&str]) -> bool {
    let terms = tokenize(text).into_iter().collect::<BTreeSet<_>>();
    needles.iter().any(|needle| {
        if needle.contains(' ') {
            text.contains(needle)
        } else {
            terms.contains(*needle)
        }
    })
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

fn is_question_word(token: &str) -> bool {
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

#[cfg(test)]
mod tests {
    use super::{
        covered_requirement_ids, decompose_enterprise_rag_question, split_subquestions,
        QuestionRequirementKind,
    };

    #[test]
    fn decomposes_project_delivery_question_into_slots_and_subquestions() {
        let decomposition = decompose_enterprise_rag_question(
            "Who owns the Apollo launch blocker, what is the deadline, and how should support verify the risk?",
        );

        assert!(decomposition.multi_requirement);
        assert!(decomposition
            .slots
            .iter()
            .any(|slot| slot.contains("owner")));
        assert!(decomposition
            .slots
            .iter()
            .any(|slot| slot.contains("status blocker")));
        assert!(decomposition.subquestions.len() >= 2);
        assert!(decomposition
            .requirements
            .iter()
            .any(|item| item.kind == QuestionRequirementKind::Anchor && item.text == "Apollo"));
    }

    #[test]
    fn decomposes_threshold_metric_and_cost_slots() {
        let decomposition = decompose_enterprise_rag_question(
            "What p95 latency threshold and cost limit are required for the EU route?",
        );

        assert!(decomposition
            .slots
            .iter()
            .any(|slot| slot.contains("threshold")));
        assert!(decomposition
            .slots
            .iter()
            .any(|slot| slot.contains("latency")));
        assert!(decomposition.slots.iter().any(|slot| slot.contains("cost")));
    }

    #[test]
    fn coverage_reports_requirements_supported_by_payload() {
        let decomposition = decompose_enterprise_rag_question(
            "Who owns the Apollo launch blocker and what is the deadline?",
        );
        let covered = covered_requirement_ids(
            &decomposition,
            "project=Apollo owner=Maya launch blocker is auth; deadline is 2026-05-01.",
        );

        assert!(covered.len() >= 3, "{covered:?}");
    }

    #[test]
    fn split_subquestions_handles_connectors_and_lists() {
        let parts = split_subquestions(
            "What caused the incident, what mitigation shipped, and where is the follow-up ticket?",
        );

        assert!(parts.iter().any(|part| part.contains("caused")));
        assert!(parts.iter().any(|part| part.contains("mitigation")));
        assert!(parts.iter().any(|part| part.contains("follow-up ticket")));
    }
}
