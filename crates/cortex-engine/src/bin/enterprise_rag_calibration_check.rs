use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use cortex_engine::search::{
    rerank_calibration_profile, EnterpriseRagQuestionType, RerankCalibrationProfile,
    WeightedScoreReranker,
};
use serde_json::{json, Value};

#[derive(Clone, Debug, PartialEq, Eq)]
struct Args {
    questions: PathBuf,
    output: PathBuf,
    limit: Option<usize>,
    offset: usize,
    min_calibrated_pct: u64,
    min_semantic_vector_pct: u64,
    min_constrained_condition_pct: u64,
}

#[derive(Clone, Debug)]
struct QuestionRow {
    question_id: String,
    question: String,
    question_type: Option<EnterpriseRagQuestionType>,
}

#[derive(Clone, Copy, Debug, Default)]
struct TypeStats {
    total: u64,
    calibrated: u64,
    vector_heavy: u64,
    lexical_heavy: u64,
    condition_boosted: u64,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = parse_args(env::args().skip(1))?;
    let mut rows = read_questions(&args.questions)?;
    if args.offset > rows.len() {
        rows.clear();
    } else if args.offset > 0 {
        rows = rows.split_off(args.offset);
    }
    if let Some(limit) = args.limit {
        rows.truncate(limit);
    }
    if rows.is_empty() {
        return Err("selected question set is empty".to_owned());
    }
    let report = build_report(&rows, &args);
    if let Some(parent) = args.output.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("failed to encode report: {error}"))?;
    fs::write(&args.output, format!("{text}\n"))
        .map_err(|error| format!("failed to write {}: {error}", args.output.display()))?;
    println!("{}", summary(&report, &args.output));
    let status = report
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("failed");
    if status != "passed" {
        return Err(format!(
            "calibration check failed; see {}",
            args.output.display()
        ));
    }
    Ok(())
}

fn parse_args(mut args: impl Iterator<Item = String>) -> Result<Args, String> {
    let mut questions = None;
    let mut output = None;
    let mut limit = None;
    let mut offset = 0usize;
    let mut min_calibrated_pct = 95u64;
    let mut min_semantic_vector_pct = 60u64;
    let mut min_constrained_condition_pct = 70u64;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--questions" | "--questions-file" => {
                questions = Some(PathBuf::from(next_value(&mut args, &arg)?));
            }
            "--output" | "--report" => output = Some(PathBuf::from(next_value(&mut args, &arg)?)),
            "--limit" => limit = Some(parse_usize(&next_value(&mut args, &arg)?, &arg)?),
            "--offset" => offset = parse_usize(&next_value(&mut args, &arg)?, &arg)?,
            "--min-calibrated-pct" => {
                min_calibrated_pct = parse_u64(&next_value(&mut args, &arg)?, &arg)?.min(100)
            }
            "--min-semantic-vector-pct" => {
                min_semantic_vector_pct = parse_u64(&next_value(&mut args, &arg)?, &arg)?.min(100)
            }
            "--min-constrained-condition-pct" => {
                min_constrained_condition_pct =
                    parse_u64(&next_value(&mut args, &arg)?, &arg)?.min(100)
            }
            "--help" | "-h" => return Err(usage()),
            _ => return Err(format!("unknown argument {arg}\n{}", usage())),
        }
    }
    Ok(Args {
        questions: questions.ok_or_else(usage)?,
        output: output.ok_or_else(usage)?,
        limit,
        offset,
        min_calibrated_pct,
        min_semantic_vector_pct,
        min_constrained_condition_pct,
    })
}

fn next_value(args: &mut impl Iterator<Item = String>, name: &str) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("{name} requires a value\n{}", usage()))
}

fn parse_usize(value: &str, name: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|error| format!("{name} expects a positive integer: {error}"))
}

fn parse_u64(value: &str, name: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|error| format!("{name} expects an integer: {error}"))
}

fn usage() -> String {
    "usage: enterprise_rag_calibration_check --questions <questions.jsonl> --output <report.json> [--limit N] [--offset N] [--min-calibrated-pct PCT] [--min-semantic-vector-pct PCT] [--min-constrained-condition-pct PCT]".to_owned()
}

fn read_questions(path: &Path) -> Result<Vec<QuestionRow>, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let mut rows = Vec::new();
    for (line_index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(line)
            .map_err(|error| format!("invalid JSONL line {}: {error}", line_index + 1))?;
        let raw_type = value.get("question_type").and_then(Value::as_str);
        rows.push(QuestionRow {
            question_id: required_str(&value, "question_id", line_index)?,
            question: required_str(&value, "question", line_index)?,
            question_type: raw_type.and_then(EnterpriseRagQuestionType::parse),
        });
    }
    Ok(rows)
}

fn required_str(value: &Value, key: &str, line_index: usize) -> Result<String, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty())
        .map(str::to_owned)
        .ok_or_else(|| format!("line {} missing string field {key}", line_index + 1))
}

fn build_report(rows: &[QuestionRow], args: &Args) -> Value {
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
        "scope_mapping_payload_bonus": profile.reranker.scope_mapping_payload_bonus,
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

fn summary(report: &Value, output: &Path) -> String {
    json!({
        "questions": report.get("questions").and_then(Value::as_u64).unwrap_or(0),
        "calibrated_pct": report.get("calibrated_pct").and_then(Value::as_u64).unwrap_or(0),
        "semantic_vector_pct": report.get("semantic_vector_pct").and_then(Value::as_u64).unwrap_or(0),
        "constrained_condition_pct": report.get("constrained_condition_pct").and_then(Value::as_u64).unwrap_or(0),
        "status": report.get("status").and_then(Value::as_str).unwrap_or("failed"),
        "output": output.display().to_string(),
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::{build_report, parse_args, QuestionRow};
    use cortex_engine::search::EnterpriseRagQuestionType;

    #[test]
    fn parse_args_accepts_thresholds() {
        let args = parse_args(
            [
                "--questions",
                "questions.jsonl",
                "--output",
                "report.json",
                "--limit",
                "50",
                "--min-calibrated-pct",
                "80",
            ]
            .into_iter()
            .map(str::to_owned),
        )
        .unwrap();

        assert_eq!(args.limit, Some(50));
        assert_eq!(args.min_calibrated_pct, 80);
    }

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
