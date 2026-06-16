use std::collections::BTreeSet;

use cortex_engine::search::{
    decompose_enterprise_rag_question, route_policy_for_query, SearchDiversityDiagnostics,
    SearchMode, SearchQuery, SearchQueryIntent, SearchRerankInput, SearchReranker,
};

use super::super::{
    ENGINE_PREFILTER_DEFAULT_DOC_LIMIT, ENGINE_PREFILTER_LEXICAL_HEAD_COUNT,
    ENGINE_PREFILTER_STRONG_EVIDENCE_SCORE,
};
use super::candidate::{PrefilterCandidate, PrefilterSearchOutput};
use super::diversity::select_diverse_prefilter_candidates;

pub(crate) fn select_prefilter_candidates(
    mut candidates: Vec<PrefilterCandidate>,
    query: SearchQuery<'_>,
    top_k: usize,
    reranker: Option<&dyn SearchReranker>,
) -> PrefilterSearchOutput {
    let mut lexical_candidates = candidates.clone();
    lexical_candidates.sort_by_key(|candidate| {
        (
            std::cmp::Reverse(candidate.lexical_score),
            candidate.cell_id.0,
        )
    });
    if let Some(reranker) = reranker {
        for candidate in &mut candidates {
            candidate.score = reranker.rerank_score(SearchRerankInput {
                query_text: query.text,
                query_vector: query.vector,
                candidate_id: candidate.cell_id.0,
                lexical_score: candidate.lexical_score,
                vector_score: candidate.vector_score,
                base_score: candidate.score,
                metadata: None,
                payload: Some(&candidate.payload),
            });
        }
    }
    candidates.sort_by_key(|candidate| (std::cmp::Reverse(candidate.score), candidate.cell_id.0));
    let (selected, diversity_diagnostics) = if query.mode == SearchMode::HybridRerank {
        let selection =
            select_diverse_prefilter_candidates(lexical_candidates, candidates, query.text, top_k);
        (selection.candidates, Some(selection.diagnostics))
    } else {
        (
            merge_prefilter_candidates(query.text, lexical_candidates, candidates, top_k),
            None,
        )
    };
    PrefilterSearchOutput {
        payloads: selected
            .into_iter()
            .map(|candidate| candidate.payload)
            .collect(),
        diversity_diagnostics,
    }
}

pub(crate) fn empty_prefilter_diversity_diagnostics(
    query_text: &str,
) -> SearchDiversityDiagnostics {
    let route_policy = route_policy_for_query(query_text);
    SearchDiversityDiagnostics {
        intent: cortex_engine::search::classify_search_query_intent(query_text),
        diversity_enabled: route_policy.diversity,
        lambda_q16: route_policy.diversity_lambda_q16,
        input_candidates: 0,
        output_candidates: 0,
        skipped_candidates: 0,
        max_payload_similarity_q16: 0,
        max_cluster_similarity_q16: 0,
        selected_with_payload_similarity: 0,
        selected_with_cluster_similarity: 0,
    }
}

pub(crate) fn merge_prefilter_candidates(
    query_text: &str,
    lexical_candidates: Vec<PrefilterCandidate>,
    reranked_candidates: Vec<PrefilterCandidate>,
    top_k: usize,
) -> Vec<PrefilterCandidate> {
    let lexical_head = top_k.min(prefilter_lexical_head_count(query_text, top_k));
    let vector_promotions = top_k.saturating_sub(lexical_head);
    let pool_limit = top_k.saturating_mul(4).max(top_k);
    let mut selected = Vec::with_capacity(pool_limit);
    let mut seen = BTreeSet::new();
    push_unique_prefilter_candidates(
        &mut selected,
        &mut seen,
        lexical_candidates.iter().take(lexical_head),
        pool_limit,
    );
    push_unique_prefilter_candidates(
        &mut selected,
        &mut seen,
        reranked_candidates.iter().take(vector_promotions),
        pool_limit,
    );
    push_unique_prefilter_candidates(
        &mut selected,
        &mut seen,
        lexical_candidates.iter().skip(lexical_head),
        pool_limit,
    );
    push_unique_prefilter_candidates(
        &mut selected,
        &mut seen,
        reranked_candidates.iter().skip(vector_promotions),
        pool_limit,
    );
    prune_weak_prefilter_tail(query_text, selected, top_k)
}

pub(crate) fn push_unique_prefilter_candidates<'a>(
    selected: &mut Vec<PrefilterCandidate>,
    seen: &mut BTreeSet<u64>,
    candidates: impl Iterator<Item = &'a PrefilterCandidate>,
    top_k: usize,
) {
    for candidate in candidates {
        if selected.len() >= top_k {
            break;
        }
        if seen.insert(candidate.cell_id.0) {
            selected.push(candidate.clone());
        }
    }
}

pub(crate) fn prefilter_lexical_head_count(query_text: &str, top_k: usize) -> usize {
    let decomposition = decompose_enterprise_rag_question(query_text);
    let multipart = decomposition.requirements.len() > 1;
    let head = match cortex_engine::search::classify_search_query_intent(query_text) {
        SearchQueryIntent::Lookup
        | SearchQueryIntent::InfoNotFound
        | SearchQueryIntent::Constrained => ENGINE_PREFILTER_LEXICAL_HEAD_COUNT,
        SearchQueryIntent::Semantic if multipart => top_k,
        SearchQueryIntent::Semantic => ENGINE_PREFILTER_LEXICAL_HEAD_COUNT,
        SearchQueryIntent::ProjectRelated
        | SearchQueryIntent::HighLevel
        | SearchQueryIntent::ConflictingInfo
        | SearchQueryIntent::Completeness => top_k,
    };
    top_k.min(head.max(1))
}

pub(crate) fn prune_weak_prefilter_tail(
    query_text: &str,
    candidates: Vec<PrefilterCandidate>,
    top_k: usize,
) -> Vec<PrefilterCandidate> {
    let default_limit = top_k.min(prefilter_default_doc_limit(query_text, top_k));
    if candidates.len() <= default_limit {
        return candidates;
    }
    candidates
        .into_iter()
        .enumerate()
        .filter_map(|(index, candidate)| {
            if index < default_limit
                || candidate.evidence_score >= ENGINE_PREFILTER_STRONG_EVIDENCE_SCORE
            {
                Some(candidate)
            } else {
                None
            }
        })
        .take(top_k)
        .collect()
}

pub(crate) fn prefilter_default_doc_limit(query_text: &str, top_k: usize) -> usize {
    let decomposition = decompose_enterprise_rag_question(query_text);
    let multipart = decomposition.requirements.len() > 1;
    let limit = match cortex_engine::search::classify_search_query_intent(query_text) {
        SearchQueryIntent::Lookup => ENGINE_PREFILTER_DEFAULT_DOC_LIMIT,
        SearchQueryIntent::InfoNotFound => 3,
        SearchQueryIntent::Constrained => ENGINE_PREFILTER_DEFAULT_DOC_LIMIT,
        SearchQueryIntent::Semantic if multipart => top_k,
        SearchQueryIntent::Semantic => ENGINE_PREFILTER_DEFAULT_DOC_LIMIT.saturating_add(1),
        SearchQueryIntent::ProjectRelated
        | SearchQueryIntent::HighLevel
        | SearchQueryIntent::ConflictingInfo
        | SearchQueryIntent::Completeness => top_k,
    };
    top_k.min(limit.max(1))
}
