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
            reranker.scope_mapping_metadata_bonus = 2;
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
