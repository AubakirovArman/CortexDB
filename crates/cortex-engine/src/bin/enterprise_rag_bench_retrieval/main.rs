use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process::ExitCode;
use std::time::Instant;

use args::{Args, BenchmarkRetrievalMode};
use cortex_aql::{default_weights, RetrievalMode};
use cortex_core::CellId;
use cortex_engine::{CellMetadata, Database, RetrievedCell};
use cortex_storage::indexes::LexicalIndex;
use document::{build_payload, extract_document_content};
use io::{read_json, read_jsonl, read_uuid_index, write_json, write_jsonl};
use retrieval::{BenchmarkMetadataIndex, BenchmarkRetrievalIndex};
use serde_json::{json, Value};

mod args;
mod document;
mod io;
mod logger;
mod metrics;
mod prefilter;
mod reporting;
mod retrieval;
#[cfg(test)]
mod tests;
mod vectors;

use logger::RunLogger;
#[cfg(test)]
use metrics::DiversityRunMetrics;
use metrics::RunReportMetrics;
#[cfg(test)]
use prefilter::{
    inferred_source_types_from_query, merge_prefilter_candidates, prefilter_default_doc_limit,
    prefilter_evidence_score, prefilter_lexical_head_count, select_diverse_prefilter_candidates,
    source_prefilter_payloads, ExternalPrefilterRetrieval, PrefilterCandidate,
};
use prefilter::{
    retrieve_search_questions, should_use_source_payload_prefilter, SearchPrefilterContext,
    SourcePayloadPrefilter,
};
use reporting::{
    assess_vector_readiness, benchmark_task, doc_id_from_payload,
    load_external_prefilter_retrieval, reject_oracle_fields, report_payload, required_str,
    source_types,
};
#[cfg(test)]
use reporting::{bench_view, build_benchmark_aql, parse_status_kib, quote_aql_string};
#[cfg(test)]
use vectors::{
    body_vector_from_payload, load_id_vectors, parse_query_vector, payload_has_vector,
    vector_dot_score, BenchmarkSearchIndex,
};
use vectors::{load_document_vectors, load_query_vectors, DocumentVectorLookup};

const ORACLE_FIELDS: &[&str] = &[
    "answer_facts",
    "expected_doc_ids",
    "gold_answer",
    "question_type",
    "source_types",
];
const CLEAN_PREFILTER_RETRIEVAL_FIELDS: &[&str] =
    &["answer", "document_ids", "question", "question_id"];
