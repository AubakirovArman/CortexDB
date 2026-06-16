use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use args::Args;
use cortex_core::CellId;
use cortex_engine::{Database, DatabaseSearchResult, SearchLimit};
use model::{TurnIndex, TurnMeta};
use serde_json::{json, Value};
use view::{scope_for_sample, view_for_sample};

mod args;
mod model;
mod view;

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
    if args.reset_db && args.db_root.exists() {
        fs::remove_dir_all(&args.db_root)
            .map_err(|error| format!("failed to reset {}: {error}", args.db_root.display()))?;
    }
    let samples = read_json_list(&args.data_file)?;
    let mut db = Database::open(&args.db_root)
        .map_err(|error| format!("failed to open database: {error}"))?;
    let index = ingest_samples(&mut db, &samples)?;
    db.checkpoint()
        .map_err(|error| format!("failed to checkpoint LoCoMo turns: {error}"))?;
    let rows = retrieve_questions(&db, &samples, &index, &args)?;
    write_jsonl(&args.output, &rows)?;
    if let Some(report) = &args.report {
        write_json(report, &report_payload(&samples, &index, &args))?;
    }
    Ok(())
}

fn ingest_samples(db: &mut Database, samples: &[Value]) -> Result<TurnIndex, String> {
    let mut next_cell = 1u64;
    let mut index = TurnIndex::default();
    for (sample_index, sample) in samples.iter().enumerate() {
        let sample_id = required_str(sample, "sample_id", sample_index)?;
        let conversation = object_field(sample, "conversation", sample_index)?;
        for session in sessions(conversation) {
            let date_key = format!("{session}_date_time");
            let date = conversation
                .get(date_key.as_str())
                .and_then(Value::as_str)
                .unwrap_or("");
            let turns = conversation
                .get(&session)
                .and_then(Value::as_array)
                .ok_or_else(|| format!("{sample_id}:{session}: expected turn list"))?;
            for turn in turns {
                let dia_id = required_str_value(turn, "dia_id")?;
                let meta = TurnMeta {
                    sample_id: sample_id.to_owned(),
                    session: session.clone(),
                    date: date.to_owned(),
                    dia_id: dia_id.to_owned(),
                    speaker: required_str_value(turn, "speaker")?.to_owned(),
                    text: required_str_value(turn, "text")?.to_owned(),
                };
                let cell_id = CellId(next_cell);
                next_cell = next_cell.checked_add(1).ok_or("cell id overflow")?;
                db.put_cell(cell_id, build_payload(&meta).into_bytes())
                    .map_err(|error| format!("failed to put {sample_id}:{dia_id}: {error}"))?;
                index.by_cell.insert(cell_id, meta);
            }
        }
    }
    Ok(index)
}

fn retrieve_questions(
    db: &Database,
    samples: &[Value],
    index: &TurnIndex,
    args: &Args,
) -> Result<Vec<Value>, String> {
    let mut rows = Vec::new();
    let mut seen = 0usize;
    for (sample_index, sample) in samples.iter().enumerate() {
        let sample_id = required_str(sample, "sample_id", sample_index)?;
        let qa_rows = sample
            .get("qa")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("{sample_id}: qa must be a list"))?;
        let view = view_for_sample(sample_id);
        for (qa_index, qa) in qa_rows.iter().enumerate() {
            if args.max_questions.is_some_and(|limit| seen >= limit) {
                return Ok(rows);
            }
            let question = required_str_value(qa, "question")?;
            let results = db
                .search_keyword(question, &view, SearchLimit(args.top_k))
                .map_err(|error| format!("failed to search {sample_id}:{qa_index}: {error}"))?;
            rows.push(json!({
                "question_id": format!("{sample_id}:q{}", qa_index + 1),
                "sample_id": sample_id,
                "question": question,
                "answer": qa.get("answer").cloned().unwrap_or(Value::Null),
                "category": qa.get("category").cloned().unwrap_or(Value::Null),
                "evidence_dialog_ids": evidence_ids(qa),
                "retrieval_list": retrieval_list(results, index),
            }));
            seen += 1;
            if args.progress_every > 0 && seen.is_multiple_of(args.progress_every) {
                eprintln!("retrieved {seen} LoCoMo questions");
            }
        }
    }
    Ok(rows)
}

