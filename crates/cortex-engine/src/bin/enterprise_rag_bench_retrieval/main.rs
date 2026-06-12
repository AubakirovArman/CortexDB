use std::env;
use std::fs;
use std::process::ExitCode;
use std::time::Instant;

use args::{Args, BenchmarkRetrievalMode};
use cortex_engine::Database;
use cortex_storage::indexes::LexicalIndex;
use io::{read_jsonl, read_uuid_index, write_json, write_jsonl};
use retrieval::{BenchmarkMetadataIndex, BenchmarkRetrievalIndex};
use serde_json::json;

mod args;
mod constants;
mod document;
mod helpers;
mod ingest;
mod io;
mod logger;
mod metrics;
mod prefilter;
mod question_retrieval;
mod reporting;
mod retrieval;
#[cfg(test)]
mod tests;
mod vectors;

pub(crate) use constants::*;
#[cfg(test)]
pub(crate) use document::build_payload;
pub(crate) use helpers::{
    doc_id_to_cell_id, document_total, retrieval_row, round_ms, round_pct, throughput_per_sec,
};
use ingest::ingest_documents;
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
use question_retrieval::{
    retrieve_aql_questions, retrieve_cached_questions, retrieve_metadata_only_questions,
};
#[cfg(test)]
pub(crate) use reporting::doc_id_from_payload;
pub(crate) use reporting::required_str;
use reporting::{
    assess_vector_readiness, load_external_prefilter_retrieval, reject_oracle_fields,
    report_payload,
};
#[cfg(test)]
use reporting::{bench_view, build_benchmark_aql, parse_status_kib, quote_aql_string};
#[cfg(test)]
pub(crate) use vectors::DocumentVectorLookup;
#[cfg(test)]
use vectors::{
    body_vector_from_payload, load_id_vectors, parse_query_vector, payload_has_vector,
    vector_dot_score, BenchmarkSearchIndex,
};
use vectors::{load_document_vectors, load_query_vectors};

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