const MAX_SKIP_CHECKPOINT_ENGINE_DOCS: usize = 10_000;
const ENGINE_PREFILTER_SHORTLIST_LIMIT: usize = 128;
const ENGINE_PREFILTER_LEXICAL_HEAD_COUNT: usize = 4;
const ENGINE_PREFILTER_DEFAULT_DOC_LIMIT: usize = 6;
const ENGINE_PREFILTER_STRONG_EVIDENCE_SCORE: u32 = 32;

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

    let engine_search_mode = matches!(
        args.retrieval_mode,
        BenchmarkRetrievalMode::EngineKeyword
            | BenchmarkRetrievalMode::EngineHybrid
            | BenchmarkRetrievalMode::EngineHybridRerank
    );
    if args.skip_checkpoint
        && engine_search_mode
        && document_total(&args, uuid_index.len()) > MAX_SKIP_CHECKPOINT_ENGINE_DOCS
    {
        return Err(format!(
            "--skip-checkpoint {} is limited to <= {} documents until a persisted candidate source is available; use --max-documents for smoke tests or run against a checkpointed DB",
            args.retrieval_mode.as_str(),
            MAX_SKIP_CHECKPOINT_ENGINE_DOCS
        ));
    }

    let mut document_vectors = load_document_vectors(args.document_vectors.as_ref())?;
    if !document_vectors.is_empty() {
        logger.log(&format!(
            "loaded {} document vectors",
            document_vectors.len()
        ));
    }
    let external_prefilter_retrieval =
        load_external_prefilter_retrieval(args.prefilter_retrieval.as_deref())?;
    if let Some(prefilter_retrieval) = &external_prefilter_retrieval {
        logger.log(&format!(
            "loaded external clean prefilter retrieval rows={} questions={}",
            prefilter_retrieval.rows,
            prefilter_retrieval.by_question_id.len()
        ));
        logger.status(
            "load_prefilter_retrieval",
            "done",
            "external prefilter retrieval loaded",
            Some(prefilter_retrieval.rows),
            Some(prefilter_retrieval.rows),
            &[("questions", json!(prefilter_retrieval.by_question_id.len()))],
        );
    }
    let use_source_payload_prefilter = should_use_source_payload_prefilter(
        &args,
        engine_search_mode,
        &external_prefilter_retrieval,
    );
    report_metrics.source_payload_prefilter = use_source_payload_prefilter;
    if use_source_payload_prefilter {
        logger.log("enable source-payload prefilter path");
        logger.status(
            "source_payload_prefilter",
            "done",
            "source-payload prefilter path enabled",
            None,
            None,
            &[],
        );
    }
    let collect_ingest_lexical =
        args.skip_checkpoint && engine_search_mode && !use_source_payload_prefilter;
    let mut ingest_lexical = collect_ingest_lexical.then(LexicalIndex::default);

    if use_source_payload_prefilter {
        logger.log("skip corpus ingest; source-payload prefilter reads bounded documents directly");
        report_metrics.documents_indexed = 0;
        logger.status(
            "ingest",
            "skipped",
            "source-payload prefilter skips corpus ingest",
            Some(0),
            Some(0),
            &[],
        );
    } else if !args.skip_ingest {
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
        let indexed = ingest_documents(
            &mut db,
            &uuid_index,
            &mut document_vectors,
            ingest_lexical.as_mut(),
            &args,
            &logger,
        )?;
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
        if args.skip_checkpoint {
            logger.log("skip checkpoint");
            logger.status("checkpoint", "skipped", "skip checkpoint", None, None, &[]);
        } else {
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
        }
    } else {
        logger.log("skip corpus ingest");
        report_metrics.documents_indexed = 0;
        logger.status("ingest", "skipped", "skip corpus ingest", None, None, &[]);
    }

    let query_vectors = load_query_vectors(args.query_vectors.as_ref())?;
    let vector_readiness = assess_vector_readiness(
        &db,
        &uuid_index,
        args.retrieval_mode,
        questions.len(),
        query_vectors.len(),
        document_vectors.len(),
        use_source_payload_prefilter,
    )?;
    if !vector_readiness.ready {
        logger.log(&format!(
            "vector readiness warning: {}",
            vector_readiness.warnings.join("; ")
        ));
    }
    logger.status(
        "vector_readiness",
        if vector_readiness.ready {
            "done"
        } else {
            "warning"
        },
        "checked vector readiness",
        Some(vector_readiness.sampled_documents_with_vectors),
        Some(vector_readiness.sampled_documents),
        &[
            (
                "required_for_mode",
                json!(vector_readiness.required_for_mode),
            ),
            (
                "document_vector_sample_coverage_pct",
                json!(vector_readiness.document_vector_sample_coverage_pct),
            ),
        ],
    );
    report_metrics.vector_readiness = Some(vector_readiness);
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
    let metadata_index = BenchmarkMetadataIndex::from_uuid_index(&uuid_index);
    let ingest_prefilter_index = ingest_lexical
        .take()
        .map(|lexical| BenchmarkRetrievalIndex::from_lexical(lexical, &uuid_index));
    let checkpoint_search_prefilter_index = if engine_search_mode
        && ingest_prefilter_index.is_none()
        && !args.skip_checkpoint
        && !args.disable_search_prefilter
        && !use_source_payload_prefilter
    {
        logger.log("load cached lexical prefilter index for engine search retrieval");
        Some(BenchmarkRetrievalIndex::load(&db, &uuid_index)?)
    } else {
        None
    };
    let search_prefilter_index = if args.disable_search_prefilter {
        None
    } else {
        ingest_prefilter_index
            .as_ref()
            .or(checkpoint_search_prefilter_index.as_ref())
    };
    let rows = if let Some(rows) =
        retrieve_metadata_only_questions(&metadata_index, &questions, &args, &logger)?
    {
        rows
    } else {
        let mut source_payload_prefilter = if use_source_payload_prefilter {
            Some(SourcePayloadPrefilter::new(
                &uuid_index,
                args.sources_dir.clone(),
            )?)
        } else {
            None
        };
        match args.retrieval_mode {
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
            BenchmarkRetrievalMode::EngineAql => {
                logger.log("load cached lexical prefilter index for AQL retrieval");
                let retrieval_index = BenchmarkRetrievalIndex::load(&db, &uuid_index)?;
                retrieve_aql_questions(
                    &db,
                    &retrieval_index,
                    &uuid_index,
                    &questions,
                    &query_vectors,
                    &args,
                    &logger,
                )?
            }
            BenchmarkRetrievalMode::EngineKeyword
            | BenchmarkRetrievalMode::EngineHybrid
            | BenchmarkRetrievalMode::EngineHybridRerank => {
                let output = retrieve_search_questions(
                    &db,
                    &uuid_index,
                    &questions,
                    &query_vectors,
                    SearchPrefilterContext {
                        index: search_prefilter_index,
                        source_payloads: source_payload_prefilter.as_mut(),
                        document_vectors: &mut document_vectors,
                        external_retrieval: external_prefilter_retrieval.as_ref(),
                    },
                    &args,
                    &logger,
                )?;
                report_metrics.diversity = output.diversity;
                output.rows
            }
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

fn round_pct(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn ingest_documents(
    db: &mut Database,
    uuid_index: &BTreeMap<String, String>,
    document_vectors: &mut DocumentVectorLookup,
    mut ingest_lexical: Option<&mut LexicalIndex>,
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
        let vector = document_vectors.get(doc_id)?;
        let payload = build_payload(doc_id, rel_path, &title, &content, vector.as_deref());
        let candidate = u32::try_from(index + 1).map_err(|_| "candidate id overflow")?;
        let cell_id = CellId(u64::from(candidate));
        let payload = payload.into_bytes();
        if let Some(lexical) = ingest_lexical.as_mut() {
            index_payload_lexical(lexical, candidate, &payload);
        }
        batch.push((cell_id, payload));
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

fn index_payload_lexical(lexical: &mut LexicalIndex, candidate: u32, payload: &[u8]) {
    let metadata = CellMetadata::from_payload(payload);
    let weighted_terms = metadata.weighted_lexical_terms();
    lexical.doc_lengths.insert(
        candidate,
        weighted_terms.values().copied().sum::<u32>().max(1),
    );
    for (term, frequency) in weighted_terms {
        lexical
            .terms
            .entry(term.clone())
            .or_default()
            .insert(candidate);
        lexical
            .term_frequencies
            .entry(term)
            .or_default()
            .insert(candidate, frequency);
    }
    for (field, terms) in metadata.lexical_field_terms() {
        lexical
            .field_doc_lengths
            .entry(field.clone())
            .or_default()
            .insert(candidate, terms.values().copied().sum::<u32>().max(1));
        let field_terms = lexical.field_term_frequencies.entry(field).or_default();
        for (term, frequency) in terms {
            field_terms
                .entry(term)
                .or_default()
                .insert(candidate, frequency);
        }
    }
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

fn retrieve_metadata_only_questions(
    metadata_index: &BenchmarkMetadataIndex,
    questions: &[Value],
    args: &Args,
    logger: &RunLogger,
) -> Result<Option<Vec<Value>>, String> {
    if !args.official_clean || questions.is_empty() {
        return Ok(None);
    }
    let mut rows = Vec::with_capacity(questions.len());
    for (index, question) in questions.iter().enumerate() {
        let qid = required_str(question, "question_id", index)?;
        let query = required_str(question, "question", index)?;
        let doc_ids = metadata_index.search_doc_ids(query, args.top_k);
        if doc_ids.is_empty() {
            return Ok(None);
        }
        rows.push(json!({
            "question_id": qid,
            "question": query,
            "answer": "",
            "document_ids": doc_ids.into_iter().map(Value::String).collect::<Vec<_>>(),
        }));
    }
    logger.log("metadata-only retrieval satisfied all clean questions");
    logger.status(
        "retrieve_questions",
        "running",
        "metadata-only retrieval satisfied all clean questions",
        Some(rows.len()),
        Some(questions.len()),
        &[],
    );
    Ok(Some(rows))
}

fn retrieve_aql_questions(
    db: &Database,
    retrieval_index: &BenchmarkRetrievalIndex,
    uuid_index: &BTreeMap<String, String>,
    questions: &[Value],
    query_vectors: &BTreeMap<String, Vec<i16>>,
    args: &Args,
    logger: &RunLogger,
) -> Result<Vec<Value>, String> {
    let mut rows = Vec::with_capacity(questions.len());
    let doc_to_cell = doc_id_to_cell_id(uuid_index)?;
    for (index, question) in questions.iter().enumerate() {
        let qid = required_str(question, "question_id", index)?;
        let query = required_str(question, "question", index)?;
        let vector = query_vectors.get(qid);
        let cells = retrieval_index
            .search_doc_ids(query, &[], args.top_k.max(64))
            .iter()
            .filter_map(|doc_id| doc_to_cell.get(doc_id).copied())
            .filter_map(|cell_id| {
                db.get_latest_cell_with_descriptor(cell_id)
                    .map(|(payload, descriptor)| RetrievedCell {
                        cell_id,
                        payload,
                        descriptor,
                    })
            })
            .collect::<Vec<_>>();
        let mode = if vector.is_some() {
            RetrievalMode::Semantic
        } else {
            RetrievalMode::Balanced
        };
        let task = benchmark_task(query, vector);
        let results = db
            .rerank_retrieved_cells_for_task(cells, &task, &default_weights(mode))
            .into_iter()
            .take(args.top_k)
            .collect::<Vec<_>>();
        rows.push(retrieval_row(
            qid,
            query,
            results.into_iter().map(|result| result.payload),
        ));
        if args.progress_every > 0
            && ((index + 1).is_multiple_of(args.progress_every)
                || questions.len() <= args.progress_every)
        {
            logger.log(&format!("retrieved {}/{}", index + 1, questions.len()));
            logger.status(
                "retrieve_questions",
                "running",
                "retrieve AQL question rows",
                Some(index + 1),
                Some(questions.len()),
                &[("last_question_id", json!(qid))],
            );
        }
    }
    Ok(rows)
}

fn doc_id_to_cell_id(
    uuid_index: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, CellId>, String> {
    uuid_index
        .keys()
        .enumerate()
        .map(|(index, doc_id)| {
            let id = u64::try_from(index + 1).map_err(|_| "cell id overflow".to_owned())?;
            Ok((doc_id.clone(), CellId(id)))
        })
        .collect()
}

fn retrieval_row(qid: &str, query: &str, payloads: impl IntoIterator<Item = Vec<u8>>) -> Value {
    let mut seen = BTreeSet::new();
    let document_ids = payloads
        .into_iter()
        .filter_map(|payload| doc_id_from_payload(&payload))
        .filter(|doc_id| seen.insert(doc_id.clone()))
        .map(Value::String)
        .collect::<Vec<_>>();
    json!({
        "question_id": qid,
        "question": query,
        "answer": "",
        "document_ids": document_ids,
    })
}
