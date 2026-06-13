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

#[test]
fn expands_bidirectional_enterprise_terms_without_gold_labels() {
    let analyzed = analyze_search_query("Who is the DRI for the slipped rollout ETA?");

    assert!(analyzed.weighted_terms.contains_key("owner"));
    assert!(analyzed.weighted_terms.contains_key("blocked"));
    assert!(analyzed.weighted_terms.contains_key("deadline"));
}

#[test]
fn expands_high_level_phrases_without_question_type_oracle() {
    let analyzed = analyze_search_query("Give me the high level company overview");

    assert!(analyzed.weighted_terms.contains_key("mission"));
    assert!(analyzed.weighted_terms.contains_key("charter"));
    assert!(analyzed.weighted_terms.contains_key("about"));
}
