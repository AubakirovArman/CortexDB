use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use cortex_engine::search::extract_query_conditions;
use serde_json::{json, Value};

#[derive(Clone, Debug, PartialEq, Eq)]
struct Args {
    questions: PathBuf,
    output: PathBuf,
    limit: Option<usize>,
    offset: usize,
    min_constrained_coverage_pct: u64,
}

#[derive(Clone, Debug)]
struct QuestionRow {
    question_id: String,
    question: String,
    question_type: String,
}

#[derive(Clone, Copy, Debug, Default)]
struct TypeStats {
    total: u64,
    structured: u64,
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
    let report = build_report(&rows, &args)?;
    if let Some(parent) = args.output.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("failed to encode report: {error}"))?;
    fs::write(&args.output, format!("{text}\n"))
        .map_err(|error| format!("failed to write {}: {error}", args.output.display()))?;
    println!("{}", summary(&report, &args.output));
    if report["status"] != "passed" {
        return Err(format!(
            "condition check failed; see {}",
            args.output.display()
        ));
    }
    Ok(())
}

fn summary(report: &Value, output: &Path) -> String {
    json!({
        "questions": report.get("questions").and_then(Value::as_u64).unwrap_or(0),
        "constrained_questions": report
            .get("constrained_questions")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        "structured_constrained_questions": report
            .get("structured_constrained_questions")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        "constrained_coverage_pct": report
            .get("constrained_coverage_pct")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        "status": report.get("status").and_then(Value::as_str).unwrap_or("failed"),
        "output": output.display().to_string(),
    })
    .to_string()
}

