use std::collections::BTreeSet;

use cortex_engine::search::{
    decompose_enterprise_rag_question, route_policy_for_query, SearchDiversityDiagnostics,
    SearchQueryIntent,
};
use cortex_engine::CellMetadata;

use super::{push_unique_prefilter_candidates, PrefilterCandidate};

pub(crate) struct PrefilterDiverseSelection {
    pub(crate) candidates: Vec<PrefilterCandidate>,
    pub(crate) diagnostics: SearchDiversityDiagnostics,
}

pub(crate) fn select_diverse_prefilter_candidates(
    protected_order_candidates: Vec<PrefilterCandidate>,
    mut candidates: Vec<PrefilterCandidate>,
    query_text: &str,
    top_k: usize,
) -> PrefilterDiverseSelection {
    let route_policy = route_policy_for_query(query_text);
    // This path starts from an already-clean external retrieval row. It may
    // reorder or diversify the bounded candidate pool, but it must not shrink
    // the row because full-500 retrieval promotion is judged on document
    // recall@top_k. Adaptive answer/context caps belong to the product search
    // and ContextPack paths, not to this retrieval-only gate.
    let result_limit = top_k;
    let mut diagnostics = SearchDiversityDiagnostics {
        intent: cortex_engine::search::classify_search_query_intent(query_text),
        diversity_enabled: route_policy.diversity,
        lambda_q16: route_policy.diversity_lambda_q16,
        input_candidates: candidates.len(),
        output_candidates: 0,
        skipped_candidates: 0,
        max_payload_similarity_q16: 0,
        max_cluster_similarity_q16: 0,
        selected_with_payload_similarity: 0,
        selected_with_cluster_similarity: 0,
    };
    if candidates.len() <= result_limit {
        diagnostics.output_candidates = candidates.len();
        return PrefilterDiverseSelection {
            candidates,
            diagnostics,
        };
    }
    if !route_policy.diversity {
        candidates.truncate(result_limit);
        diagnostics.output_candidates = candidates.len();
        diagnostics.skipped_candidates = diagnostics
            .input_candidates
            .saturating_sub(diagnostics.output_candidates);
        return PrefilterDiverseSelection {
            candidates,
            diagnostics,
        };
    }

    let mut selected = Vec::<PrefilterCandidate>::with_capacity(result_limit);
    let protected_head =
        prefilter_diversity_protected_head_count(diagnostics.intent, query_text, result_limit);
    if protected_head > 0 {
        let mut seen = BTreeSet::new();
        push_unique_prefilter_candidates(
            &mut selected,
            &mut seen,
            protected_order_candidates.iter().take(protected_head),
            result_limit,
        );
        candidates.retain(|candidate| !seen.contains(&candidate.cell_id.0));
    }
    while !candidates.is_empty() && selected.len() < result_limit {
        let mut best = None::<(usize, PrefilterDiversitySimilarity, u64)>;
        for (index, candidate) in candidates.iter().enumerate() {
            let similarity = prefilter_diversity_similarity_q16(candidate, &selected);
            diagnostics.max_payload_similarity_q16 = diagnostics
                .max_payload_similarity_q16
                .max(similarity.payload_q16);
            diagnostics.max_cluster_similarity_q16 = diagnostics
                .max_cluster_similarity_q16
                .max(similarity.cluster_q16);
            let score = prefilter_mmr_diversity_score(
                candidate.score,
                similarity.max_q16(),
                route_policy.diversity_lambda_q16,
            );
            if best
                .as_ref()
                .is_none_or(|(_, _, best_score)| score > *best_score)
            {
                best = Some((index, similarity, score));
            }
        }
        let (best_index, best_similarity, _) =
            best.unwrap_or((0, PrefilterDiversitySimilarity::default(), 0));
        if !selected.is_empty() && best_similarity.payload_q16 > 0 {
            diagnostics.selected_with_payload_similarity += 1;
        }
        if !selected.is_empty() && best_similarity.cluster_q16 > 0 {
            diagnostics.selected_with_cluster_similarity += 1;
        }
        selected.push(candidates.remove(best_index));
    }
    diagnostics.output_candidates = selected.len();
    diagnostics.skipped_candidates = diagnostics
        .input_candidates
        .saturating_sub(diagnostics.output_candidates);
    PrefilterDiverseSelection {
        candidates: selected,
        diagnostics,
    }
}

