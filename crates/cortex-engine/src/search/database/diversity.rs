use std::collections::BTreeSet;

use crate::query::CellMetadata;

use super::super::routing::{classify_search_query_intent, route_policy_for_query};
use super::super::SearchQuery;
use super::ranking::search_rerank_result_limit;
use super::{DatabaseSearchResult, SearchDiversityDiagnostics};

pub(super) fn select_diverse_results(
    mut results: Vec<DatabaseSearchResult>,
    query: SearchQuery<'_>,
) -> DiverseSelection {
    let result_limit = search_rerank_result_limit(query);
    let route_policy = route_policy_for_query(query.text);
    let mut diagnostics = SearchDiversityDiagnostics {
        intent: classify_search_query_intent(query.text),
        diversity_enabled: route_policy.diversity,
        lambda_q16: route_policy.diversity_lambda_q16,
        input_candidates: results.len(),
        output_candidates: 0,
        skipped_candidates: 0,
        max_payload_similarity_q16: 0,
        max_cluster_similarity_q16: 0,
        selected_with_payload_similarity: 0,
        selected_with_cluster_similarity: 0,
    };
    if results.len() <= result_limit {
        diagnostics.output_candidates = results.len();
        return DiverseSelection {
            results,
            diagnostics,
        };
    }
    if !route_policy.diversity {
        results.truncate(result_limit);
        diagnostics.output_candidates = results.len();
        diagnostics.skipped_candidates = diagnostics
            .input_candidates
            .saturating_sub(diagnostics.output_candidates);
        return DiverseSelection {
            results,
            diagnostics,
        };
    }

    let mut selected = Vec::<DatabaseSearchResult>::with_capacity(result_limit);
    while !results.is_empty() && selected.len() < result_limit {
        let mut best = None::<(usize, DiversitySimilarity, u64)>;
        for (index, candidate) in results.iter().enumerate() {
            let similarity = diversity_similarity_q16(candidate, &selected);
            diagnostics.max_payload_similarity_q16 = diagnostics
                .max_payload_similarity_q16
                .max(similarity.payload_q16);
            diagnostics.max_cluster_similarity_q16 = diagnostics
                .max_cluster_similarity_q16
                .max(similarity.cluster_q16);
            let score = mmr_diversity_score(
                candidate,
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
            best.unwrap_or((0, DiversitySimilarity::default(), 0));
        if !selected.is_empty() && best_similarity.payload_q16 > 0 {
            diagnostics.selected_with_payload_similarity += 1;
        }
        if !selected.is_empty() && best_similarity.cluster_q16 > 0 {
            diagnostics.selected_with_cluster_similarity += 1;
        }
        selected.push(results.remove(best_index));
    }
    diagnostics.output_candidates = selected.len();
    diagnostics.skipped_candidates = diagnostics
        .input_candidates
        .saturating_sub(diagnostics.output_candidates);
    DiverseSelection {
        results: selected,
        diagnostics,
    }
}

fn mmr_diversity_score(
    candidate: &DatabaseSearchResult,
    similarity_q16: u64,
    lambda_q16: u16,
) -> u64 {
    let relevance = u128::from(candidate.score).saturating_mul(u128::from(lambda_q16)) / 65_535;
    let diversity_weight = 65_535u16.saturating_sub(lambda_q16);
    let redundancy_penalty = u128::from(candidate.score)
        .saturating_mul(u128::from(diversity_weight))
        .saturating_mul(u128::from(similarity_q16))
        / (65_535u128 * 65_535u128);
    u64::try_from(relevance.saturating_sub(redundancy_penalty)).unwrap_or(u64::MAX)
}

pub(super) struct DiverseSelection {
    pub(super) results: Vec<DatabaseSearchResult>,
    pub(super) diagnostics: SearchDiversityDiagnostics,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct DiversitySimilarity {
    payload_q16: u64,
    cluster_q16: u64,
}

impl DiversitySimilarity {
    fn max_q16(self) -> u64 {
        self.payload_q16.max(self.cluster_q16)
    }
}

fn diversity_similarity_q16(
    candidate: &DatabaseSearchResult,
    selected: &[DatabaseSearchResult],
) -> DiversitySimilarity {
    selected
        .iter()
        .map(|existing| DiversitySimilarity {
            payload_q16: payload_jaccard_q16(&candidate.metadata, &existing.metadata),
            cluster_q16: metadata_cluster_similarity_q16(&candidate.metadata, &existing.metadata),
        })
        .max_by_key(|similarity| similarity.max_q16())
        .unwrap_or_default()
}

fn payload_jaccard_q16(left: &CellMetadata, right: &CellMetadata) -> u64 {
    let left = payload_terms(left);
    let right = payload_terms(right);
    if left.is_empty() || right.is_empty() {
        return 0;
    }
    let intersection = left.intersection(&right).count() as u64;
    let union = left.union(&right).count() as u64;
    intersection.saturating_mul(65_535) / union.max(1)
}

fn payload_terms(metadata: &CellMetadata) -> BTreeSet<String> {
    metadata
        .terms
        .iter()
        .filter(|term| term.len() >= 3)
        .cloned()
        .collect()
}

fn metadata_cluster_similarity_q16(left: &CellMetadata, right: &CellMetadata) -> u64 {
    let mut score = 0;
    score = score.max(matching_cluster_score(
        left.content_hash.as_deref(),
        right.content_hash.as_deref(),
        65_535,
    ));
    score = score.max(matching_cluster_score(
        left.document_id.as_deref(),
        right.document_id.as_deref(),
        65_535,
    ));
    score = score.max(matching_cluster_score(
        left.parent_id.as_deref(),
        right.parent_id.as_deref(),
        58_982,
    ));
    score = score.max(matching_cluster_score(
        left.source_hash.as_deref(),
        right.source_hash.as_deref(),
        52_428,
    ));
    score = score.max(matching_cluster_score(
        left.path.as_deref(),
        right.path.as_deref(),
        49_152,
    ));
    score = score.max(matching_cluster_score(
        left.project.as_deref(),
        right.project.as_deref(),
        36_864,
    ));
    score = score.max(matching_cluster_score(
        left.entity.as_deref(),
        right.entity.as_deref(),
        32_768,
    ));
    score = score.max(matching_cluster_score(
        left.topic.as_deref(),
        right.topic.as_deref(),
        24_576,
    ));
    score = score.max(matching_cluster_score(
        left.source.as_deref(),
        right.source.as_deref(),
        16_384,
    ));
    score
}

fn matching_cluster_score(left: Option<&str>, right: Option<&str>, score: u64) -> u64 {
    match (left, right) {
        (Some(left), Some(right)) if !left.trim().is_empty() && left == right => score,
        _ => 0,
    }
}
