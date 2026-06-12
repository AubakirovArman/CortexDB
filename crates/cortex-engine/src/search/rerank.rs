use std::cmp::Reverse;

use crate::query::CellMetadata;

use super::{
    analyze_search_query, classify_enterprise_rag_question_type, condition_payload_bonus,
    covered_requirement_ids, decompose_enterprise_rag_question, extract_query_conditions,
    map_query_to_scope, scope_mapping_metadata_bonus, scope_mapping_payload_bonus, tokenize,
    EnterpriseRagQuestionType,
};

const REQUIREMENT_PAYLOAD_BONUS: u64 = 2_500;
const Q16_ONE: u32 = 65_535;

#[derive(Clone, Copy, Debug)]
pub struct SearchRerankInput<'a> {
    pub query_text: &'a str,
    pub query_vector: Option<&'a [i16]>,
    pub candidate_id: u64,
    pub lexical_score: u64,
    pub vector_score: u64,
    pub base_score: u64,
    pub metadata: Option<&'a CellMetadata>,
    pub payload: Option<&'a [u8]>,
}

pub trait SearchReranker {
    fn rerank_score(&self, input: SearchRerankInput<'_>) -> u64;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HybridRrfWeights {
    pub lexical_q16: u32,
    pub vector_q16: u32,
}

impl HybridRrfWeights {
    pub fn lexical_heavy() -> Self {
        Self {
            lexical_q16: 45_000,
            vector_q16: Q16_ONE - 45_000,
        }
    }

    pub fn vector_heavy() -> Self {
        Self {
            lexical_q16: 20_000,
            vector_q16: Q16_ONE - 20_000,
        }
    }

