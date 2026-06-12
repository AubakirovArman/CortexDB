use std::collections::BTreeSet;

use super::{analyze_search_query, tokenize, QueryAnchorKind};
use crate::query::metadata::CellMetadata;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum QueryScopeField {
    Source,
    Scope,
    Project,
    Team,
    Owner,
    Topic,
    Entity,
}

impl QueryScopeField {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Scope => "scope",
            Self::Project => "project",
            Self::Team => "team",
            Self::Owner => "owner",
            Self::Topic => "topic",
            Self::Entity => "entity",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryScopeDirective {
    pub field: QueryScopeField,
    pub value: String,
    pub confidence_q16: u16,
    pub hard_filter: bool,
    pub terms: Vec<String>,
    pub reason: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryScopeMapping {
    pub query: String,
    pub directives: Vec<QueryScopeDirective>,
}

impl QueryScopeMapping {
    pub fn has_scope_filter(&self) -> bool {
        self.directives.iter().any(|directive| {
            matches!(
                directive.field,
                QueryScopeField::Source
                    | QueryScopeField::Scope
                    | QueryScopeField::Project
                    | QueryScopeField::Team
                    | QueryScopeField::Topic
                    | QueryScopeField::Entity
            )
        })
    }

    pub fn source_filters(&self) -> Vec<String> {
        self.values_for_field(QueryScopeField::Source)
    }

    pub fn project_filters(&self) -> Vec<String> {
        self.values_for_field(QueryScopeField::Project)
    }

    pub fn scope_filters(&self) -> Vec<String> {
        self.values_for_field(QueryScopeField::Scope)
    }

    fn values_for_field(&self, field: QueryScopeField) -> Vec<String> {
        self.directives
            .iter()
            .filter(|directive| directive.field == field)
            .map(|directive| directive.value.clone())
            .collect()
    }
}

pub fn map_query_to_scope(query: &str) -> QueryScopeMapping {
    let query = compact_whitespace(query);
    let analyzed = analyze_search_query(&query);
    let mut builder = ScopeMappingBuilder::default();

    for source in analyzed.source_hints {
        builder.push(
            QueryScopeField::Source,
            source,
            61_000,
            true,
            "explicit_source_hint",
        );
    }
    for anchor in analyzed.anchors {
        match anchor.kind {
            QueryAnchorKind::TicketId => builder.push(
                QueryScopeField::Source,
                "jira".to_owned(),
                58_000,
                false,
                "ticket_anchor_source_hint",
            ),
            QueryAnchorKind::PullRequest | QueryAnchorKind::FilePath => builder.push(
                QueryScopeField::Source,
                "github".to_owned(),
                58_000,
                false,
                "code_anchor_source_hint",
            ),
            _ => {}
        }
    }

    for (field, value, confidence, reason) in lexicon_scope_hints(&query) {
        builder.push(field, value, confidence, false, reason);
    }
    for project in project_name_hints(&query).into_iter().take(8) {
        builder.push(
            QueryScopeField::Project,
            project,
            50_000,
            false,
            "project_name_hint",
        );
    }
    for team in team_name_hints(&query).into_iter().take(6) {
        builder.push(QueryScopeField::Team, team, 48_000, false, "team_name_hint");
    }

    QueryScopeMapping {
        query,
        directives: builder.finish(),
    }
}

pub fn scope_mapping_payload_bonus(mapping: &QueryScopeMapping, payload: &[u8]) -> u64 {
    if mapping.directives.is_empty() {
        return 0;
    }
    let metadata = CellMetadata::from_payload(payload);
    scope_mapping_metadata_bonus(mapping, &metadata)
}

pub fn scope_mapping_metadata_bonus(mapping: &QueryScopeMapping, metadata: &CellMetadata) -> u64 {
    if mapping.directives.is_empty() {
        return 0;
    }
    mapping
        .directives
        .iter()
        .filter(|directive| directive_matches_metadata(directive, metadata))
        .map(|directive| u64::from(directive.confidence_q16) / 6)
        .sum()
}

fn directive_matches_metadata(directive: &QueryScopeDirective, metadata: &CellMetadata) -> bool {
    let values = metadata_values_for_field(directive.field, metadata);
    values.iter().any(|value| {
        value_matches_directive(value, directive)
            || directive
                .terms
                .iter()
                .filter(|term| term.len() >= 3)
                .any(|term| tokenize(value).contains(term))
    })
}

fn metadata_values_for_field(field: QueryScopeField, metadata: &CellMetadata) -> Vec<&str> {
    match field {
        QueryScopeField::Source => vec![
            metadata.source.as_deref(),
            metadata
                .source_ref
                .as_ref()
                .map(|source| source.source_id.as_str()),
            metadata.path.as_deref(),
            metadata.title.as_deref(),
            Some(metadata.body_text.as_str()),
        ],
        QueryScopeField::Scope => vec![
            Some(metadata.scope.as_str()),
            metadata.topic.as_deref(),
            metadata.path.as_deref(),
            metadata.title.as_deref(),
            Some(metadata.body_text.as_str()),
        ],
        QueryScopeField::Project => vec![
            metadata.project.as_deref(),
            metadata.document_id.as_deref(),
            metadata.title.as_deref(),
            metadata.path.as_deref(),
            Some(metadata.body_text.as_str()),
        ],
        QueryScopeField::Team => vec![
            metadata.entity.as_deref(),
            metadata.owner.as_deref(),
            metadata.project.as_deref(),
            metadata.path.as_deref(),
            metadata.title.as_deref(),
            Some(metadata.body_text.as_str()),
        ],
        QueryScopeField::Owner => vec![
            metadata.owner.as_deref(),
            metadata.entity.as_deref(),
            metadata.title.as_deref(),
            Some(metadata.body_text.as_str()),
        ],
        QueryScopeField::Topic => vec![
            metadata.topic.as_deref(),
            metadata.section.as_deref(),
            metadata.title.as_deref(),
            metadata.path.as_deref(),
            Some(metadata.body_text.as_str()),
        ],
        QueryScopeField::Entity => vec![
            metadata.entity.as_deref(),
            metadata.project.as_deref(),
            metadata.owner.as_deref(),
            metadata.title.as_deref(),
            Some(metadata.body_text.as_str()),
        ],
    }
    .into_iter()
    .flatten()
    .collect()
}

fn value_matches_directive(value: &str, directive: &QueryScopeDirective) -> bool {
    let haystack = normalize_for_match(value);
    let needle = normalize_for_match(&directive.value);
    if needle.len() >= 3 && haystack.contains(&needle) {
        return true;
    }
    let value_terms = tokenize(value).into_iter().collect::<BTreeSet<_>>();
    let hits = directive
        .terms
        .iter()
        .filter(|term| value_terms.contains(*term))
        .count();
    hits >= directive.terms.len().clamp(1, 2)
}

#[derive(Default)]
struct ScopeMappingBuilder {
    directives: Vec<QueryScopeDirective>,
    seen: BTreeSet<String>,
}

impl ScopeMappingBuilder {
    fn push(
        &mut self,
        field: QueryScopeField,
        value: String,
        confidence_q16: u16,
        hard_filter: bool,
        reason: &'static str,
    ) {
        let value = clean_scope_value(&value);
        let terms = tokenize(&value)
            .into_iter()
            .filter(|term| !is_scope_stopword(term))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        if value.is_empty() || terms.is_empty() {
            return;
        }
        let key = format!("{}:{}", field.as_str(), normalize_for_match(&value));
        if !self.seen.insert(key) {
            return;
        }
        self.directives.push(QueryScopeDirective {
            field,
            value,
            confidence_q16,
            hard_filter,
            terms,
            reason,
        });
    }

    fn finish(mut self) -> Vec<QueryScopeDirective> {
        self.directives
            .sort_by_key(|directive| (directive.field, directive.value.clone()));
        self.directives
    }
}

fn lexicon_scope_hints(query: &str) -> Vec<(QueryScopeField, String, u16, &'static str)> {
    let lower = query.to_ascii_lowercase();
    let mut out = Vec::new();
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
        if contains_word_or_phrase(&lower, needle) {
            out.push((
                QueryScopeField::Scope,
                value.to_owned(),
                44_000,
                "department_scope_hint",
            ));
        }
    }
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
        if contains_word_or_phrase(&lower, needle) {
            out.push((
                QueryScopeField::Topic,
                value.to_owned(),
                42_000,
                "topic_scope_hint",
            ));
        }
    }
    out
}

