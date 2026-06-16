use std::env;
use std::fs;
use std::path::Path;

use serde_json::{json, Value};

#[path = "enterprise_rag_decomposition_check/args.rs"]
mod args;
#[path = "enterprise_rag_decomposition_check/questions.rs"]
mod questions;
#[path = "enterprise_rag_decomposition_check/report.rs"]
mod report;

use args::parse_args;
use questions::read_questions;
use report::build_report;

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
