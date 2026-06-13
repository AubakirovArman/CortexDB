use std::env;
use std::fs;
use std::time::Instant;

use cortex_engine::Database;
use cortex_storage::indexes::LexicalIndex;
use serde_json::json;

use crate::args::{Args, BenchmarkRetrievalMode};
use crate::helpers::document_total;
use crate::io::{read_jsonl, read_uuid_index};
use crate::logger::RunLogger;
use crate::metrics::RunReportMetrics;
use crate::prefilter::should_use_source_payload_prefilter;
use crate::reporting::{load_external_prefilter_retrieval, reject_oracle_fields};
use crate::vectors::load_document_vectors;
use crate::MAX_SKIP_CHECKPOINT_ENGINE_DOCS;

use super::PipelineState;

pub(super) fn load(started: Instant) -> Result<PipelineState, String> {
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
    let db =
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

    let document_vectors = load_document_vectors(args.document_vectors.as_ref())?;
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
    let ingest_lexical = collect_ingest_lexical.then(LexicalIndex::default);

    Ok(PipelineState {
        args,
        logger,
        report_metrics,
        questions,
        uuid_index,
        db,
        engine_search_mode,
        document_vectors,
        external_prefilter_retrieval,
        use_source_payload_prefilter,
        ingest_lexical,
    })
}