fn project_name_hints(query: &str) -> Vec<String> {
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
            if let Some(next) = tokens.get(index + 1) {
                if is_name_like(next) {
                    values.insert(next.clone());
                }
            }
            if index > 0 {
                let previous = &tokens[index - 1];
                if is_name_like(previous) {
                    values.insert(previous.clone());
                }
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

fn team_name_hints(query: &str) -> Vec<String> {
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
            if let Some(next) = tokens.get(index + 1) {
                if is_name_like(next) {
                    values.insert(next.clone());
                }
            }
        }
    }
    values.into_iter().collect()
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

fn contains_word_or_phrase(lower: &str, needle: &str) -> bool {
    if needle.contains(' ') || needle.contains('-') {
        return lower.contains(needle);
    }
    lower
        .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
        .any(|word| word == needle)
}

fn is_name_like(token: &str) -> bool {
    let mut chars = token.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    token.len() >= 3
        && first.is_ascii_uppercase()
        && chars.any(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit())
}

fn clean_scope_value(value: &str) -> String {
    compact_whitespace(value.trim_matches(|ch: char| {
        matches!(
            ch,
            ',' | ';' | ':' | ')' | '(' | '[' | ']' | '{' | '}' | '"' | '\'' | '?' | '!'
        )
    }))
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

fn normalize_for_match(value: &str) -> String {
    tokenize(value).join(" ")
}

fn compact_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_scope_stopword(term: &str) -> bool {
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

#[cfg(test)]
mod tests {
    use super::{
        map_query_to_scope, scope_mapping_payload_bonus, QueryScopeField, QueryScopeMapping,
    };

    #[test]
    fn maps_explicit_source_and_project_without_oracle() {
        let mapping = map_query_to_scope("What did the Slack thread say about Apollo rollout?");

        assert!(has_directive(&mapping, QueryScopeField::Source, "slack"));
        assert!(has_directive(&mapping, QueryScopeField::Project, "Apollo"));
        assert!(mapping.has_scope_filter());
    }

    #[test]
    fn infers_source_from_ticket_and_pr_anchors() {
        let mapping = map_query_to_scope("Which PR #42 fixed AUTH-123?");

        assert!(has_directive(&mapping, QueryScopeField::Source, "github"));
        assert!(has_directive(&mapping, QueryScopeField::Source, "jira"));
    }

    #[test]
    fn maps_department_and_topic_scope_from_question_text() {
        let mapping = map_query_to_scope("What is the security team's SSO rollout policy?");

        assert!(has_directive(&mapping, QueryScopeField::Scope, "security"));
        assert!(has_directive(&mapping, QueryScopeField::Topic, "sso"));
        assert!(has_directive(&mapping, QueryScopeField::Topic, "rollout"));
    }

    #[test]
    fn payload_bonus_rewards_metadata_scope_matches() {
        let mapping = map_query_to_scope("What blocked Apollo rollout in Slack?");
        let matched = scope_mapping_payload_bonus(
            &mapping,
            b"source=slack\nproject=Apollo\ntopic=rollout\n\nApollo rollout blocker was auth.",
        );
        let weak = scope_mapping_payload_bonus(
            &mapping,
            b"source=gmail\nproject=Hermes\n\nOffice cleaning schedule.",
        );

        assert!(matched > weak);
    }

    fn has_directive(mapping: &QueryScopeMapping, field: QueryScopeField, value: &str) -> bool {
        mapping
            .directives
            .iter()
            .any(|directive| directive.field == field && directive.value == value)
    }
}
