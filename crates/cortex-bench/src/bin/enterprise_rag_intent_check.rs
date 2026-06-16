use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use cortex_engine::{
    classify_enterprise_rag_question, EnterpriseRagIntentClassification, EnterpriseRagQuestionType,
};
use serde_json::json;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args = Args::parse(env::args().skip(1))?;
    let rows = read_questions(&args.questions)?;
    let selected = select_rows(&rows, args.offset, args.limit);
    if selected.is_empty() {
        return Err("selected question set is empty".to_owned());
    }
    let report = evaluate(selected, &args)?;
    let report_json = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("failed to serialize report: {error}"))?;
    if let Some(output) = &args.output {
        if let Some(parent) = output.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
        }
        fs::write(output, format!("{report_json}\n"))
            .map_err(|error| format!("failed to write {}: {error}", output.display()))?;
    }
    println!("{report_json}");
    if report["status"] == "passed" {
        Ok(())
    } else {
        Err("enterprise_rag_intent_check failed".to_owned())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Args {
    questions: PathBuf,
    output: Option<PathBuf>,
    limit: Option<usize>,
    offset: usize,
    min_accuracy_pct: u64,
}

impl Args {
    fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut questions = None;
        let mut output = None;
        let mut limit = None;
        let mut offset = 0usize;
        let mut min_accuracy_pct = 90u64;
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--questions" | "--questions-file" => {
                    questions = Some(PathBuf::from(next_value(&mut args, &arg)?));
                }
                "--output" | "--report" => {
                    output = Some(PathBuf::from(next_value(&mut args, &arg)?))
                }
                "--limit" => limit = Some(parse_usize(&next_value(&mut args, &arg)?, &arg)?),
                "--offset" => offset = parse_usize(&next_value(&mut args, &arg)?, &arg)?,
                "--min-accuracy-pct" => {
                    min_accuracy_pct = parse_u64(&next_value(&mut args, &arg)?, &arg)?;
                }
                "--help" | "-h" => return Err(usage()),
                value => return Err(format!("unknown option: {value}")),
            }
        }
        Ok(Self {
            questions: questions.ok_or_else(|| "--questions is required".to_owned())?,
            output,
            limit,
            offset,
            min_accuracy_pct,
        })
    }
}

#[derive(Clone, Debug)]
struct QuestionRow {
    question_id: String,
    question: String,
    expected_type: EnterpriseRagQuestionType,
}

fn read_questions(path: &PathBuf) -> Result<Vec<QuestionRow>, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let mut rows = Vec::new();
    for (line_index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let value: serde_json::Value = serde_json::from_str(line)
            .map_err(|error| format!("invalid JSON on line {}: {error}", line_index + 1))?;
        let question_id = string_field(&value, "question_id", line_index)?;
        let question = string_field(&value, "question", line_index)?;
        let raw_type = string_field(&value, "question_type", line_index)?;
        let expected_type = EnterpriseRagQuestionType::parse(&raw_type).ok_or_else(|| {
            format!(
                "unsupported question_type '{}' on line {}",
                raw_type,
                line_index + 1
            )
        })?;
        rows.push(QuestionRow {
            question_id,
            question,
            expected_type,
        });
    }
    Ok(rows)
}

fn select_rows(rows: &[QuestionRow], offset: usize, limit: Option<usize>) -> &[QuestionRow] {
    if offset >= rows.len() {
        return &[];
    }
    let end = limit
        .map(|limit| offset.saturating_add(limit).min(rows.len()))
        .unwrap_or(rows.len());
    &rows[offset..end]
}