    pub fn balanced() -> Self {
        Self {
            lexical_q16: 32_768,
            vector_q16: Q16_ONE - 32_768,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RerankCalibrationProfile {
    pub question_type: EnterpriseRagQuestionType,
    pub rrf_weights: HybridRrfWeights,
    pub reranker: WeightedScoreReranker,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WeightedScoreReranker {
    pub calibrate_by_question_type: bool,
    pub lexical_weight: u32,
    pub vector_weight: u32,
    pub anchor_payload_bonus: u64,
    pub source_hint_payload_bonus: u64,
    pub scope_mapping_payload_bonus: u64,
    pub condition_payload_bonus: u64,
    pub no_evidence_overlap_score_q16: u16,
}

impl Default for WeightedScoreReranker {
    fn default() -> Self {
        Self {
            calibrate_by_question_type: false,
            lexical_weight: 2,
            vector_weight: 2,
            anchor_payload_bonus: 25_000,
            source_hint_payload_bonus: 10_000,
            scope_mapping_payload_bonus: 1,
            condition_payload_bonus: 1,
            no_evidence_overlap_score_q16: 16_384,
        }
    }
}

impl WeightedScoreReranker {
    pub fn fixed_default() -> Self {
        Self::default()
    }

    pub fn enterprise_rag_calibrated() -> Self {
        Self {
            calibrate_by_question_type: true,
            ..Self::default()
        }
    }

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

pub fn calibrated_hybrid_rrf_weights(query_text: &str) -> HybridRrfWeights {
    rerank_calibration_profile(query_text, WeightedScoreReranker::default()).rrf_weights
}

pub fn rerank_calibration_profile(
    query_text: &str,
    base: WeightedScoreReranker,
) -> RerankCalibrationProfile {
    let question_type = calibration_question_type(query_text);
    let mut reranker = WeightedScoreReranker {
        calibrate_by_question_type: false,
        ..base
    };
    let rrf_weights = match question_type {
        EnterpriseRagQuestionType::Basic => {
            reranker.lexical_weight = 4;
            reranker.vector_weight = 1;
            reranker.no_evidence_overlap_score_q16 = 12_000;
            HybridRrfWeights::lexical_heavy()
        }
        EnterpriseRagQuestionType::Semantic | EnterpriseRagQuestionType::IntraDocumentReasoning => {
            reranker.lexical_weight = 1;
            reranker.vector_weight = 4;
            reranker.anchor_payload_bonus = 18_000;
            reranker.no_evidence_overlap_score_q16 = 20_000;
            HybridRrfWeights::vector_heavy()
        }
        EnterpriseRagQuestionType::ProjectRelated => {
            reranker.lexical_weight = 2;
            reranker.vector_weight = 3;
            reranker.scope_mapping_payload_bonus = 2;
            reranker.anchor_payload_bonus = 30_000;
            HybridRrfWeights {
                lexical_q16: 24_000,
                vector_q16: Q16_ONE - 24_000,
            }
        }
        EnterpriseRagQuestionType::Constrained => {
            reranker.lexical_weight = 4;
            reranker.vector_weight = 1;
            reranker.condition_payload_bonus = 3;
            reranker.anchor_payload_bonus = 28_000;
            reranker.no_evidence_overlap_score_q16 = 10_000;
            HybridRrfWeights::lexical_heavy()
        }
        EnterpriseRagQuestionType::ConflictingInfo => {
            reranker.lexical_weight = 3;
            reranker.vector_weight = 2;
            reranker.source_hint_payload_bonus = 14_000;
            HybridRrfWeights {
                lexical_q16: 38_000,
                vector_q16: Q16_ONE - 38_000,
            }
        }
        EnterpriseRagQuestionType::Completeness => {
            reranker.lexical_weight = 2;
            reranker.vector_weight = 3;
            reranker.anchor_payload_bonus = 22_000;
            reranker.no_evidence_overlap_score_q16 = 22_000;
            HybridRrfWeights {
                lexical_q16: 28_000,
                vector_q16: Q16_ONE - 28_000,
            }
        }
        EnterpriseRagQuestionType::HighLevel => {
            reranker.lexical_weight = 1;
            reranker.vector_weight = 4;
            reranker.source_hint_payload_bonus = 18_000;
            reranker.no_evidence_overlap_score_q16 = 24_000;
            HybridRrfWeights::vector_heavy()
        }
        EnterpriseRagQuestionType::InfoNotFound | EnterpriseRagQuestionType::Miscellaneous => {
            reranker.lexical_weight = 2;
            reranker.vector_weight = 2;
            reranker.no_evidence_overlap_score_q16 = 8_000;
            HybridRrfWeights::balanced()
        }
    };
    RerankCalibrationProfile {
        question_type,
        rrf_weights,
        reranker,
    }
}

fn calibration_question_type(query_text: &str) -> EnterpriseRagQuestionType {
    let classified = classify_enterprise_rag_question_type(query_text);
    if classified == EnterpriseRagQuestionType::Basic
        && looks_like_complex_semantic_query(query_text)
    {
        EnterpriseRagQuestionType::Semantic
    } else {
        classified
    }
}

fn looks_like_complex_semantic_query(query_text: &str) -> bool {
    let lower = query_text.to_ascii_lowercase();
    if lower.split_whitespace().count() < 16 {
        return false;
    }
    if lower.starts_with("why ")
        || lower.starts_with("how should ")
        || lower.starts_with("when a ")
        || lower.starts_with("when someone ")
        || lower.starts_with("after ")
    {
        return true;
    }
    [
        "how do we",
        "how should we",
        "in our ",
        "for the ",
        "what change",
        "what configuration change",
        "what mechanism",
        "what fields",
        "what is the specific risk",
        "what are the immediate follow up",
        "what short lived workaround",
        "what end-to-end response time",
        "what first-response-time",
        "what was the requested plan",
        "what is the expected update rhythm",
        "where can i find",
        "before an external audit",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

pub(crate) fn sort_reranked<T>(
    values: &mut [T],
    mut candidate_id: impl FnMut(&T) -> u32,
    mut score: impl FnMut(&T) -> u64,
) {
    values.sort_by_key(|value| (Reverse(score(value)), candidate_id(value)));
}

fn payload_signal_bonus(input: SearchRerankInput<'_>, reranker: &WeightedScoreReranker) -> u64 {
    let Some(payload) = input.payload else {
        return 0;
    };
    let payload = String::from_utf8_lossy(payload).to_lowercase();
    let analyzed = analyze_search_query(input.query_text);
    let mut bonus = 0u64;
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
            .saturating_mul(REQUIREMENT_PAYLOAD_BONUS),
    );
    let scope_mapping = map_query_to_scope(input.query_text);
    let scope_mapping_bonus = input.metadata.map_or_else(
        || scope_mapping_payload_bonus(&scope_mapping, payload.as_bytes()),
        |metadata| scope_mapping_metadata_bonus(&scope_mapping, metadata),
    );
    bonus = bonus
        .saturating_add(scope_mapping_bonus.saturating_mul(reranker.scope_mapping_payload_bonus));
    let conditions = extract_query_conditions(input.query_text);
    bonus = bonus.saturating_add(
        condition_payload_bonus(&conditions, payload.as_bytes())
            .saturating_mul(reranker.condition_payload_bonus),
    );
    for term in tokenize(input.query_text) {
        if payload.contains(&term) {
            bonus = bonus.saturating_add(1_000);
        }
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
    score.saturating_mul(u64::from(no_overlap_score_q16)) / 65_535
}

fn has_evidence_overlap(query_text: &str, payload: &[u8]) -> bool {
    evidence_overlap_score(query_text, payload) >= 2
}

fn evidence_overlap_score(query_text: &str, payload: &[u8]) -> u32 {
    let payload = String::from_utf8_lossy(payload).to_lowercase();
    let analyzed = analyze_search_query(query_text);
    let mut score = 0u32;
    for anchor in analyzed.anchors {
        if anchor.terms.iter().any(|term| payload.contains(term)) {
            score = score.saturating_add(2);
        }
    }
    if analyzed
        .source_hints
        .iter()
        .any(|source| payload.contains(source))
    {
        score = score.saturating_add(2);
    }
    let scope_mapping = map_query_to_scope(query_text);
    if scope_mapping_payload_bonus(&scope_mapping, payload.as_bytes()) > 0 {
        score = score.saturating_add(2);
    }
    let conditions = extract_query_conditions(query_text);
    if condition_payload_bonus(&conditions, payload.as_bytes()) > 0 {
        score = score.saturating_add(2);
    }
    let decomposition = decompose_enterprise_rag_question(query_text);
    let covered_requirements = covered_requirement_ids(&decomposition, &payload);
    if covered_requirements.len() >= 2 {
        score = score.saturating_add(
            u32::try_from(covered_requirements.len())
                .unwrap_or(u32::MAX)
                .saturating_mul(2),
        );
    }
    for term in evidence_terms(query_text) {
        if payload.contains(&term) {
            score = score.saturating_add(1);
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

#[cfg(test)]
mod tests {
    use crate::query::CellMetadata;

    use super::{
        calibrated_hybrid_rrf_weights, evidence_overlap_score, rerank_calibration_profile,
        SearchRerankInput, SearchReranker, WeightedScoreReranker,
    };

    #[test]
    fn weighted_reranker_rewards_anchor_payload_matches() {
        let reranker = WeightedScoreReranker::default();
        let matched = reranker.rerank_score(SearchRerankInput {
            query_text: "Which PR #42 fixed AUTH-123?",
            query_vector: None,
            candidate_id: 1,
            lexical_score: 1,
            vector_score: 0,
            base_score: 1,
            metadata: None,
            payload: Some(b"scope=project\n\nAUTH-123 was fixed by PR #42."),
        });
        let unmatched = reranker.rerank_score(SearchRerankInput {
            query_text: "Which PR #42 fixed AUTH-123?",
            query_vector: None,
            candidate_id: 2,
            lexical_score: 1,
            vector_score: 0,
            base_score: 1,
            metadata: None,
            payload: Some(b"scope=project\n\nGeneral engineering update."),
        });

        assert!(matched > unmatched);
    }

    #[test]
    fn weighted_reranker_penalizes_candidates_without_evidence_overlap() {
        let reranker = WeightedScoreReranker::default();
        let matched = reranker.rerank_score(SearchRerankInput {
            query_text: "Which PR #42 fixed AUTH-123?",
            query_vector: None,
            candidate_id: 1,
            lexical_score: 0,
            vector_score: 10_000,
            base_score: 10_000,
            metadata: None,
            payload: Some(b"AUTH-123 was fixed by PR #42."),
        });
        let unmatched = reranker.rerank_score(SearchRerankInput {
            query_text: "Which PR #42 fixed AUTH-123?",
            query_vector: None,
            candidate_id: 2,
            lexical_score: 0,
            vector_score: 10_000,
            base_score: 10_000,
            metadata: None,
            payload: Some(b"General engineering update."),
        });

        assert!(matched > unmatched);
        assert!(unmatched < 10_000);
    }

    #[test]
    fn evidence_overlap_requires_more_than_one_broad_query_term() {
        let weak = evidence_overlap_score(
            "What are the upload size limits for multipart requests?",
            b"Upload planning notes only.",
        );
        let strong = evidence_overlap_score(
            "What are the upload size limits for multipart requests?",
            b"Multipart upload requests have size limits.",
        );
        let anchored = evidence_overlap_score(
            "Which PR #42 fixed AUTH-123?",
            b"AUTH-123 was fixed by PR #42.",
        );

        assert_eq!(weak, 1);
        assert!(strong >= 2);
        assert!(anchored >= 2);
    }

    #[test]
    fn weighted_reranker_rewards_sub_requirement_coverage() {
        let reranker = WeightedScoreReranker::default();
        let matched = reranker.rerank_score(SearchRerankInput {
            query_text: "Who owns the Apollo launch blocker and what is the deadline?",
            query_vector: None,
            candidate_id: 1,
            lexical_score: 0,
            vector_score: 0,
            base_score: 1,
            metadata: None,
            payload: Some(b"Apollo owner Maya. Launch blocker is auth. Deadline is 2026-05-01."),
        });
        let weak = reranker.rerank_score(SearchRerankInput {
            query_text: "Who owns the Apollo launch blocker and what is the deadline?",
            query_vector: None,
            candidate_id: 2,
            lexical_score: 0,
            vector_score: 0,
            base_score: 1,
            metadata: None,
            payload: Some(b"Launch celebration notes."),
        });

        assert!(matched > weak);
    }

    #[test]
    fn weighted_reranker_rewards_query_scope_mapping_matches() {
        let reranker = WeightedScoreReranker::default();
        let matched = reranker.rerank_score(SearchRerankInput {
            query_text: "What did Slack say about the Apollo rollout?",
            query_vector: None,
            candidate_id: 1,
            lexical_score: 0,
            vector_score: 0,
            base_score: 1,
            metadata: None,
            payload: Some(b"source=slack\nproject=Apollo\ntopic=rollout\n\nLaunch update."),
        });
        let weak = reranker.rerank_score(SearchRerankInput {
            query_text: "What did Slack say about the Apollo rollout?",
            query_vector: None,
            candidate_id: 2,
            lexical_score: 0,
            vector_score: 0,
            base_score: 1,
            metadata: None,
            payload: Some(b"source=gmail\nproject=Hermes\n\nOffice schedule."),
        });

        assert!(matched > weak);
    }

    #[test]
    fn weighted_reranker_uses_descriptor_metadata_before_payload_scope_mapping() {
        let reranker = WeightedScoreReranker::default();
        let descriptor_metadata =
            CellMetadata::from_payload(b"source=jira\nproject=Apollo\n\nLaunch update.");
        let matched = reranker.rerank_score(SearchRerankInput {
            query_text: "What did Jira say about the Apollo rollout?",
            query_vector: None,
            candidate_id: 1,
            lexical_score: 0,
            vector_score: 0,
            base_score: 1,
            metadata: Some(&descriptor_metadata),
            payload: Some(b"source=gmail\nproject=Hermes\n\nLaunch update."),
        });
        let weak = reranker.rerank_score(SearchRerankInput {
            query_text: "What did Jira say about the Apollo rollout?",
            query_vector: None,
            candidate_id: 2,
            lexical_score: 0,
            vector_score: 0,
            base_score: 1,
            metadata: None,
            payload: Some(b"source=gmail\nproject=Hermes\n\nLaunch update."),
        });

        assert!(matched > weak);
    }

    #[test]
    fn weighted_reranker_rewards_numeric_condition_matches() {
        let reranker = WeightedScoreReranker::default();
        let matched = reranker.rerank_score(SearchRerankInput {
            query_text: "What p95 latency threshold must be under 200 ms?",
            query_vector: None,
            candidate_id: 1,
            lexical_score: 0,
            vector_score: 0,
            base_score: 1,
            metadata: None,
            payload: Some(b"p95 latency threshold is 180 ms for the EU route."),
        });
        let weak = reranker.rerank_score(SearchRerankInput {
            query_text: "What p95 latency threshold must be under 200 ms?",
            query_vector: None,
            candidate_id: 2,
            lexical_score: 0,
            vector_score: 0,
            base_score: 1,
            metadata: None,
            payload: Some(b"p95 latency threshold is 280 ms for the EU route."),
        });

        assert!(matched > weak);
    }

    #[test]
    fn default_reranker_is_not_enterprise_rag_calibrated() {
        let default = WeightedScoreReranker::default();
        let calibrated = WeightedScoreReranker::enterprise_rag_calibrated()
            .calibrated_for_query("Which approach is recommended for delayed adoption?");

        assert!(!default.calibrate_by_question_type);
        assert_ne!(default.vector_weight, calibrated.vector_weight);
        assert!(calibrated.vector_weight > calibrated.lexical_weight);
    }

    #[test]
    fn calibration_profiles_are_selected_from_question_text() {
        let semantic = rerank_calibration_profile(
            "Which approach is recommended for delayed enterprise rollout adoption?",
            WeightedScoreReranker::default(),
        );
        let constrained = rerank_calibration_profile(
            "Which incident where p95 latency threshold was under 200 ms?",
            WeightedScoreReranker::default(),
        );

        assert!(semantic.reranker.vector_weight > semantic.reranker.lexical_weight);
        assert!(semantic.rrf_weights.vector_q16 > semantic.rrf_weights.lexical_q16);
        assert!(constrained.reranker.lexical_weight > constrained.reranker.vector_weight);
        assert!(constrained.reranker.condition_payload_bonus > 1);
    }

    #[test]
    fn calibration_promotes_complex_explanatory_basic_queries_to_vector_heavy_profile() {
        let profile = rerank_calibration_profile(
            "In our GPU inference runtime, what change was introduced to cut the worst-case temporary device-memory spike when short and long requests are interleaved?",
            WeightedScoreReranker::default(),
        );

        assert_eq!(profile.question_type.as_str(), "semantic");
        assert!(profile.rrf_weights.vector_q16 > profile.rrf_weights.lexical_q16);
    }

    #[test]
    fn calibrated_rrf_weights_do_not_use_one_fixed_profile() {
        let semantic =
            calibrated_hybrid_rrf_weights("Which approach is recommended for delayed adoption?");
        let basic = calibrated_hybrid_rrf_weights("What are the default billing migration values?");

        assert_ne!(semantic, basic);
        assert!(semantic.vector_q16 > semantic.lexical_q16);
        assert!(basic.lexical_q16 > basic.vector_q16);
    }
}