fn parse_args(mut args: impl Iterator<Item = String>) -> Result<Args, String> {
    let mut questions = None;
    let mut output = None;
    let mut limit = None;
    let mut offset = 0usize;
    let mut min_constrained_coverage_pct = 70u64;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--questions" | "--questions-file" => {
                questions = Some(PathBuf::from(next_value(&mut args, &arg)?));
            }
            "--output" | "--report" => output = Some(PathBuf::from(next_value(&mut args, &arg)?)),
            "--limit" => limit = Some(parse_usize(&next_value(&mut args, &arg)?, &arg)?),
            "--offset" => offset = parse_usize(&next_value(&mut args, &arg)?, &arg)?,
            "--min-constrained-coverage-pct" => {
                min_constrained_coverage_pct =
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
        min_constrained_coverage_pct,
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
    "usage: enterprise_rag_condition_check --questions <questions.jsonl> --output <report.json> [--limit N] [--offset N] [--min-constrained-coverage-pct PCT]".to_owned()
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
        rows.push(QuestionRow {
            question_id: required_str(&value, "question_id", line_index)?,
            question: required_str(&value, "question", line_index)?,
            question_type: value
                .get("question_type")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_owned(),
        });
    }
    if rows.is_empty() {
        return Err("questions file is empty".to_owned());
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

fn build_report(rows: &[QuestionRow], args: &Args) -> Result<Value, String> {
    if rows.is_empty() {
        return Err("no questions selected".to_owned());
    }
    let mut constrained_questions = 0u64;
    let mut structured_constrained_questions = 0u64;
    let mut by_type = BTreeMap::<String, TypeStats>::new();
    let mut details = Vec::with_capacity(rows.len());
    for row in rows {
        let extraction = extract_query_conditions(&row.question);
        let structured = extraction.has_structured_conditions();
        if row.question_type == "constrained" {
            constrained_questions += 1;
            if structured {
                structured_constrained_questions += 1;
            }
        }
        by_type
            .entry(row.question_type.clone())
            .or_default()
            .record(structured);
        details.push(json!({
            "question_id": row.question_id,
            "question_type": row.question_type,
            "question": row.question,
            "structured": structured,
            "numeric_conditions": extraction.numeric_conditions.iter().map(|condition| {
                json!({
                    "id": condition.id,
                    "operator": condition.operator.as_str(),
                    "raw_text": condition.raw_text,
                    "metric_terms": condition.metric_terms,
                    "values": condition.values.iter().map(|value| {
                        json!({
                            "raw": value.raw,
                            "scaled_value": value.scaled_value,
                            "currency": value.currency,
                            "unit": value.unit,
                        })
                    }).collect::<Vec<_>>(),
                })
            }).collect::<Vec<_>>(),
            "condition_slots": extraction.condition_slots.iter().map(|slot| {
                json!({
                    "id": slot.id,
                    "operator_hint": slot.operator_hint.map(|operator| operator.as_str()),
                    "raw_text": slot.raw_text,
                    "metric_terms": slot.metric_terms,
                })
            }).collect::<Vec<_>>(),
            "temporal_range": extraction.temporal_range.map(|range| {
                json!({
                    "start": range.start.as_iso_date(),
                    "end": range.end.as_iso_date(),
                })
            }),
        }));
    }
    let constrained_coverage_pct = structured_constrained_questions
        .saturating_mul(100)
        .checked_div(constrained_questions)
        .unwrap_or(0);
    let mut errors = Vec::new();
    if constrained_questions == 0 {
        errors.push("selected questions contain no constrained rows".to_owned());
    }
    if constrained_coverage_pct < args.min_constrained_coverage_pct {
        errors.push(format!(
            "constrained_coverage_pct {constrained_coverage_pct} < {}",
            args.min_constrained_coverage_pct
        ));
    }
    Ok(json!({
        "schema_version": "cortexdb.enterprise_rag_condition_check.v1",
        "questions": rows.len(),
        "constrained_questions": constrained_questions,
        "structured_constrained_questions": structured_constrained_questions,
        "constrained_coverage_pct": constrained_coverage_pct,
        "min_constrained_coverage_pct": args.min_constrained_coverage_pct,
        "status": if errors.is_empty() { "passed" } else { "failed" },
        "errors": errors,
        "by_question_type": by_type
            .into_iter()
            .map(|(question_type, stats)| {
                let coverage = stats
                    .structured
                    .saturating_mul(100)
                    .checked_div(stats.total)
                    .unwrap_or(0);
                (question_type, json!({
                    "total": stats.total,
                    "structured": stats.structured,
                    "coverage_pct": coverage,
                }))
            })
            .collect::<BTreeMap<_, _>>(),
        "details": details,
    }))
}

impl TypeStats {
    fn record(&mut self, structured: bool) {
        self.total += 1;
        if structured {
            self.structured += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{build_report, parse_args, Args, QuestionRow};

    #[test]
    fn parse_args_accepts_threshold_and_limit() {
        let args = parse_args(
            [
                "--questions",
                "questions.jsonl",
                "--output",
                "report.json",
                "--limit",
                "50",
                "--min-constrained-coverage-pct",
                "70",
            ]
            .into_iter()
            .map(str::to_owned),
        )
        .unwrap();

        assert_eq!(args.limit, Some(50));
        assert_eq!(args.min_constrained_coverage_pct, 70);
    }

    #[test]
    fn report_passes_when_constrained_questions_have_conditions() {
        let args = Args {
            questions: "questions.jsonl".into(),
            output: "report.json".into(),
            limit: None,
            offset: 0,
            min_constrained_coverage_pct: 70,
        };
        let rows = vec![
            QuestionRow {
                question_id: "q1".to_owned(),
                question: "What threshold must be under 200 ms?".to_owned(),
                question_type: "constrained".to_owned(),
            },
            QuestionRow {
                question_id: "q2".to_owned(),
                question: "What changed after 2026-04-01?".to_owned(),
                question_type: "constrained".to_owned(),
            },
        ];

        let report = build_report(&rows, &args).unwrap();

        assert_eq!(report["status"], json!("passed"));
        assert_eq!(report["constrained_coverage_pct"], json!(100));
    }
}
