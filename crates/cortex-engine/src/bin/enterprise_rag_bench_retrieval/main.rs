use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process::ExitCode;

use args::Args;
use cortex_core::CellId;
use cortex_engine::Database;
use document::{build_payload, extract_document_content};
use io::{read_json, read_jsonl, read_uuid_index, write_json, write_jsonl};
use retrieval::BenchmarkRetrievalIndex;
use serde_json::{json, Value};

mod args;
mod document;
mod io;
mod retrieval;

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

    let questions = read_jsonl(&args.questions)?;
    let uuid_index = read_uuid_index(&args.uuid_index)?;
    let mut db =
        Database::open(&args.db_root).map_err(|error| format!("failed to open db: {error}"))?;

    if !args.skip_ingest {
        let indexed = ingest_documents(&mut db, &uuid_index, &args)?;
        db.checkpoint()
            .map_err(|error| format!("failed to checkpoint benchmark corpus: {error}"))?;
        eprintln!("checkpointed {indexed} EnterpriseRAG-Bench documents");
    }

    let retrieval_index = BenchmarkRetrievalIndex::load(&db, &uuid_index)?;
    let rows = retrieve_questions(&retrieval_index, &questions, &args)?;
    write_jsonl(&args.output, &rows)?;
    if let Some(report) = &args.report {
        write_json(report, &report_payload(&questions, &uuid_index, &args))?;
    }
    Ok(())
}

fn ingest_documents(
    db: &mut Database,
    uuid_index: &BTreeMap<String, String>,
    args: &Args,
) -> Result<usize, String> {
    let mut indexed = 0usize;
    let mut batch = Vec::with_capacity(args.batch_size);
    for (index, (doc_id, rel_path)) in uuid_index.iter().enumerate() {
        if args.max_documents.is_some_and(|max| indexed >= max) {
            break;
        }
        let document = read_json(&args.sources_dir.join(rel_path))?;
        let (title, content) = extract_document_content(&document);
        let payload = build_payload(doc_id, rel_path, &title, &content);
        let cell_id = CellId(u64::try_from(index + 1).map_err(|_| "cell id overflow")?);
        batch.push((cell_id, payload.into_bytes()));
        if batch.len() >= args.batch_size {
            flush_batch(db, &mut batch, doc_id)?;
        }
        indexed += 1;
        if args.progress_every > 0 && indexed.is_multiple_of(args.progress_every) {
            eprintln!("indexed {indexed}/{}", uuid_index.len());
        }
    }
    flush_batch(db, &mut batch, "final batch")?;
    Ok(indexed)
}

fn flush_batch(
    db: &mut Database,
    batch: &mut Vec<(CellId, Vec<u8>)>,
    label: &str,
) -> Result<(), String> {
    if batch.is_empty() {
        return Ok(());
    }
    let cells = std::mem::take(batch);
    db.put_cells(cells)
        .map_err(|error| format!("failed to put batch near {label}: {error}"))?;
    Ok(())
}

fn retrieve_questions(
    retrieval_index: &BenchmarkRetrievalIndex,
    questions: &[Value],
    args: &Args,
) -> Result<Vec<Value>, String> {
    let mut rows = Vec::with_capacity(questions.len());
    for (index, question) in questions.iter().enumerate() {
        let qid = required_str(question, "question_id", index)?;
        let query = required_str(question, "question", index)?;
        let source_types = source_types(question);
        let mut seen = BTreeSet::<String>::new();
        let mut doc_ids = Vec::<Value>::new();
        for doc_id in retrieval_index.search_doc_ids(query, &source_types, args.top_k) {
            if seen.insert(doc_id.clone()) {
                doc_ids.push(Value::String(doc_id));
            }
        }
        rows.push(json!({
            "question_id": qid,
            "question": query,
            "question_type": question.get("question_type").and_then(Value::as_str).unwrap_or("unknown"),
            "answer": "",
            "document_ids": doc_ids,
        }));
        if args.progress_every > 0
            && ((index + 1).is_multiple_of(args.progress_every)
                || questions.len() <= args.progress_every)
        {
            eprintln!("retrieved {}/{}", index + 1, questions.len());
        }
    }
    Ok(rows)
}

fn report_payload(
    questions: &[Value],
    uuid_index: &BTreeMap<String, String>,
    args: &Args,
) -> Value {
    let mut by_type = BTreeMap::<String, usize>::new();
    for question in questions {
        let question_type = question
            .get("question_type")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_owned();
        *by_type.entry(question_type).or_default() += 1;
    }
    json!({
        "schema_version": "cortexdb.enterprise_rag_bench.retrieval_report.v1",
        "questions": questions.len(),
        "documents_indexed": args.max_documents.unwrap_or(uuid_index.len()),
        "top_k": args.top_k,
        "batch_size": args.batch_size,
        "source_type_filter": true,
        "by_question_type": by_type,
        "output": args.output,
        "db_root": args.db_root,
        "runner": "cortex-engine-keyword-source-aware-retrieval",
    })
}

fn required_str<'a>(row: &'a Value, field: &str, index: usize) -> Result<&'a str, String> {
    row.get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("row {} missing non-empty {field}", index + 1))
}

fn source_types(row: &Value) -> Vec<String> {
    row.get("source_types")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}
