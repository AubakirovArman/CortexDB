use cortex_engine::search::{
    analyze_search_query, condition_payload_bonus, covered_requirement_ids,
    decompose_enterprise_rag_question, extract_query_conditions, map_query_to_scope,
    scope_mapping_payload_bonus, tokenize,
};

pub(crate) fn lexical_rank_score(rank: usize, shortlist_limit: usize) -> u64 {
    u64::try_from(shortlist_limit.saturating_sub(rank))
        .unwrap_or(0)
        .saturating_mul(1_000_000)
}

pub(crate) fn prefilter_evidence_score(query_text: &str, payload: &[u8]) -> u32 {
    let payload_text = String::from_utf8_lossy(payload);
    let payload_lower = payload_text.to_lowercase();
    let mut score = 0u32;
    let analyzed = analyze_search_query(query_text);
    for anchor in analyzed.anchors {
        if anchor.terms.iter().any(|term| payload_lower.contains(term)) {
            score = score.saturating_add(4);
        }
    }
    if analyzed
        .source_hints
        .iter()
        .any(|source| payload_lower.contains(source))
    {
        score = score.saturating_add(4);
    }
    let scope_mapping = map_query_to_scope(query_text);
    if scope_mapping_payload_bonus(&scope_mapping, payload) > 0 {
        score = score.saturating_add(4);
    }
    let conditions = extract_query_conditions(query_text);
    if condition_payload_bonus(&conditions, payload) > 0 {
        score = score.saturating_add(4);
    }
    let decomposition = decompose_enterprise_rag_question(query_text);
    score = score.saturating_add(
        u32::try_from(covered_requirement_ids(&decomposition, &payload_text).len())
            .unwrap_or(u32::MAX)
            .saturating_mul(2),
    );
    for term in tokenize(query_text)
        .into_iter()
        .filter(|term| is_prefilter_evidence_term(term))
    {
        if payload_lower.contains(&term) {
            score = score.saturating_add(1);
        }
    }
    score
}

pub(crate) fn is_prefilter_evidence_term(term: &str) -> bool {
    term.len() >= 3
        && !matches!(
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
                | "list"
                | "all"
                | "any"
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
                | "this"
                | "that"
                | "project"
                | "team"
                | "docs"
                | "document"
        )
}
