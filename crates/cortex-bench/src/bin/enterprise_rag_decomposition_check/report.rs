use std::collections::BTreeMap;

use cortex_engine::search::{decompose_enterprise_rag_question, QuestionDecomposition};
use serde_json::{json, Value};

use super::args::Args;
use super::questions::QuestionRow;

#[derive(Clone, Copy, Debug, Default)]
struct TypeStats {
    total: u64,
    expected_multi: u64,
    decomposed_multi: u64,
}

pub(super) fn build_report(rows: &[QuestionRow], args: &Args) -> Result<Value, String> {
    if rows.is_empty() {
        return Err("no questions selected".to_owned());
    }
    let mut expected_multi = 0u64;
    let mut decomposed_multi = 0u64;
    let mut by_type = BTreeMap::<String, TypeStats>::new();
    let mut details = Vec::with_capacity(rows.len());
    for row in rows {
        let decomposition = decompose_enterprise_rag_question(&row.question);
        let expected = expected_multi_requirement(row);
        let actual = decomposition.multi_requirement;
        if expected {
            expected_multi += 1;
            if actual {
                decomposed_multi += 1;
            }
        }
        let stats = by_type.entry(row.question_type.clone()).or_default();
        stats.total += 1;
        if expected {
            stats.expected_multi += 1;
        }
        if expected && actual {
            stats.decomposed_multi += 1;
        }
        details.push(detail(row, expected, actual, &decomposition));
    }
    let multi_coverage_pct = decomposed_multi
        .saturating_mul(100)
        .checked_div(expected_multi)
        .unwrap_or(100);
    let average_requirement_count = rows
        .iter()
        .map(|row| {
            decompose_enterprise_rag_question(&row.question)
                .requirements
                .len() as u64
        })
        .sum::<u64>()
        / rows.len() as u64;
    let mut errors = Vec::new();
    if multi_coverage_pct < args.min_multi_coverage_pct {
        errors.push(format!(
            "multi_coverage_pct {multi_coverage_pct} < {}",
            args.min_multi_coverage_pct
        ));
    }
    Ok(json!({
        "schema_version": "cortexdb.enterprise_rag_decomposition_check.v1",
        "questions": rows.len(),
        "expected_multi_questions": expected_multi,
        "decomposed_multi_questions": decomposed_multi,
        "multi_coverage_pct": multi_coverage_pct,
        "min_multi_coverage_pct": args.min_multi_coverage_pct,
        "average_requirement_count": average_requirement_count,
        "status": if errors.is_empty() { "passed" } else { "failed" },
        "errors": errors,
        "by_question_type": by_type
            .into_iter()
            .map(|(question_type, stats)| {
                let coverage = stats
                    .decomposed_multi
                    .saturating_mul(100)
                    .checked_div(stats.expected_multi)
                    .unwrap_or(100);
                (question_type, json!({
                    "total": stats.total,
                    "expected_multi": stats.expected_multi,
                    "decomposed_multi": stats.decomposed_multi,
                    "multi_coverage_pct": coverage,
                }))
            })
            .collect::<BTreeMap<_, _>>(),
        "details": details,
    }))
}

fn expected_multi_requirement(row: &QuestionRow) -> bool {
    if row.answer_fact_count > 1 || row.expected_doc_count > 1 {
        return true;
    }
    let question_type = row.question_type.as_str();
    if matches!(
        question_type,
        "completeness"
            | "project_related"
            | "constrained"
            | "conflicting_info"
            | "intra_document_reasoning"
    ) {
        return true;
    }
    let lower = row.question.to_ascii_lowercase();
    lower.contains(" and what ")
        || lower.contains(" and how ")
        || lower.contains(" and when ")
        || lower.contains(" including ")
        || lower.contains(" along with ")
        || lower.contains(" both ")
        || lower.contains(" compare ")
        || lower.contains("list all")
        || lower.contains("what are")
        || lower.contains(',')
        || lower.contains(';')
}

fn detail(
    row: &QuestionRow,
    expected_multi: bool,
    decomposed_multi: bool,
    decomposition: &QuestionDecomposition,
) -> Value {
    json!({
        "question_id": row.question_id,
        "question_type": row.question_type,
        "question": row.question,
        "expected_multi": expected_multi,
        "decomposed_multi": decomposed_multi,
        "multi_requirement": decomposition.multi_requirement,
        "anchors": decomposition.anchors,
        "slots": decomposition.slots,
        "subquestions": decomposition.subquestions,
        "requirements": decomposition
            .requirements
            .iter()
            .map(|requirement| json!({
                "id": requirement.id,
                "kind": requirement.kind.as_str(),
                "text": requirement.text,
                "tokens": requirement.tokens,
            }))
            .collect::<Vec<_>>(),
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{build_report, expected_multi_requirement};
    use crate::args::Args;
    use crate::questions::QuestionRow;

    #[test]
    fn expected_multi_uses_offline_labels_and_text_only_markers() {
        assert!(expected_multi_requirement(&QuestionRow {
            question_id: "q1".to_owned(),
            question: "What caused the incident and how was it mitigated?".to_owned(),
            question_type: "basic".to_owned(),
            answer_fact_count: 1,
            expected_doc_count: 1,
        }));
        assert!(expected_multi_requirement(&QuestionRow {
            question_id: "q2".to_owned(),
            question: "Who owns launch?".to_owned(),
            question_type: "project_related".to_owned(),
            answer_fact_count: 0,
            expected_doc_count: 0,
        }));
    }

    #[test]
    fn report_passes_when_expected_multi_questions_are_decomposed() {
        let args = Args {
            questions: "questions.jsonl".into(),
            output: "report.json".into(),
            limit: None,
            offset: 0,
            min_multi_coverage_pct: 80,
        };
        let rows = vec![QuestionRow {
            question_id: "q1".to_owned(),
            question: "Who owns the Apollo blocker and what is the deadline?".to_owned(),
            question_type: "project_related".to_owned(),
            answer_fact_count: 2,
            expected_doc_count: 1,
        }];

        let report = build_report(&rows, &args).unwrap();

        assert_eq!(report["status"], json!("passed"));
        assert_eq!(report["multi_coverage_pct"], json!(100));
    }
}
