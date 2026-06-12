use super::{map_query_to_scope, scope_mapping_payload_bonus, QueryScopeField, QueryScopeMapping};

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
