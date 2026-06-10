use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use args::{Args, BenchmarkRetrievalMode};
use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::CellId;
use cortex_engine::search::{parse_vector_literal, SearchMode, SearchQuery, WeightedScoreReranker};
use cortex_engine::{scope_id, Database};
use document::{build_payload, extract_document_content};
use io::{read_json, read_jsonl, read_uuid_index, write_json, write_jsonl};
use retrieval::BenchmarkRetrievalIndex;
use serde_json::{json, Value};

mod args;
mod document;
mod io;
mod retrieval;

const ORACLE_FIELDS: &[&str] = &[
    "answer_facts",
    "expected_doc_ids",
    "gold_answer",
    "question_type",
    "source_types",
];

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
    let started = Instant::now();
    let args = Args::parse(env::args().skip(1))?;
    let logger = RunLogger::new(started, args.log_file.clone(), args.status_file.clone())?;
    let mut report_metrics = RunReportMetrics::default();
    logger.log("start enterprise_rag_bench_retrieval");
    logger.status("startup", "running", "parsed arguments", None, None, &[]);
    if args.reset_db && args.db_root.exists() {
        logger.log(&format!("reset db root {}", args.db_root.display()));
        logger.status("startup", "running", "reset database root", None, None, &[]);
        fs::remove_dir_all(&args.db_root)
            .map_err(|error| format!("failed to reset {}: {error}", args.db_root.display()))?;
    }

    logger.log(&format!("load questions {}", args.questions.display()));
    logger.status(
        "load_questions",
        "running",
        "load questions",
        None,
        None,
        &[],
    );
    let questions = read_jsonl(&args.questions)?;
    logger.log(&format!("loaded {} questions", questions.len()));
    logger.status(
        "load_questions",
        "done",
        "questions loaded",
        Some(questions.len()),
        Some(questions.len()),
        &[],
    );
    if args.official_clean {
        reject_oracle_fields(&questions)?;
        logger.log("official-clean question guard passed");
    }
    logger.log(&format!("load uuid index {}", args.uuid_index.display()));
    logger.status(
        "load_corpus_index",
        "running",
        "load uuid index",
        None,
        None,
        &[],
    );
    let uuid_index = read_uuid_index(&args.uuid_index)?;
    logger.log(&format!("loaded {} corpus ids", uuid_index.len()));
    logger.status(
        "load_corpus_index",
        "done",
        "uuid index loaded",
        Some(uuid_index.len()),
        Some(uuid_index.len()),
        &[],
    );
    logger.log(&format!("open database {}", args.db_root.display()));
    logger.status("open_database", "running", "open CortexDB", None, None, &[]);
    let mut db =
        Database::open(&args.db_root).map_err(|error| format!("failed to open db: {error}"))?;
    logger.log("database open");
    logger.status("open_database", "done", "database open", None, None, &[]);

    let document_vectors = load_document_vectors(args.document_vectors.as_ref())?;
    if !document_vectors.is_empty() {
        logger.log(&format!(
            "loaded {} document vectors",
            document_vectors.len()
        ));
    }

    if !args.skip_ingest {
        logger.log("begin corpus ingest");
        logger.status(
            "ingest",
            "running",
            "begin corpus ingest",
            Some(0),
            Some(document_total(&args, uuid_index.len())),
            &[],
        );
        let ingest_started = Instant::now();
        let indexed = ingest_documents(&mut db, &uuid_index, &document_vectors, &args, &logger)?;
        report_metrics.documents_indexed = indexed;
        report_metrics.ingest_duration_ms =
            Some(round_ms(ingest_started.elapsed().as_secs_f64() * 1000.0));
        logger.log(&format!("ingested {indexed} documents"));
        logger.status(
            "ingest",
            "done",
            "corpus ingest done",
            Some(indexed),
            Some(document_total(&args, uuid_index.len())),
            &[],
        );
        logger.log("begin checkpoint");
        logger.status(
            "checkpoint",
            "running",
            "checkpoint benchmark corpus",
            None,
            None,
            &[],
        );
        let checkpoint_started = Instant::now();
        db.checkpoint()
            .map_err(|error| format!("failed to checkpoint benchmark corpus: {error}"))?;
        report_metrics.checkpoint_duration_ms = Some(round_ms(
            checkpoint_started.elapsed().as_secs_f64() * 1000.0,
        ));
        logger.log(&format!("checkpointed {indexed} documents"));
        logger.status("checkpoint", "done", "checkpoint complete", None, None, &[]);
    } else {
        logger.log("skip corpus ingest");
        report_metrics.documents_indexed = 0;
        logger.status("ingest", "skipped", "skip corpus ingest", None, None, &[]);
    }

    let query_vectors = load_query_vectors(args.query_vectors.as_ref())?;
    logger.log("begin question retrieval");
    logger.status(
        "retrieve_questions",
        "running",
        "begin question retrieval",
        Some(0),
        Some(questions.len()),
        &[],
    );
    let retrieval_started = Instant::now();
    let rows = match args.retrieval_mode {
        BenchmarkRetrievalMode::CachedLexical => {
            logger.log("load cached lexical retrieval index from checkpoint segments");
            logger.status(
                "load_retrieval_index",
                "running",
                "load cached lexical retrieval index",
                None,
                None,
                &[],
            );
            let retrieval_index = BenchmarkRetrievalIndex::load(&db, &uuid_index)?;
            logger.log("cached lexical retrieval index loaded");
            logger.status(
                "load_retrieval_index",
                "done",
                "cached lexical retrieval index loaded",
                None,
                None,
                &[],
            );
            retrieve_cached_questions(&retrieval_index, &questions, &args, &logger)?
        }
        BenchmarkRetrievalMode::EngineKeyword | BenchmarkRetrievalMode::EngineHybrid => {
            retrieve_engine_questions(&db, &questions, &query_vectors, &args, &logger)?
        }
    };
    report_metrics.retrieval_duration_ms =
        Some(round_ms(retrieval_started.elapsed().as_secs_f64() * 1000.0));
    report_metrics.questions_retrieved = rows.len();
    logger.log(&format!("retrieved {} question rows", rows.len()));
    logger.status(
        "retrieve_questions",
        "done",
        "question retrieval done",
        Some(rows.len()),
        Some(questions.len()),
        &[],
    );
    logger.log(&format!("write retrieval {}", args.output.display()));
    logger.status(
        "write_outputs",
        "running",
        "write retrieval output",
        None,
        None,
        &[],
    );
    write_jsonl(&args.output, &rows)?;
    if let Some(report) = &args.report {
        logger.log(&format!("write report {}", report.display()));
        report_metrics.total_duration_ms = round_ms(started.elapsed().as_secs_f64() * 1000.0);
        write_json(
            report,
            &report_payload(&questions, &uuid_index, &args, &report_metrics),
        )?;
    }
    logger.log("finished enterprise_rag_bench_retrieval");
    logger.status(
        "write_outputs",
        "done",
        "retrieval output written",
        Some(rows.len()),
        Some(questions.len()),
        &[
            ("output", json!(args.output.display().to_string())),
            (
                "report",
                json!(args.report.as_ref().map(|path| path.display().to_string())),
            ),
        ],
    );
    Ok(())
}