fn evaluate(rows: &[QuestionRow], args: &Args) -> Result<serde_json::Value, String> {
    let mut correct = 0u64;
    let mut by_type = BTreeMap::<String, TypeStats>::new();
    let mut confusion = BTreeMap::<String, BTreeMap<String, u64>>::new();
    let mut details = Vec::with_capacity(rows.len());
    for row in rows {
        let classification = classify_enterprise_rag_question(&row.question);
        let predicted = classification.question_type;
        let matched = predicted == row.expected_type;
        if matched {
            correct += 1;
        }
        by_type
            .entry(row.expected_type.as_str().to_owned())
            .or_default()
            .record(matched);
        *confusion
            .entry(row.expected_type.as_str().to_owned())
            .or_default()
            .entry(predicted.as_str().to_owned())
            .or_default() += 1;
        details.push(detail(row, classification, matched));
    }
    let accuracy_pct = correct * 100 / rows.len() as u64;
    let mut errors = Vec::new();
    if accuracy_pct < args.min_accuracy_pct {
        errors.push(format!(
            "accuracy_pct {accuracy_pct} < {}",
            args.min_accuracy_pct
        ));
    }
    Ok(json!({
        "schema_version": "cortexdb.enterprise_rag_intent_check.v1",
        "questions": rows.len(),
        "correct": correct,
        "accuracy_pct": accuracy_pct,
        "min_accuracy_pct": args.min_accuracy_pct,
        "status": if errors.is_empty() { "passed" } else { "failed" },
        "errors": errors,
        "by_question_type": by_type
            .into_iter()
            .map(|(question_type, stats)| {
                let accuracy_pct = stats
                    .correct
                    .saturating_mul(100)
                    .checked_div(stats.total)
                    .unwrap_or(0);
                (question_type, json!({
                    "total": stats.total,
                    "correct": stats.correct,
                    "accuracy_pct": accuracy_pct,
                }))
            })
            .collect::<BTreeMap<_, _>>(),
        "confusion": confusion,
        "details": details,
    }))
}

fn detail(
    row: &QuestionRow,
    classification: EnterpriseRagIntentClassification,
    matched: bool,
) -> serde_json::Value {
    json!({
        "question_id": row.question_id,
        "question": row.question,
        "expected_question_type": row.expected_type.as_str(),
        "predicted_question_type": classification.question_type.as_str(),
        "matched": matched,
        "confidence_q16": classification.confidence_q16,
        "matched_signals": classification.matched_signals,
    })
}

#[derive(Clone, Copy, Debug, Default)]
struct TypeStats {
    total: u64,
    correct: u64,
}

impl TypeStats {
    fn record(&mut self, matched: bool) {
        self.total += 1;
        if matched {
            self.correct += 1;
        }
    }
}

fn string_field(
    value: &serde_json::Value,
    field: &str,
    line_index: usize,
) -> Result<String, String> {
    value
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("missing non-empty {field} on line {}", line_index + 1))
}

fn next_value(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, String> {
    args.next()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn parse_usize(value: &str, flag: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|_| format!("{flag} expects a non-negative integer"))
}

fn parse_u64(value: &str, flag: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|_| format!("{flag} expects a non-negative integer"))
}

fn usage() -> String {
    "usage: enterprise_rag_intent_check --questions PATH [--output PATH] [--offset N] [--limit N] [--min-accuracy-pct N]".to_owned()
}

#[cfg(test)]
mod tests {
    use super::Args;
    use std::path::PathBuf;

    #[test]
    fn parse_args_accepts_named_paths_and_threshold() {
        let args = Args::parse([
            "--questions".to_owned(),
            "questions.jsonl".to_owned(),
            "--output".to_owned(),
            "report.json".to_owned(),
            "--limit".to_owned(),
            "50".to_owned(),
            "--offset".to_owned(),
            "10".to_owned(),
            "--min-accuracy-pct".to_owned(),
            "80".to_owned(),
        ])
        .unwrap();

        assert_eq!(args.questions, PathBuf::from("questions.jsonl"));
        assert_eq!(args.output, Some(PathBuf::from("report.json")));
        assert_eq!(args.limit, Some(50));
        assert_eq!(args.offset, 10);
        assert_eq!(args.min_accuracy_pct, 80);
    }

    #[test]
    fn parse_args_rejects_missing_questions() {
        assert!(Args::parse(Vec::<String>::new()).is_err());
    }
}
