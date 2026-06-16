use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::CellId;
use cortex_engine::{scope_id, Database, SearchLimit};
use serde_json::{json, Value};

const SCOPE: &str = "bench:multihop_rag";

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
    let corpus = read_json_list(&args.corpus)?;
    let queries = read_json_list(&args.queries)?;
    let mut db = Database::open(&args.db_root)
        .map_err(|error| format!("failed to open database: {error}"))?;
    ingest_corpus(&mut db, &corpus)?;
    db.checkpoint()
        .map_err(|error| format!("failed to checkpoint indexed corpus: {error}"))?;

    let view = view_for_scope(SCOPE);
    let mut output_rows = Vec::with_capacity(queries.len());
    let mut by_type = BTreeMap::<String, usize>::new();
    for (index, row) in queries.into_iter().enumerate() {
        let query = required_str(&row, "query", index)?;
        let question_type = row
            .get("question_type")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_owned();
        *by_type.entry(question_type).or_default() += 1;
        let results = db
            .search_keyword(query, &view, SearchLimit(args.top_k))
            .map_err(|error| format!("failed to search query {}: {error}", index + 1))?;
        let mut object = row
            .as_object()
            .cloned()
            .ok_or_else(|| format!("query row {} must be an object", index + 1))?;
        object.insert("gold_list".to_owned(), gold_list(&row));
        object.insert(
            "retrieval_list".to_owned(),
            Value::Array(
                results
                    .into_iter()
                    .enumerate()
                    .map(|(rank, result)| {
                        json!({
                            "rank": rank + 1,
                            "cell_id": result.cell_id.0,
                            "score": result.score,
                            "text": String::from_utf8_lossy(&result.payload),
                        })
                    })
                    .collect(),
            ),
        );
        output_rows.push(Value::Object(object));
        if args.progress_every > 0 && (index + 1) % args.progress_every == 0 {
            eprintln!("retrieved {}/{}", index + 1, output_rows.capacity());
        }
    }

    write_json(&args.output, &Value::Array(output_rows))?;
    if let Some(report) = args.report {
        write_json(
            &report,
            &json!({
                "schema_version": "cortexdb.multihop_rag.retrieval_report.v2",
                "queries": by_type.values().sum::<usize>(),
                "corpus_rows": corpus.len(),
                "top_k": args.top_k,
                "by_question_type": by_type,
                "output": args.output,
                "db_root": args.db_root,
                "runner": "cortex-engine-single-process",
            }),
        )?;
    }
    Ok(())
}

fn ingest_corpus(db: &mut Database, corpus: &[Value]) -> Result<(), String> {
    for (index, row) in corpus.iter().enumerate() {
        let body = required_str(row, "body", index)?;
        let payload = build_payload(row, body);
        let cell_id = CellId(u64::try_from(index + 1).map_err(|_| "cell id overflow")?);
        db.put_cell(cell_id, payload.into_bytes())
            .map_err(|error| format!("failed to put corpus row {}: {error}", index + 1))?;
    }
    Ok(())
}

fn build_payload(row: &Value, body: &str) -> String {
    [
        "scope=bench:multihop_rag".to_owned(),
        "status=ready".to_owned(),
        "type=document_block".to_owned(),
        format!("title={}", field(row, "title")),
        format!("source={}", field(row, "source")),
        format!("published_at={}", field(row, "published_at")),
        format!("url={}", field(row, "url")),
        String::new(),
        body.to_owned(),
    ]
    .join("\n")
}

fn field(row: &Value, name: &str) -> String {
    row.get(name)
        .and_then(Value::as_str)
        .unwrap_or("")
        .replace(['\r', '\n'], " ")
        .trim()
        .to_owned()
}

fn gold_list(row: &Value) -> Value {
    row.get("evidence_list")
        .and_then(Value::as_array)
        .map(|items| Value::Array(items.clone()))
        .unwrap_or_else(|| Value::Array(Vec::new()))
}

fn required_str<'a>(row: &'a Value, field: &str, index: usize) -> Result<&'a str, String> {
    row.get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("row {} missing non-empty {field}", index + 1))
}

fn read_json_list(path: &PathBuf) -> Result<Vec<Value>, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let value: Value = serde_json::from_str(&text)
        .map_err(|error| format!("invalid JSON {}: {error}", path.display()))?;
    match value {
        Value::Array(rows) => Ok(rows),
        _ => Err(format!("{}: expected JSON array", path.display())),
    }
}

fn write_json(path: &PathBuf, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }
    fs::write(
        path,
        serde_json::to_string_pretty(value).map_err(|error| error.to_string())? + "\n",
    )
    .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn view_for_scope(scope: &str) -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: Some("multihop-rag-runner".to_owned()),
        readable_brains: std::collections::BTreeSet::from([BrainId(1)]),
        readable_scopes: std::collections::BTreeSet::from([scope_id(scope)]),
        writable_scopes: std::collections::BTreeSet::new(),
        allowed_modes: std::collections::BTreeSet::from([RetrievalMode::Balanced]),
        allowed_memory_types: std::collections::BTreeSet::from([MemoryType::Decision]),
        max_context_budget_tokens: 4_000,
        default_context_budget_tokens: 1_000,
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

#[derive(Debug, PartialEq, Eq)]
struct Args {
    queries: PathBuf,
    corpus: PathBuf,
    db_root: PathBuf,
    output: PathBuf,
    report: Option<PathBuf>,
    top_k: usize,
    progress_every: usize,
    reset_db: bool,
}

impl Args {
    fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut queries = None;
        let mut corpus = None;
        let mut db_root = None;
        let mut output = None;
        let mut report = None;
        let mut top_k = 10usize;
        let mut progress_every = 250usize;
        let mut reset_db = false;
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--queries" => queries = Some(PathBuf::from(next_value(&mut args, &arg)?)),
                "--corpus" => corpus = Some(PathBuf::from(next_value(&mut args, &arg)?)),
                "--db-root" => db_root = Some(PathBuf::from(next_value(&mut args, &arg)?)),
                "--output" => output = Some(PathBuf::from(next_value(&mut args, &arg)?)),
                "--report" => report = Some(PathBuf::from(next_value(&mut args, &arg)?)),
                "--top-k" => top_k = parse_usize(&next_value(&mut args, &arg)?, &arg)?,
                "--progress-every" => {
                    progress_every = parse_usize(&next_value(&mut args, &arg)?, &arg)?
                }
                "--reset-db" => reset_db = true,
                "--help" | "-h" => return Err(usage()),
                value => return Err(format!("unknown option: {value}\n{}", usage())),
            }
        }
        Ok(Self {
            queries: queries.ok_or_else(|| "--queries is required".to_owned())?,
            corpus: corpus.ok_or_else(|| "--corpus is required".to_owned())?,
            db_root: db_root.ok_or_else(|| "--db-root is required".to_owned())?,
            output: output.ok_or_else(|| "--output is required".to_owned())?,
            report,
            top_k,
            progress_every,
            reset_db,
        })
    }
}

fn next_value(args: &mut impl Iterator<Item = String>, name: &str) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("{name} requires a value"))
}

fn parse_usize(value: &str, name: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|_| format!("{name} must be an integer"))
}

fn usage() -> String {
    "usage: multihop_rag_retrieval --queries <json> --corpus <json> --db-root <path> --output <json> [--report <json>] [--top-k <n>] [--reset-db]".to_owned()
}
