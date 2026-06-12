use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use cortex_engine::search::{decompose_enterprise_rag_question, QuestionDecomposition};
use serde_json::{json, Value};

#[derive(Clone, Debug, PartialEq, Eq)]
struct Args {
    questions: PathBuf,
    output: PathBuf,
    limit: Option<usize>,
    offset: usize,
    min_multi_coverage_pct: u64,
}

#[derive(Clone, Debug)]
struct QuestionRow {
    question_id: String,
    question: String,
    question_type: String,
    answer_fact_count: usize,
    expected_doc_count: usize,
}

#[derive(Clone, Copy, Debug, Default)]
struct TypeStats {
    total: u64,
    expected_multi: u64,
    decomposed_multi: u64,
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
    let status = report
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("failed");
    if status != "passed" {
        return Err(format!(
            "decomposition check failed; see {}",
            args.output.display()
        ));
    }
    Ok(())
}

fn summary(report: &Value, output: &Path) -> String {
    json!({
        "questions": report.get("questions").and_then(Value::as_u64).unwrap_or(0),
        "expected_multi_questions": report
            .get("expected_multi_questions")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        "decomposed_multi_questions": report
            .get("decomposed_multi_questions")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        "multi_coverage_pct": report
            .get("multi_coverage_pct")
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
    let mut min_multi_coverage_pct = 80u64;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--questions" | "--questions-file" => {
                questions = Some(PathBuf::from(next_value(&mut args, &arg)?));
            }
            "--output" | "--report" => output = Some(PathBuf::from(next_value(&mut args, &arg)?)),
            "--limit" => limit = Some(parse_usize(&next_value(&mut args, &arg)?, &arg)?),
            "--offset" => offset = parse_usize(&next_value(&mut args, &arg)?, &arg)?,
            "--min-multi-coverage-pct" => {
                min_multi_coverage_pct = parse_u64(&next_value(&mut args, &arg)?, &arg)?.min(100)
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
        min_multi_coverage_pct,
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
    "usage: enterprise_rag_decomposition_check --questions <questions.jsonl> --output <report.json> [--limit N] [--offset N] [--min-multi-coverage-pct PCT]".to_owned()
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
        let question_id = required_str(&value, "question_id", line_index)?;
        let question = required_str(&value, "question", line_index)?;
        let question_type = value
            .get("question_type")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_owned();
        let answer_fact_count = value
            .get("answer_facts")
            .and_then(Value::as_array)
            .map_or(0, Vec::len);
        let expected_doc_count = value
            .get("expected_doc_ids")
            .and_then(Value::as_array)
            .map_or(0, Vec::len);
        rows.push(QuestionRow {
            question_id,
            question,
            question_type,
            answer_fact_count,
            expected_doc_count,
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

    use super::{build_report, expected_multi_requirement, parse_args, Args, QuestionRow};

    #[test]
    fn parse_args_accepts_named_paths_and_threshold() {
        let args = parse_args(
            [
                "--questions",
                "questions.jsonl",
                "--output",
                "report.json",
                "--limit",
                "50",
                "--min-multi-coverage-pct",
                "80",
            ]
            .into_iter()
            .map(str::to_owned),
        )
        .unwrap();

        assert_eq!(args.limit, Some(50));
        assert_eq!(args.min_multi_coverage_pct, 80);
    }

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
