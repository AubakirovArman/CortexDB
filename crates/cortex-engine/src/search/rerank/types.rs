use crate::query::CellMetadata;

use super::super::frozen_weights;
use super::super::EnterpriseRagQuestionType;

pub(super) const Q16_ONE: u32 = frozen_weights::Q16_ONE_U32;

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
            lexical_q16: frozen_weights::RRF_LEXICAL_HEAVY_Q16,
            vector_q16: Q16_ONE - frozen_weights::RRF_LEXICAL_HEAVY_Q16,
        }
    }

    pub fn vector_heavy() -> Self {
        Self {
            lexical_q16: frozen_weights::RRF_VECTOR_HEAVY_Q16,
            vector_q16: Q16_ONE - frozen_weights::RRF_VECTOR_HEAVY_Q16,
        }
    }

    pub fn balanced() -> Self {
        Self {
            lexical_q16: frozen_weights::RRF_BALANCED_LEXICAL_Q16,
            vector_q16: Q16_ONE - frozen_weights::RRF_BALANCED_LEXICAL_Q16,
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
    pub scope_mapping_metadata_bonus: u64,
    pub condition_payload_bonus: u64,
    pub no_evidence_overlap_score_q16: u16,
}

impl Default for WeightedScoreReranker {
    fn default() -> Self {
        Self {
            calibrate_by_question_type: false,
            lexical_weight: frozen_weights::RERANK_DEFAULT_LEXICAL_WEIGHT,
            vector_weight: frozen_weights::RERANK_DEFAULT_VECTOR_WEIGHT,
            anchor_payload_bonus: frozen_weights::RERANK_DEFAULT_ANCHOR_PAYLOAD_BONUS,
            source_hint_payload_bonus: frozen_weights::RERANK_DEFAULT_SOURCE_HINT_PAYLOAD_BONUS,
            scope_mapping_metadata_bonus:
                frozen_weights::RERANK_DEFAULT_SCOPE_MAPPING_METADATA_BONUS,
            condition_payload_bonus: frozen_weights::RERANK_DEFAULT_CONDITION_PAYLOAD_BONUS,
            no_evidence_overlap_score_q16: frozen_weights::RERANK_DEFAULT_NO_EVIDENCE_OVERLAP_Q16,
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
}
