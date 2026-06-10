use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use cortex_engine::{SearchIndexes, SearchMode, SearchQuery};
use serde::Deserialize;
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

#[derive(Debug, PartialEq, Eq)]
struct Args {
    documents: PathBuf,
    questions: PathBuf,
    output: Option<PathBuf>,
    top_k: usize,
    min_average_recall_delta_pct: i64,
    min_full_recall_delta: i64,
    min_engine_average_recall_pct: u64,
}

impl Args {
    fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut documents = None;
        let mut questions = None;
        let mut output = None;
        let mut top_k = 3usize;
        let mut min_average_recall_delta_pct = 20i64;
        let mut min_full_recall_delta = 2i64;
        let mut min_engine_average_recall_pct = 95u64;
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--documents" => documents = Some(PathBuf::from(next_value(&mut args, &arg)?)),
                "--questions" => questions = Some(PathBuf::from(next_value(&mut args, &arg)?)),
                "--output" => output = Some(PathBuf::from(next_value(&mut args, &arg)?)),
                "--top-k" => top_k = parse_usize(&next_value(&mut args, &arg)?, &arg)?,
                "--min-average-recall-delta-pct" => {
                    min_average_recall_delta_pct = parse_i64(&next_value(&mut args, &arg)?, &arg)?
                }
                "--min-full-recall-delta" => {
                    min_full_recall_delta = parse_i64(&next_value(&mut args, &arg)?, &arg)?
                }
                "--min-engine-average-recall-pct" => {
                    min_engine_average_recall_pct = parse_u64(&next_value(&mut args, &arg)?, &arg)?
                }
                "--help" | "-h" => return Err(usage()),
                value => return Err(format!("unknown option: {value}")),
            }
        }
        Ok(Self {
            documents: documents.ok_or_else(|| "--documents is required".to_owned())?,
            questions: questions.ok_or_else(|| "--questions is required".to_owned())?,
            output,
            top_k,
            min_average_recall_delta_pct,
            min_full_recall_delta,
            min_engine_average_recall_pct,
        })
    }
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
        plain_docs.push(PlainDoc {
            doc_id: doc.doc_id.clone(),
            terms: plain_terms(&format!("{} {} {}", doc.title, doc.path, doc.body)),
        });
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

#[derive(Clone, Debug)]
struct PlainDoc {
    doc_id: String,
    terms: BTreeMap<String, u32>,
}

fn plain_search(docs: &[PlainDoc], query: &str, limit: usize) -> Vec<String> {
    let query_terms = plain_terms(query);
    let mut scored = docs
        .iter()
        .filter_map(|doc| {
            let score = query_terms
                .iter()
                .map(|(term, weight)| {
                    u64::from(*weight) * u64::from(*doc.terms.get(term).unwrap_or(&0))
                })
                .sum::<u64>();
            (score > 0).then(|| (score, doc.doc_id.clone()))
        })
        .collect::<Vec<_>>();
    scored.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    scored
        .into_iter()
        .take(limit)
        .map(|(_, doc_id)| doc_id)
        .collect()
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

fn plain_terms(text: &str) -> BTreeMap<String, u32> {
    let mut terms = BTreeMap::new();
    let mut current = String::new();
    for ch in text.chars() {
        if ch.is_alphanumeric() {
            current.extend(ch.to_lowercase());
        } else if !current.is_empty() {
            if !is_stopword(&current) {
                *terms.entry(std::mem::take(&mut current)).or_default() += 1;
            } else {
                current.clear();
            }
        }
    }
    if !current.is_empty() && !is_stopword(&current) {
        *terms.entry(current).or_default() += 1;
    }
    terms
}

fn is_stopword(term: &str) -> bool {
    matches!(
        term,
        "a" | "an"
            | "and"
            | "are"
            | "as"
            | "at"
            | "be"
            | "by"
            | "for"
            | "from"
            | "how"
            | "is"
            | "it"
            | "of"
            | "on"
            | "or"
            | "should"
            | "the"
            | "to"
            | "we"
            | "what"
            | "when"
            | "which"
            | "who"
            | "why"
    )
}

fn read_jsonl<T: for<'de> Deserialize<'de>>(path: &PathBuf) -> Result<Vec<T>, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let mut values = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        values.push(serde_json::from_str(line).map_err(|error| {
            format!(
                "{} line {} is invalid JSON: {error}",
                path.display(),
                index + 1
            )
        })?);
    }
    Ok(values)
}

fn next_value(args: &mut impl Iterator<Item = String>, option: &str) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("{option} requires a value"))
}

fn parse_usize(value: &str, option: &str) -> Result<usize, String> {
    let parsed = value
        .parse()
        .map_err(|error| format!("{option} must be usize: {error}"))?;
    if parsed == 0 {
        return Err(format!("{option} must be greater than zero"));
    }
    Ok(parsed)
}

fn parse_i64(value: &str, option: &str) -> Result<i64, String> {
    value
        .parse()
        .map_err(|error| format!("{option} must be i64: {error}"))
}

fn parse_u64(value: &str, option: &str) -> Result<u64, String> {
    value
        .parse()
        .map_err(|error| format!("{option} must be u64: {error}"))
}

fn usage() -> String {
    "usage: query_understanding_lift_check --documents PATH --questions PATH \
     [--top-k N] [--output PATH] [--min-average-recall-delta-pct N] \
     [--min-full-recall-delta N] [--min-engine-average-recall-pct N]"
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_search_does_not_expand_terms() {
        let docs = vec![PlainDoc {
            doc_id: "doc_owner".to_owned(),
            terms: plain_terms("assigned DRI dependency"),
        }];

        assert!(plain_search(&docs, "owner blocker", 3).is_empty());
    }

    #[test]
    fn parse_args_rejects_zero_top_k() {
        let error = Args::parse([
            "--documents".to_owned(),
            "docs.jsonl".to_owned(),
            "--questions".to_owned(),
            "questions.jsonl".to_owned(),
            "--top-k".to_owned(),
            "0".to_owned(),
        ])
        .unwrap_err();

        assert!(error.contains("greater than zero"));
    }
}