fn prefilter_diversity_protected_head_count(
    intent: SearchQueryIntent,
    query_text: &str,
    result_limit: usize,
) -> usize {
    let head = match intent {
        SearchQueryIntent::Lookup
        | SearchQueryIntent::InfoNotFound
        | SearchQueryIntent::Constrained => 0,
        SearchQueryIntent::ConflictingInfo => 2,
        SearchQueryIntent::Completeness => result_limit / 2,
        SearchQueryIntent::HighLevel => 3,
        SearchQueryIntent::ProjectRelated | SearchQueryIntent::Semantic => result_limit,
    };
    if decompose_enterprise_rag_question(query_text)
        .requirements
        .len()
        > 3
    {
        return head.max(result_limit / 2).min(result_limit);
    }
    head.min(result_limit)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct PrefilterDiversitySimilarity {
    payload_q16: u64,
    cluster_q16: u64,
}

impl PrefilterDiversitySimilarity {
    fn max_q16(self) -> u64 {
        self.payload_q16.max(self.cluster_q16)
    }
}

fn prefilter_diversity_similarity_q16(
    candidate: &PrefilterCandidate,
    selected: &[PrefilterCandidate],
) -> PrefilterDiversitySimilarity {
    selected
        .iter()
        .map(|existing| PrefilterDiversitySimilarity {
            payload_q16: prefilter_payload_jaccard_q16(&candidate.payload, &existing.payload),
            cluster_q16: prefilter_metadata_cluster_similarity_q16(
                &candidate.payload,
                &existing.payload,
            ),
        })
        .max_by_key(|similarity| similarity.max_q16())
        .unwrap_or_default()
}

fn prefilter_mmr_diversity_score(score: u64, similarity_q16: u64, lambda_q16: u16) -> u64 {
    let relevance = u128::from(score).saturating_mul(u128::from(lambda_q16)) / 65_535;
    let diversity_weight = 65_535u16.saturating_sub(lambda_q16);
    let redundancy_penalty = u128::from(score)
        .saturating_mul(u128::from(diversity_weight))
        .saturating_mul(u128::from(similarity_q16))
        / (65_535u128 * 65_535u128);
    u64::try_from(relevance.saturating_sub(redundancy_penalty)).unwrap_or(u64::MAX)
}

fn prefilter_payload_jaccard_q16(left: &[u8], right: &[u8]) -> u64 {
    let left = prefilter_payload_terms(left);
    let right = prefilter_payload_terms(right);
    if left.is_empty() || right.is_empty() {
        return 0;
    }
    let intersection = left.intersection(&right).count() as u64;
    let union = left.union(&right).count() as u64;
    intersection.saturating_mul(65_535) / union.max(1)
}

fn prefilter_payload_terms(payload: &[u8]) -> BTreeSet<String> {
    CellMetadata::from_payload(payload)
        .terms
        .into_iter()
        .filter(|term| term.len() >= 3)
        .collect()
}

fn prefilter_metadata_cluster_similarity_q16(left: &[u8], right: &[u8]) -> u64 {
    let left = CellMetadata::from_payload(left);
    let right = CellMetadata::from_payload(right);
    let mut score = 0;
    score = score.max(prefilter_matching_cluster_score(
        left.content_hash.as_deref(),
        right.content_hash.as_deref(),
        65_535,
    ));
    score = score.max(prefilter_matching_cluster_score(
        left.document_id.as_deref(),
        right.document_id.as_deref(),
        65_535,
    ));
    score = score.max(prefilter_matching_cluster_score(
        left.parent_id.as_deref(),
        right.parent_id.as_deref(),
        58_982,
    ));
    score = score.max(prefilter_matching_cluster_score(
        left.source_hash.as_deref(),
        right.source_hash.as_deref(),
        52_428,
    ));
    score = score.max(prefilter_matching_cluster_score(
        left.path.as_deref(),
        right.path.as_deref(),
        49_152,
    ));
    score = score.max(prefilter_matching_cluster_score(
        left.project.as_deref(),
        right.project.as_deref(),
        36_864,
    ));
    score = score.max(prefilter_matching_cluster_score(
        left.entity.as_deref(),
        right.entity.as_deref(),
        32_768,
    ));
    score = score.max(prefilter_matching_cluster_score(
        left.topic.as_deref(),
        right.topic.as_deref(),
        24_576,
    ));
    score = score.max(prefilter_matching_cluster_score(
        left.source.as_deref(),
        right.source.as_deref(),
        16_384,
    ));
    score
}

fn prefilter_matching_cluster_score(left: Option<&str>, right: Option<&str>, score: u64) -> u64 {
    match (left, right) {
        (Some(left), Some(right)) if !left.trim().is_empty() && left == right => score,
        _ => 0,
    }
}
