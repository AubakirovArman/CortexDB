use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process::ExitCode;

use args::Args;
use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::CellId;
use cortex_engine::{scope_id, Database, SearchLimit};
use document::{build_payload, extract_document_content, payload_field};
use io::{read_json, read_jsonl, read_uuid_index, write_json, write_jsonl};
use serde_json::{json, Value};

mod args;
mod document;
mod io;

const SCOPE: &str = "bench:enterprise_rag";

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
        ingest_documents(&mut db, &uuid_index, &args)?;
        db.checkpoint()
            .map_err(|error| format!("failed to checkpoint benchmark corpus: {error}"))?;
    }

    let rows = retrieve_questions(&db, &questions, &args)?;
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
) -> Result<(), String> {
    let mut indexed = 0usize;
    for (index, (doc_id, rel_path)) in uuid_index.iter().enumerate() {
        if args.max_documents.is_some_and(|max| indexed >= max) {
            break;
        }
        let document = read_json(&args.sources_dir.join(rel_path))?;
        let (title, content) = extract_document_content(&document);
        let payload = build_payload(doc_id, rel_path, &title, &content);
        let cell_id = CellId(u64::try_from(index + 1).map_err(|_| "cell id overflow")?);
        db.put_cell(cell_id, payload.into_bytes())
            .map_err(|error| format!("failed to put {doc_id}: {error}"))?;
        indexed += 1;
        if args.progress_every > 0 && indexed.is_multiple_of(args.progress_every) {
            eprintln!("indexed {indexed}/{}", uuid_index.len());
        }
    }
    Ok(())
}

fn retrieve_questions(
    db: &Database,
    questions: &[Value],
    args: &Args,
) -> Result<Vec<Value>, String> {
    let view = view_for_scope(SCOPE);
    let mut rows = Vec::with_capacity(questions.len());
    for (index, question) in questions.iter().enumerate() {
        let qid = required_str(question, "question_id", index)?;
        let query = required_str(question, "question", index)?;
        let results = db
            .search_keyword(query, &view, SearchLimit(args.top_k))
            .map_err(|error| format!("failed to search {qid}: {error}"))?;
        let mut seen = BTreeSet::<String>::new();
        let mut doc_ids = Vec::<Value>::new();
        for result in results {
            let payload = String::from_utf8_lossy(&result.payload);
            if let Some(doc_id) = payload_field(&payload, "doc_id") {
                if seen.insert(doc_id.clone()) {
                    doc_ids.push(Value::String(doc_id));
                }
            }
        }
        rows.push(json!({
            "question_id": qid,
            "question": query,
            "question_type": question.get("question_type").and_then(Value::as_str).unwrap_or("unknown"),
            "answer": "",
            "document_ids": doc_ids,
        }));
        if args.progress_every > 0 && (index + 1).is_multiple_of(args.progress_every) {
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
        "by_question_type": by_type,
        "output": args.output,
        "db_root": args.db_root,
        "runner": "cortex-engine-keyword-retrieval",
    })
}

fn required_str<'a>(row: &'a Value, field: &str, index: usize) -> Result<&'a str, String> {
    row.get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("row {} missing non-empty {field}", index + 1))
}

fn view_for_scope(scope: &str) -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: Some("enterprise-rag-bench-runner".to_owned()),
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope_id(scope)]),
        writable_scopes: BTreeSet::new(),
        allowed_modes: BTreeSet::from([RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::from([MemoryType::Decision]),
        max_context_budget_tokens: 8_000,
        default_context_budget_tokens: 2_000,
        max_candidate_limit: 100,
        default_candidate_limit: 20,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: Some(3_600),
        allow_remember: false,
        allow_verify_fact: false,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: None,
    }
}
