use super::super::frozen_weights::{self, FrozenRerankProfile};
use super::super::{classify_enterprise_rag_question_type, EnterpriseRagQuestionType};
use super::types::{HybridRrfWeights, RerankCalibrationProfile, WeightedScoreReranker, Q16_ONE};

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
    let profile = match question_type {
        EnterpriseRagQuestionType::Basic => frozen_weights::BASIC_PROFILE,
        EnterpriseRagQuestionType::Semantic | EnterpriseRagQuestionType::IntraDocumentReasoning => {
            frozen_weights::SEMANTIC_PROFILE
        }
        EnterpriseRagQuestionType::ProjectRelated => frozen_weights::PROJECT_RELATED_PROFILE,
        EnterpriseRagQuestionType::Constrained => frozen_weights::CONSTRAINED_PROFILE,
        EnterpriseRagQuestionType::ConflictingInfo => frozen_weights::CONFLICTING_INFO_PROFILE,
        EnterpriseRagQuestionType::Completeness => frozen_weights::COMPLETENESS_PROFILE,
        EnterpriseRagQuestionType::HighLevel => frozen_weights::HIGH_LEVEL_PROFILE,
        EnterpriseRagQuestionType::InfoNotFound | EnterpriseRagQuestionType::Miscellaneous => {
            frozen_weights::INFO_NOT_FOUND_PROFILE
        }
    };
    let rrf_weights = apply_profile(&mut reranker, profile);
    RerankCalibrationProfile {
        question_type,
        rrf_weights,
        reranker,
    }
}

fn apply_profile(
    reranker: &mut WeightedScoreReranker,
    profile: FrozenRerankProfile,
) -> HybridRrfWeights {
    reranker.lexical_weight = profile.lexical_weight;
    reranker.vector_weight = profile.vector_weight;
    reranker.anchor_payload_bonus = profile.anchor_payload_bonus;
    reranker.source_hint_payload_bonus = profile.source_hint_payload_bonus;
    reranker.scope_mapping_metadata_bonus = profile.scope_mapping_metadata_bonus;
    reranker.condition_payload_bonus = profile.condition_payload_bonus;
    reranker.no_evidence_overlap_score_q16 = profile.no_evidence_overlap_score_q16;
    HybridRrfWeights {
        lexical_q16: profile.rrf_lexical_q16,
        vector_q16: Q16_ONE - profile.rrf_lexical_q16,
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