struct RunLogger {
    started: Instant,
    log_file: Option<PathBuf>,
    status_file: Option<PathBuf>,
}

#[derive(Default)]
struct RunReportMetrics {
    total_duration_ms: f64,
    documents_indexed: usize,
    ingest_duration_ms: Option<f64>,
    checkpoint_duration_ms: Option<f64>,
    questions_retrieved: usize,
    retrieval_duration_ms: Option<f64>,
}

impl RunLogger {
    fn new(
        started: Instant,
        log_file: Option<PathBuf>,
        status_file: Option<PathBuf>,
    ) -> Result<Self, String> {
        if let Some(path) = &log_file {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
            }
            fs::write(path, "")
                .map_err(|error| format!("failed to initialize {}: {error}", path.display()))?;
        }
        if let Some(path) = &status_file {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
            }
        }
        Ok(Self {
            started,
            log_file,
            status_file,
        })
    }

    fn log(&self, message: &str) {
        let line = format!(
            "[enterprise-rag-retrieval +{:>6.1}s] {message}",
            self.started.elapsed().as_secs_f64()
        );
        eprintln!("{line}");
        if let Some(path) = &self.log_file {
            if let Ok(mut handle) = OpenOptions::new().create(true).append(true).open(path) {
                let _ = writeln!(handle, "{line}");
            }
        }
    }

    fn status(
        &self,
        stage: &str,
        state: &str,
        message: &str,
        completed: Option<usize>,
        total: Option<usize>,
        extra: &[(&str, Value)],
    ) {
        let Some(path) = &self.status_file else {
            return;
        };
        let elapsed_seconds = self.started.elapsed().as_secs_f64();
        let mut payload = json!({
            "schema_version": "cortexdb.enterprise_rag_bench.retrieval_progress_status.v1",
            "stage": stage,
            "state": state,
            "message": message,
            "elapsed_seconds": (elapsed_seconds * 10.0).round() / 10.0,
            "updated_unix_ms": now_unix_ms(),
            "log_file": self.log_file.as_ref().map(|path| path.display().to_string()),
        });
        if let Some(completed) = completed {
            payload["completed"] = json!(completed);
        }
        if let Some(total) = total {
            payload["total"] = json!(total);
        }
        if let (Some(completed), Some(total)) = (completed, total) {
            let progress_pct = if total > 0 {
                completed as f64 / total as f64 * 100.0
            } else {
                100.0
            };
            payload["progress_pct"] = json!((progress_pct * 100.0).round() / 100.0);
        }
        if let Some(object) = payload.as_object_mut() {
            for (key, value) in extra {
                object.insert((*key).to_owned(), value.clone());
            }
        }
        if let Ok(bytes) = serde_json::to_vec_pretty(&payload) {
            let mut with_newline = bytes;
            with_newline.push(b'\n');
            let _ = fs::write(path, with_newline);
        }
    }
}

fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

fn document_total(args: &Args, corpus_len: usize) -> usize {
    args.max_documents
        .map(|max_documents| max_documents.min(corpus_len))
        .unwrap_or(corpus_len)
}

fn throughput_per_sec(units: usize, duration_ms: Option<f64>) -> f64 {
    let Some(duration_ms) = duration_ms else {
        return 0.0;
    };
    if duration_ms <= 0.0 {
        return 0.0;
    }
    round_ms(units as f64 / (duration_ms / 1000.0))
}

fn round_ms(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

fn ingest_documents(
    db: &mut Database,
    uuid_index: &BTreeMap<String, String>,
    document_vectors: &BTreeMap<String, Vec<i16>>,
    args: &Args,
    logger: &RunLogger,
) -> Result<usize, String> {
    let mut indexed = 0usize;
    let mut batch = Vec::with_capacity(args.batch_size);
    let total = document_total(args, uuid_index.len());
    for (index, (doc_id, rel_path)) in uuid_index.iter().enumerate() {
        if args.max_documents.is_some_and(|max| indexed >= max) {
            break;
        }
        let document = read_json(&args.sources_dir.join(rel_path))?;
        let (title, content) = extract_document_content(&document);
        let payload = build_payload(
            doc_id,
            rel_path,
            &title,
            &content,
            document_vectors.get(doc_id).map(Vec::as_slice),
        );
        let cell_id = CellId(u64::try_from(index + 1).map_err(|_| "cell id overflow")?);
        batch.push((cell_id, payload.into_bytes()));
        if batch.len() >= args.batch_size {
            flush_batch(db, &mut batch, doc_id)?;
        }
        indexed += 1;
        if args.progress_every > 0 && indexed.is_multiple_of(args.progress_every) {
            logger.log(&format!("indexed {indexed}/{total}"));
            logger.status(
                "ingest",
                "running",
                "index documents",
                Some(indexed),
                Some(total),
                &[("last_doc_id", json!(doc_id))],
            );
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

fn retrieve_cached_questions(
    retrieval_index: &BenchmarkRetrievalIndex,
    questions: &[Value],
    args: &Args,
    logger: &RunLogger,
) -> Result<Vec<Value>, String> {
    let mut rows = Vec::with_capacity(questions.len());
    for (index, question) in questions.iter().enumerate() {
        let qid = required_str(question, "question_id", index)?;
        let query = required_str(question, "question", index)?;
        let source_types = if args.official_clean {
            Vec::new()
        } else {
            source_types(question)
        };
        let mut seen = BTreeSet::<String>::new();
        let mut doc_ids = Vec::<Value>::new();
        for doc_id in retrieval_index.search_doc_ids(query, &source_types, args.top_k) {
            if seen.insert(doc_id.clone()) {
                doc_ids.push(Value::String(doc_id));
            }
        }
        let mut row = json!({
            "question_id": qid,
            "question": query,
            "answer": "",
            "document_ids": doc_ids,
        });
        if !args.official_clean {
            row["question_type"] = Value::String(
                question
                    .get("question_type")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_owned(),
            );
        }
        rows.push(row);
        if args.progress_every > 0
            && ((index + 1).is_multiple_of(args.progress_every)
                || questions.len() <= args.progress_every)
        {
            logger.log(&format!("retrieved {}/{}", index + 1, questions.len()));
            logger.status(
                "retrieve_questions",
                "running",
                "retrieve cached question rows",
                Some(index + 1),
                Some(questions.len()),
                &[("last_question_id", json!(qid))],
            );
        }
    }
    Ok(rows)
}

fn retrieve_engine_questions(
    db: &Database,
    questions: &[Value],
    query_vectors: &BTreeMap<String, Vec<i16>>,
    args: &Args,
    logger: &RunLogger,
) -> Result<Vec<Value>, String> {
    let mut rows = Vec::with_capacity(questions.len());
    let view = bench_view();
    let reranker = WeightedScoreReranker::default();
    for (index, question) in questions.iter().enumerate() {
        let qid = required_str(question, "question_id", index)?;
        let query = required_str(question, "question", index)?;
        let (mode, vector) = match args.retrieval_mode {
            BenchmarkRetrievalMode::EngineKeyword => (SearchMode::Keyword, None),
            BenchmarkRetrievalMode::EngineHybrid => {
                let vector = query_vectors.get(qid).ok_or_else(|| {
                    format!("engine-hybrid requires query vector for question_id={qid}")
                })?;
                (SearchMode::Hybrid, Some(vector.as_slice()))
            }
            BenchmarkRetrievalMode::CachedLexical => unreachable!("cached mode handled separately"),
        };
        let search_query = SearchQuery {
            text: query,
            vector,
            limit: args.top_k,
            mode,
        };
        let mode_label = args.retrieval_mode.as_str();
        let results = if args.rerank_mode.is_enabled() {
            db.search_cells_with_reranker(search_query, &view, &reranker)
                .map_err(|error| {
                    format!("engine {mode_label} rerank search failed for {qid}: {error}")
                })?
        } else {
            db.search_cells(search_query, &view)
                .map_err(|error| format!("engine {mode_label} search failed for {qid}: {error}"))?
        };
        let mut seen = BTreeSet::new();
        let document_ids = results
            .into_iter()
            .filter_map(|result| doc_id_from_payload(&result.payload))
            .filter(|doc_id| seen.insert(doc_id.clone()))
            .map(Value::String)
            .collect::<Vec<_>>();
        rows.push(json!({
            "question_id": qid,
            "question": query,
            "answer": "",
            "document_ids": document_ids,
        }));
        if args.progress_every > 0
            && ((index + 1).is_multiple_of(args.progress_every)
                || questions.len() <= args.progress_every)
        {
            logger.log(&format!("retrieved {}/{}", index + 1, questions.len()));
            logger.status(
                "retrieve_questions",
                "running",
                "retrieve engine question rows",
                Some(index + 1),
                Some(questions.len()),
                &[("last_question_id", json!(qid))],
            );
        }
    }
    Ok(rows)
}

fn report_payload(
    questions: &[Value],
    uuid_index: &BTreeMap<String, String>,
    args: &Args,
    metrics: &RunReportMetrics,
) -> Value {
    let mut by_type = BTreeMap::<String, usize>::new();
    if !args.official_clean {
        for question in questions {
            let question_type = question
                .get("question_type")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_owned();
            *by_type.entry(question_type).or_default() += 1;
        }
    }
    json!({
        "schema_version": "cortexdb.enterprise_rag_bench.retrieval_report.v1",
        "questions": questions.len(),
        "documents_indexed": args.max_documents.unwrap_or(uuid_index.len()),
        "top_k": args.top_k,
        "batch_size": args.batch_size,
        "official_clean": args.official_clean,
        "retrieval_mode": args.retrieval_mode.as_str(),
        "rerank_mode": args.rerank_mode.as_str(),
        "query_vectors": args.query_vectors.as_ref().map(|path| path.display().to_string()),
        "document_vectors": args.document_vectors.as_ref().map(|path| path.display().to_string()),
        "source_type_filter": !args.official_clean && args.retrieval_mode == BenchmarkRetrievalMode::CachedLexical,
        "performance": {
            "total_duration_ms": metrics.total_duration_ms,
            "ingest": {
                "documents_indexed": metrics.documents_indexed,
                "duration_ms": metrics.ingest_duration_ms,
                "throughput_docs_per_sec": throughput_per_sec(metrics.documents_indexed, metrics.ingest_duration_ms),
            },
            "checkpoint": {
                "duration_ms": metrics.checkpoint_duration_ms,
            },
            "retrieval": {
                "questions": metrics.questions_retrieved,
                "duration_ms": metrics.retrieval_duration_ms,
                "throughput_questions_per_sec": throughput_per_sec(metrics.questions_retrieved, metrics.retrieval_duration_ms),
            },
            "resource_usage": process_memory_report(),
        },
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

fn reject_oracle_fields(rows: &[Value]) -> Result<(), String> {
    for (index, row) in rows.iter().enumerate() {
        let Some(object) = row.as_object() else {
            continue;
        };
        let forbidden = ORACLE_FIELDS
            .iter()
            .copied()
            .filter(|field| object.contains_key(*field))
            .collect::<Vec<_>>();
        if !forbidden.is_empty() {
            return Err(format!(
                "official-clean question row {} has forbidden oracle fields: {}",
                index + 1,
                forbidden.join(", ")
            ));
        }
    }
    Ok(())
}

fn load_query_vectors(
    path: Option<&std::path::PathBuf>,
) -> Result<BTreeMap<String, Vec<i16>>, String> {
    load_id_vectors(path, "question_id")
}

fn load_document_vectors(
    path: Option<&std::path::PathBuf>,
) -> Result<BTreeMap<String, Vec<i16>>, String> {
    load_id_vectors(path, "doc_id")
}

fn load_id_vectors(
    path: Option<&std::path::PathBuf>,
    id_field: &str,
) -> Result<BTreeMap<String, Vec<i16>>, String> {
    let Some(path) = path else {
        return Ok(BTreeMap::new());
    };
    let mut vectors = BTreeMap::new();
    for (index, row) in read_jsonl(path)?.into_iter().enumerate() {
        let id = required_str(&row, id_field, index)?.to_owned();
        let vector = parse_query_vector(&row)
            .ok_or_else(|| format!("{id_field} vector row {} missing vector", index + 1))?;
        if vector.is_empty() {
            return Err(format!(
                "{} row {} has empty or invalid vector",
                path.display(),
                index + 1
            ));
        }
        vectors.insert(id, vector);
    }
    Ok(vectors)
}

fn parse_query_vector(row: &Value) -> Option<Vec<i16>> {
    let value = row.get("vector")?;
    if let Some(text) = value.as_str() {
        return parse_vector_literal(text).ok();
    }
    value.as_array().and_then(|items| {
        let values = items
            .iter()
            .filter_map(|item| item.as_i64())
            .filter_map(|item| i16::try_from(item).ok())
            .collect::<Vec<_>>();
        (!values.is_empty()).then_some(values)
    })
}

fn doc_id_from_payload(payload: &[u8]) -> Option<String> {
    String::from_utf8_lossy(payload)
        .lines()
        .find_map(|line| line.strip_prefix("doc_id=").map(str::to_owned))
}

fn process_memory_report() -> Value {
    let (rss_bytes, peak_rss_bytes) = linux_proc_status_memory_bytes().unwrap_or((0, 0));
    json!({
        "rss_bytes": rss_bytes,
        "peak_rss_bytes": peak_rss_bytes,
    })
}

fn linux_proc_status_memory_bytes() -> Option<(u64, u64)> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    let mut rss_bytes = 0;
    let mut peak_rss_bytes = 0;
    for line in status.lines() {
        if let Some(value) = line.strip_prefix("VmRSS:") {
            rss_bytes = parse_status_kib(value).unwrap_or(0);
        } else if let Some(value) = line.strip_prefix("VmHWM:") {
            peak_rss_bytes = parse_status_kib(value).unwrap_or(0);
        }
    }
    Some((rss_bytes, peak_rss_bytes.max(rss_bytes)))
}

fn parse_status_kib(value: &str) -> Option<u64> {
    let kib = value.split_whitespace().next()?.parse::<u64>().ok()?;
    kib.checked_mul(1024)
}

fn bench_view() -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: None,
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope_id("bench:enterprise_rag")]),
        writable_scopes: BTreeSet::new(),
        allowed_modes: BTreeSet::from([RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::from([MemoryType::Decision]),
        max_context_budget_tokens: 1_000,
        default_context_budget_tokens: 400,
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

fn source_types(row: &Value) -> Vec<String> {
    row.get("source_types")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use serde_json::json;

    use super::{
        doc_id_from_payload, load_id_vectors, parse_query_vector, parse_status_kib,
        reject_oracle_fields, throughput_per_sec,
    };

    #[test]
    fn official_clean_rejects_question_type_and_source_types() {
        let rows = vec![json!({
            "question_id": "q1",
            "question": "What happened?",
            "question_type": "basic",
            "source_types": ["gmail"]
        })];

        let error = reject_oracle_fields(&rows).expect_err("oracle fields should fail");

        assert!(error.contains("question_type"));
        assert!(error.contains("source_types"));
    }

    #[test]
    fn official_clean_accepts_clean_question_rows() {
        let rows = vec![json!({
            "question_id": "q1",
            "question": "What happened?"
        })];

        reject_oracle_fields(&rows).expect("clean rows should pass");
    }

    #[test]
    fn parses_query_vector_from_string_or_array() {
        assert_eq!(
            parse_query_vector(&json!({"vector": "1,2,-3"})),
            Some(vec![1, 2, -3])
        );
        assert_eq!(
            parse_query_vector(&json!({"vector": [4, 5, 6]})),
            Some(vec![4, 5, 6])
        );
    }

    #[test]
    fn loads_id_vectors_from_jsonl() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("cortexdb-vector-loader-{unique}.jsonl"));
        fs::write(
            &path,
            r#"{"doc_id":"doc-a","vector":[1,2,3]}"#.to_owned() + "\n",
        )
        .expect("write vector jsonl");

        let vectors = load_id_vectors(Some(&path), "doc_id").expect("load vectors");

        assert_eq!(vectors.get("doc-a"), Some(&vec![1, 2, 3]));
        fs::remove_file(path).ok();
    }

    #[test]
    fn extracts_doc_id_from_payload_metadata() {
        assert_eq!(
            doc_id_from_payload(b"scope=bench:enterprise_rag\ndoc_id=abc-123\n\nbody"),
            Some("abc-123".to_owned())
        );
    }

    #[test]
    fn reports_throughput_and_linux_status_memory_bytes() {
        assert_eq!(throughput_per_sec(10, Some(500.0)), 20.0);
        assert_eq!(throughput_per_sec(10, None), 0.0);
        assert_eq!(parse_status_kib(" 123 kB"), Some(125_952));
    }
}