fn retrieval_list(results: Vec<DatabaseSearchResult>, index: &TurnIndex) -> Vec<Value> {
    results
        .into_iter()
        .enumerate()
        .filter_map(|(rank, result)| {
            index.by_cell.get(&result.cell_id).map(|meta| {
                json!({
                    "rank": rank + 1,
                    "cell_id": result.cell_id.0,
                    "score": result.score,
                    "sample_id": meta.sample_id,
                    "session": meta.session,
                    "date": meta.date,
                    "dia_id": meta.dia_id,
                    "speaker": meta.speaker,
                    "text": meta.text,
                })
            })
        })
        .collect()
}

fn report_payload(samples: &[Value], index: &TurnIndex, args: &Args) -> Value {
    let mut by_category = BTreeMap::<String, usize>::new();
    let mut qa_count = 0usize;
    let mut qa_with_evidence = 0usize;
    for sample in samples {
        for qa in sample
            .get("qa")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            qa_count += 1;
            if !evidence_ids(qa).is_empty() {
                qa_with_evidence += 1;
            }
            let category = qa
                .get("category")
                .map(Value::to_string)
                .unwrap_or_else(|| "unknown".to_owned());
            *by_category.entry(category).or_default() += 1;
        }
    }
    json!({
        "schema_version": "cortexdb.locomo.retrieval_report.v1",
        "samples": samples.len(),
        "questions": args.max_questions.unwrap_or(qa_count).min(qa_count),
        "qa_with_evidence": qa_with_evidence,
        "turns_indexed": index.by_cell.len(),
        "top_k": args.top_k,
        "by_category": by_category,
        "output": args.output,
        "db_root": args.db_root,
        "runner": "cortex-engine-locomo-keyword-retrieval",
        "optional_e2e": "not_run",
    })
}

fn build_payload(meta: &TurnMeta) -> String {
    [
        format!("scope={}", scope_for_sample(&meta.sample_id)),
        "status=ready".to_owned(),
        "type=document_block".to_owned(),
        "source=locomo".to_owned(),
        format!("sample_id={}", meta.sample_id),
        format!("session={}", meta.session),
        format!("date={}", meta.date),
        format!("dia_id={}", meta.dia_id),
        format!("speaker={}", meta.speaker),
        String::new(),
        format!(
            "{} {} {}: {}",
            meta.date, meta.session, meta.speaker, meta.text
        ),
    ]
    .join("\n")
}

fn sessions(conversation: &serde_json::Map<String, Value>) -> Vec<String> {
    let mut keys = conversation
        .iter()
        .filter(|(key, value)| {
            key.starts_with("session_") && !key.ends_with("_date_time") && value.is_array()
        })
        .map(|(key, _)| key.to_owned())
        .collect::<Vec<_>>();
    keys.sort_by_key(|key| {
        key.split_once('_')
            .and_then(|(_, n)| n.parse::<u32>().ok())
            .unwrap_or(0)
    });
    keys
}

fn evidence_ids(qa: &Value) -> Vec<String> {
    qa.get("evidence")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

fn required_str<'a>(row: &'a Value, field: &str, index: usize) -> Result<&'a str, String> {
    row.get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("row {} missing non-empty {field}", index + 1))
}

fn required_str_value<'a>(row: &'a Value, field: &str) -> Result<&'a str, String> {
    row.get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("missing non-empty {field}"))
}

fn object_field<'a>(
    row: &'a Value,
    field: &str,
    index: usize,
) -> Result<&'a serde_json::Map<String, Value>, String> {
    row.get(field)
        .and_then(Value::as_object)
        .ok_or_else(|| format!("row {} missing object {field}", index + 1))
}

fn read_json_list(path: &PathBuf) -> Result<Vec<Value>, String> {
    match load_json(path)? {
        Value::Array(rows) => Ok(rows),
        _ => Err(format!("{}: expected JSON array", path.display())),
    }
}

fn load_json(path: &PathBuf) -> Result<Value, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    serde_json::from_str(&text).map_err(|error| format!("invalid JSON {}: {error}", path.display()))
}

fn write_json(path: &PathBuf, value: &Value) -> Result<(), String> {
    write_text(
        path,
        &(serde_json::to_string_pretty(value).map_err(|error| error.to_string())? + "\n"),
    )
}

fn write_jsonl(path: &PathBuf, rows: &[Value]) -> Result<(), String> {
    let mut output = String::new();
    for row in rows {
        output.push_str(&serde_json::to_string(row).map_err(|error| error.to_string())?);
        output.push('\n');
    }
    write_text(path, &output)
}

fn write_text(path: &PathBuf, text: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    fs::write(path, text).map_err(|error| format!("failed to write {}: {error}", path.display()))
}
