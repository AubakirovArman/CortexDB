use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process::ExitCode;

use cortex_engine::{SearchIndexes, SearchMode, SearchQuery};
use serde::Deserialize;
use serde_json::json;

#[path = "query_understanding_lift_check/args.rs"]
mod args;
#[path = "query_understanding_lift_check/baseline.rs"]
mod baseline;
#[path = "query_understanding_lift_check/io.rs"]
mod io;

use args::Args;
use baseline::{plain_search, PlainDoc};
use io::read_jsonl;

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
    let docs = read_jsonl::<DocumentRow>(&args.documents)?;
    let questions = read_jsonl::<QuestionRow>(&args.questions)?;
    let report = evaluate(&docs, &questions, &args)?;
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
        Err("query understanding lift check failed".to_owned())
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DocumentRow {
    doc_id: String,
    title: String,
    path: String,
    body: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct QuestionRow {
    question_id: String,
    question: String,
    expected_doc_ids: Vec<String>,
}

fn evaluate(
    docs: &[DocumentRow],
    questions: &[QuestionRow],
    args: &Args,
) -> Result<serde_json::Value, String> {
    if docs.is_empty() || questions.is_empty() {
        return Err("documents and questions must be non-empty".to_owned());
    }
    let mut index = SearchIndexes::default();
    let mut doc_to_candidate = BTreeMap::<String, u32>::new();
    let mut candidate_to_doc = BTreeMap::<u32, String>::new();
    let mut plain_docs = Vec::with_capacity(docs.len());
    for (offset, doc) in docs.iter().enumerate() {
        if doc.doc_id.is_empty() {
            return Err("document doc_id must be non-empty".to_owned());
        }
        let candidate = u32::try_from(offset + 1).map_err(|_| "too many documents".to_owned())?;
        if doc_to_candidate
            .insert(doc.doc_id.clone(), candidate)
            .is_some()
        {
            return Err(format!("duplicate doc_id {}", doc.doc_id));
        }
        candidate_to_doc.insert(candidate, doc.doc_id.clone());
        index.add_document_fields(
            candidate,
            &[(&doc.title, 4), (&doc.path, 3), (&doc.body, 1)],
        );
        plain_docs.push(PlainDoc::new(
            doc.doc_id.clone(),
            &format!("{} {} {}", doc.title, doc.path, doc.body),
        ));
    }

    let mut details = Vec::with_capacity(questions.len());
    let mut baseline_recalls = Vec::with_capacity(questions.len());
    let mut engine_recalls = Vec::with_capacity(questions.len());
    let mut baseline_full = 0i64;
    let mut engine_full = 0i64;
    for question in questions {
        if question.question_id.is_empty() || question.question.is_empty() {
            return Err("question_id and question must be non-empty".to_owned());
        }
        for doc_id in &question.expected_doc_ids {
            if !doc_to_candidate.contains_key(doc_id) {
                return Err(format!(
                    "question {} references missing doc_id {}",
                    question.question_id, doc_id
                ));
            }
        }
        let expected = question
            .expected_doc_ids
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        let baseline_docs = plain_search(&plain_docs, &question.question, args.top_k);
        let engine_docs = index
            .search(SearchQuery {
                text: &question.question,
                vector: None,
                limit: args.top_k,
                mode: SearchMode::Keyword,
            })
            .into_iter()
            .filter_map(|result| candidate_to_doc.get(&result.cell_id).cloned())
            .collect::<Vec<_>>();
        let baseline_recall = recall_pct(&baseline_docs, &expected);
        let engine_recall = recall_pct(&engine_docs, &expected);
        if baseline_recall == 100 {
            baseline_full += 1;
        }
        if engine_recall == 100 {
            engine_full += 1;
        }
        baseline_recalls.push(baseline_recall);
        engine_recalls.push(engine_recall);
        details.push(json!({
            "question_id": question.question_id,
            "baseline_doc_ids": baseline_docs,
            "engine_doc_ids": engine_docs,
            "expected_doc_ids": question.expected_doc_ids,
            "baseline_recall_pct": baseline_recall,
            "engine_recall_pct": engine_recall
        }));
    }
    let baseline_average = mean_u64(&baseline_recalls);
    let engine_average = mean_u64(&engine_recalls);
    let average_delta = engine_average as i64 - baseline_average as i64;
    let full_delta = engine_full - baseline_full;
    let mut errors = Vec::new();
    if average_delta < args.min_average_recall_delta_pct {
        errors.push(format!(
            "average_recall_delta_pct {average_delta} < {}",
            args.min_average_recall_delta_pct
        ));
    }
    if full_delta < args.min_full_recall_delta {
        errors.push(format!(
            "full_recall_delta {full_delta} < {}",
            args.min_full_recall_delta
        ));
    }
    if engine_average < args.min_engine_average_recall_pct {
        errors.push(format!(
            "engine_average_recall_pct {engine_average} < {}",
            args.min_engine_average_recall_pct
        ));
    }
    Ok(json!({
        "schema_version": "cortexdb.query_understanding_lift.v1",
        "status": if errors.is_empty() { "passed" } else { "failed" },
        "oracle_fields_allowed": false,
        "documents": docs.len(),
        "questions": questions.len(),
        "top_k": args.top_k,
        "metrics": {
            "baseline_average_recall_pct": baseline_average,
            "engine_average_recall_pct": engine_average,
            "average_recall_delta_pct": average_delta,
            "baseline_full_recall_questions": baseline_full,
            "engine_full_recall_questions": engine_full,
            "full_recall_delta": full_delta
        },
        "thresholds": {
            "min_average_recall_delta_pct": args.min_average_recall_delta_pct,
            "min_full_recall_delta": args.min_full_recall_delta,
            "min_engine_average_recall_pct": args.min_engine_average_recall_pct
        },
        "details": details,
        "errors": errors
    }))
}

fn recall_pct(retrieved: &[String], expected: &BTreeSet<String>) -> u64 {
    if expected.is_empty() {
        return 100;
    }
    let hits = retrieved
        .iter()
        .filter(|doc_id| expected.contains(*doc_id))
        .count();
    (hits as u64 * 100) / expected.len() as u64
}

fn mean_u64(values: &[u64]) -> u64 {
    if values.is_empty() {
        return 0;
    }
    values.iter().sum::<u64>() / values.len() as u64
}
