use std::collections::BTreeMap;

use cortex_engine::search::{
    rerank_calibration_profile, EnterpriseRagQuestionType, RerankCalibrationProfile,
    WeightedScoreReranker,
};
use serde_json::{json, Value};

use super::args::Args;
use super::questions::QuestionRow;

#[derive(Clone, Copy, Debug, Default)]
struct TypeStats {
    total: u64,
    calibrated: u64,
    vector_heavy: u64,
    lexical_heavy: u64,
    condition_boosted: u64,
}

pub(super) fn build_report(rows: &[QuestionRow], args: &Args) -> Value {
    let baseline = WeightedScoreReranker::fixed_default();
    let mut calibrated = 0u64;
    let mut semantic_rows = 0u64;
    let mut semantic_vector_heavy = 0u64;
    let mut constrained_rows = 0u64;
    let mut constrained_condition_boosted = 0u64;
    let mut by_expected_type = BTreeMap::<String, TypeStats>::new();
    let mut by_predicted_type = BTreeMap::<String, TypeStats>::new();
    let mut details = Vec::with_capacity(rows.len());
    for row in rows {
        let profile = rerank_calibration_profile(&row.question, WeightedScoreReranker::default());
        let is_calibrated =
            profile.reranker != baseline || profile.rrf_weights.vector_q16 != 32_767;
        if is_calibrated {
            calibrated += 1;
        }
        if let Some(expected) = row.question_type {
            by_expected_type
                .entry(expected.as_str().to_owned())
                .or_default()
                .record(&profile, is_calibrated);
            if matches!(
                expected,
                EnterpriseRagQuestionType::Semantic
                    | EnterpriseRagQuestionType::IntraDocumentReasoning
                    | EnterpriseRagQuestionType::HighLevel
            ) {
                semantic_rows += 1;
                if profile.rrf_weights.vector_q16 > profile.rrf_weights.lexical_q16 {
                    semantic_vector_heavy += 1;
                }
            }
            if expected == EnterpriseRagQuestionType::Constrained {
                constrained_rows += 1;
                if profile.reranker.condition_payload_bonus > baseline.condition_payload_bonus {
                    constrained_condition_boosted += 1;
                }
            }
        }
        by_predicted_type
            .entry(profile.question_type.as_str().to_owned())
            .or_default()
            .record(&profile, is_calibrated);
        details.push(detail(row, &profile, is_calibrated));
    }
    let calibrated_pct = percent(calibrated, rows.len() as u64);
    let semantic_vector_pct = percent(semantic_vector_heavy, semantic_rows);
    let constrained_condition_pct = percent(constrained_condition_boosted, constrained_rows);
    let mut errors = Vec::new();
    if calibrated_pct < args.min_calibrated_pct {
        errors.push(format!(
            "calibrated_pct {calibrated_pct} < {}",
            args.min_calibrated_pct
        ));
    }
    if semantic_rows > 0 && semantic_vector_pct < args.min_semantic_vector_pct {
        errors.push(format!(
            "semantic_vector_pct {semantic_vector_pct} < {}",
            args.min_semantic_vector_pct
        ));
    }
    if constrained_rows > 0 && constrained_condition_pct < args.min_constrained_condition_pct {
        errors.push(format!(
            "constrained_condition_pct {constrained_condition_pct} < {}",
            args.min_constrained_condition_pct
        ));
    }
    json!({
        "schema_version": "cortexdb.enterprise_rag_calibration_check.v1",
        "questions": rows.len(),
        "calibrated_questions": calibrated,
        "calibrated_pct": calibrated_pct,
        "min_calibrated_pct": args.min_calibrated_pct,
        "semantic_rows": semantic_rows,
        "semantic_vector_heavy": semantic_vector_heavy,
        "semantic_vector_pct": semantic_vector_pct,
        "min_semantic_vector_pct": args.min_semantic_vector_pct,
        "constrained_rows": constrained_rows,
        "constrained_condition_boosted": constrained_condition_boosted,
        "constrained_condition_pct": constrained_condition_pct,
        "min_constrained_condition_pct": args.min_constrained_condition_pct,
        "by_expected_type": stats_map(by_expected_type),
        "by_predicted_type": stats_map(by_predicted_type),
        "details": details,
        "status": if errors.is_empty() { "passed" } else { "failed" },
        "errors": errors,
    })
}

fn percent(value: u64, total: u64) -> u64 {
    value.saturating_mul(100).checked_div(total).unwrap_or(0)
}

fn detail(row: &QuestionRow, profile: &RerankCalibrationProfile, calibrated: bool) -> Value {
    json!({
        "question_id": row.question_id,
        "question": row.question,
        "expected_question_type": row.question_type.map(EnterpriseRagQuestionType::as_str),
        "predicted_question_type": profile.question_type.as_str(),
        "calibrated": calibrated,
        "rrf_lexical_q16": profile.rrf_weights.lexical_q16,
        "rrf_vector_q16": profile.rrf_weights.vector_q16,
        "lexical_weight": profile.reranker.lexical_weight,
        "vector_weight": profile.reranker.vector_weight,
        "condition_payload_bonus": profile.reranker.condition_payload_bonus,
        "scope_mapping_metadata_bonus": profile.reranker.scope_mapping_metadata_bonus,
        "no_evidence_overlap_score_q16": profile.reranker.no_evidence_overlap_score_q16,
    })
}

fn stats_map(values: BTreeMap<String, TypeStats>) -> BTreeMap<String, Value> {
    values
        .into_iter()
        .map(|(key, stats)| {
            (
                key,
                json!({
                    "total": stats.total,
                    "calibrated": stats.calibrated,
                    "calibrated_pct": percent(stats.calibrated, stats.total),
                    "vector_heavy": stats.vector_heavy,
                    "lexical_heavy": stats.lexical_heavy,
                    "condition_boosted": stats.condition_boosted,
                }),
            )
        })
        .collect()
}

impl TypeStats {
    fn record(&mut self, profile: &RerankCalibrationProfile, calibrated: bool) {
        self.total += 1;
        if calibrated {
            self.calibrated += 1;
        }
        if profile.rrf_weights.vector_q16 > profile.rrf_weights.lexical_q16 {
            self.vector_heavy += 1;
        }
        if profile.rrf_weights.lexical_q16 > profile.rrf_weights.vector_q16 {
            self.lexical_heavy += 1;
        }
        if profile.reranker.condition_payload_bonus > 1 {
            self.condition_boosted += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use cortex_engine::search::EnterpriseRagQuestionType;

    use super::build_report;
    use crate::args::parse_args;
    use crate::questions::QuestionRow;

    #[test]
    fn report_passes_for_text_derived_semantic_and_constrained_profiles() {
        let rows = vec![
            QuestionRow {
                question_id: "q1".to_owned(),
                question: "Which approach is recommended for delayed enterprise adoption?"
                    .to_owned(),
                question_type: Some(EnterpriseRagQuestionType::Semantic),
            },
            QuestionRow {
                question_id: "q2".to_owned(),
                question: "Which incident where p95 latency threshold was under 200 ms?".to_owned(),
                question_type: Some(EnterpriseRagQuestionType::Constrained),
            },
        ];
        let args = parse_args(
            [
                "--questions",
                "questions.jsonl",
                "--output",
                "report.json",
                "--min-calibrated-pct",
                "50",
                "--min-semantic-vector-pct",
                "50",
                "--min-constrained-condition-pct",
                "50",
            ]
            .into_iter()
            .map(str::to_owned),
        )
        .unwrap();
        let report = build_report(&rows, &args);

        assert_eq!(report["status"], "passed");
    }
}
