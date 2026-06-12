use super::scores::CategoryScores;
use super::types::{EnterpriseRagIntentClassification, EnterpriseRagQuestionType};

mod constraints;
mod overview;
mod project;
mod semantic;

pub fn classify_enterprise_rag_question_type(query: &str) -> EnterpriseRagQuestionType {
    classify_enterprise_rag_question(query).question_type
}

pub fn classify_enterprise_rag_question(query: &str) -> EnterpriseRagIntentClassification {
    let lower = query.to_ascii_lowercase();
    let mut scores = CategoryScores::default();

    overview::score_high_level(&lower, &mut scores);
    overview::score_miscellaneous(&lower, &mut scores);
    overview::score_info_not_found(&lower, &mut scores);
    constraints::score_completeness(&lower, &mut scores);
    constraints::score_conflicting_info(&lower, &mut scores);
    constraints::score_constrained(&lower, &mut scores);
    project::score_project_related(&lower, &mut scores);
    project::score_intra_document_reasoning(&lower, &mut scores);
    semantic::score_semantic(&lower, &mut scores);
    semantic::score_basic(&lower, &mut scores);

    let question_type = scores.best_type();
    EnterpriseRagIntentClassification {
        question_type,
        confidence_q16: scores.confidence_q16(question_type),
        matched_signals: scores.signals(question_type),
    }
}
