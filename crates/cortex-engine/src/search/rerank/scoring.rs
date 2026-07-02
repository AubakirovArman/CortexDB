use std::cmp::Reverse;

use super::super::frozen_weights;
use super::super::{
    analyze_search_query, condition_payload_bonus, covered_requirement_ids,
    decompose_enterprise_rag_question, extract_query_conditions, map_query_to_scope,
    scope_mapping_metadata_bonus, tokenize,
};
use super::calibration::rerank_calibration_profile;
use super::types::{SearchRerankInput, SearchReranker, WeightedScoreReranker};

impl WeightedScoreReranker {
    pub fn calibrated_for_query(self, query_text: &str) -> Self {
        if !self.calibrate_by_question_type {
            return self;
        }
        rerank_calibration_profile(query_text, self).reranker
    }
}

impl SearchReranker for WeightedScoreReranker {
    fn rerank_score(&self, input: SearchRerankInput<'_>) -> u64 {
        let profile = self.calibrated_for_query(input.query_text);
        let lexical = input
            .lexical_score
            .saturating_mul(u64::from(profile.lexical_weight));
        let vector = input
            .vector_score
            .saturating_mul(u64::from(profile.vector_weight));
        let score = input
            .base_score
            .saturating_add(lexical)
            .saturating_add(vector)
            .saturating_add(payload_signal_bonus(input, &profile));
        apply_evidence_overlap_gate(score, input, profile.no_evidence_overlap_score_q16)
    }
}

pub(crate) fn sort_reranked<T>(
    values: &mut [T],
    mut candidate_id: impl FnMut(&T) -> u32,
    mut score: impl FnMut(&T) -> u64,
) {
    values.sort_by_key(|value| (Reverse(score(value)), candidate_id(value)));
}

fn payload_signal_bonus(input: SearchRerankInput<'_>, reranker: &WeightedScoreReranker) -> u64 {
    let analyzed = analyze_search_query(input.query_text);
    let mut bonus = 0u64;
    if let Some(payload) = input.payload {
        let payload = String::from_utf8_lossy(payload).to_lowercase();
        for anchor in analyzed.anchors {
            for term in anchor.terms {
                if payload.contains(&term) {
                    bonus = bonus.saturating_add(reranker.anchor_payload_bonus);
                }
            }
        }
        for source in analyzed.source_hints {
            if payload.contains(&source) {
                bonus = bonus.saturating_add(reranker.source_hint_payload_bonus);
            }
        }
        let decomposition = decompose_enterprise_rag_question(input.query_text);
        let covered = covered_requirement_ids(&decomposition, &payload);
        bonus = bonus.saturating_add(
            u64::try_from(covered.len())
                .unwrap_or(u64::MAX)
                .saturating_mul(frozen_weights::RERANK_REQUIREMENT_PAYLOAD_BONUS),
        );
        let conditions = extract_query_conditions(input.query_text);
        bonus = bonus.saturating_add(
            condition_payload_bonus(&conditions, payload.as_bytes())
                .saturating_mul(reranker.condition_payload_bonus),
        );
        for term in tokenize(input.query_text) {
            if payload.contains(&term) {
                bonus = bonus.saturating_add(frozen_weights::RERANK_TERM_PAYLOAD_BONUS);
            }
        }
    };
    let scope_mapping = map_query_to_scope(input.query_text);
    if let Some(metadata) = input.metadata {
        bonus = bonus.saturating_add(
            scope_mapping_metadata_bonus(&scope_mapping, metadata)
                .saturating_mul(reranker.scope_mapping_metadata_bonus),
        );
    }
    bonus
}

fn apply_evidence_overlap_gate(
    score: u64,
    input: SearchRerankInput<'_>,
    no_overlap_score_q16: u16,
) -> u64 {
    let Some(payload) = input.payload else {
        return score;
    };
    if has_evidence_overlap(input.query_text, payload) {
        return score;
    }
    score.saturating_mul(u64::from(no_overlap_score_q16)) / frozen_weights::Q16_ONE_U64
}

fn has_evidence_overlap(query_text: &str, payload: &[u8]) -> bool {
    evidence_overlap_score(query_text, payload) >= frozen_weights::EVIDENCE_OVERLAP_THRESHOLD
}

pub(super) fn evidence_overlap_score(query_text: &str, payload: &[u8]) -> u32 {
    let payload = String::from_utf8_lossy(payload).to_lowercase();
    let analyzed = analyze_search_query(query_text);
    let mut score = 0u32;
    for anchor in analyzed.anchors {
        if anchor.terms.iter().any(|term| payload.contains(term)) {
            score = score.saturating_add(frozen_weights::EVIDENCE_ANCHOR_POINTS);
        }
    }
    if analyzed
        .source_hints
        .iter()
        .any(|source| payload.contains(source))
    {
        score = score.saturating_add(frozen_weights::EVIDENCE_SOURCE_POINTS);
    }
    let conditions = extract_query_conditions(query_text);
    if condition_payload_bonus(&conditions, payload.as_bytes()) > 0 {
        score = score.saturating_add(frozen_weights::EVIDENCE_CONDITION_POINTS);
    }
    let decomposition = decompose_enterprise_rag_question(query_text);
    let covered_requirements = covered_requirement_ids(&decomposition, &payload);
    if covered_requirements.len() >= 2 {
        score = score.saturating_add(
            u32::try_from(covered_requirements.len())
                .unwrap_or(u32::MAX)
                .saturating_mul(frozen_weights::EVIDENCE_REQUIREMENT_POINTS),
        );
    }
    for term in evidence_terms(query_text) {
        if payload.contains(&term) {
            score = score.saturating_add(frozen_weights::EVIDENCE_TERM_POINTS);
        }
    }
    score
}

fn evidence_terms(query_text: &str) -> Vec<String> {
    tokenize(query_text)
        .into_iter()
        .filter(|term| is_evidence_term(term))
        .collect()
}

fn is_evidence_term(term: &str) -> bool {
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
        )
}
