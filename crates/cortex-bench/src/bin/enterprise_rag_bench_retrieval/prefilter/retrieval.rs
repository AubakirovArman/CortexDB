use std::collections::BTreeMap;
use std::time::Instant;

use cortex_engine::search::{SearchMode, SearchQuery, WeightedScoreReranker};
use cortex_engine::Database;
use serde_json::{json, Value};

use super::super::args::{Args, BenchmarkRetrievalMode};
use super::super::logger::RunLogger;
use super::super::metrics::{DiversityRunMetrics, SearchRetrievalOutput};
use super::super::reporting::{bench_view, required_str};
use super::super::vectors::BenchmarkSearchIndex;
use super::super::{doc_id_to_cell_id, retrieval_row, round_ms};
use super::candidate::SearchPrefilterContext;
use super::external::ExternalPrefilterRetrieval;
use super::search_index::search_prefilter_payloads;
use super::source_payload::source_prefilter_payloads;

pub(crate) fn should_use_source_payload_prefilter(
    args: &Args,
    engine_search_mode: bool,
    external_prefilter_retrieval: &Option<ExternalPrefilterRetrieval>,
) -> bool {
    engine_search_mode
        && external_prefilter_retrieval.is_some()
        && !args.disable_search_prefilter
        && args.official_clean
}

pub(crate) fn retrieve_search_questions(
    db: &Database,
    uuid_index: &BTreeMap<String, String>,
    questions: &[Value],
    query_vectors: &BTreeMap<String, Vec<i16>>,
    mut prefilter: SearchPrefilterContext<'_>,
    args: &Args,
    logger: &RunLogger,
) -> Result<SearchRetrievalOutput, String> {
    let mut rows = Vec::with_capacity(questions.len());
    let mut diversity = DiversityRunMetrics::default();
    let view = bench_view();
    let reranker = WeightedScoreReranker::default();
    let doc_to_cell = if prefilter.index.is_some() {
        Some(doc_id_to_cell_id(uuid_index)?)
    } else {
        None
    };
    let reusable_index = if args.skip_checkpoint && prefilter.index.is_none() {
        logger.log("build reusable search index for skip-checkpoint retrieval");
        logger.status(
            "build_reusable_search_index",
            "running",
            "build reusable in-memory search index",
            None,
            Some(uuid_index.len()),
            &[],
        );
        let started = Instant::now();
        let index = BenchmarkSearchIndex::load(db, uuid_index, &view, logger)?;
        logger.log(&format!(
            "reusable search index built in {} ms",
            round_ms(started.elapsed().as_secs_f64() * 1000.0)
        ));
        logger.status(
            "build_reusable_search_index",
            "done",
            "reusable in-memory search index built",
            Some(uuid_index.len()),
            Some(uuid_index.len()),
            &[],
        );
        Some(index)
    } else {
        None
    };
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
            BenchmarkRetrievalMode::EngineHybridRerank => {
                let vector = query_vectors.get(qid).ok_or_else(|| {
                    format!("engine-hybrid-rerank requires query vector for question_id={qid}")
                })?;
                (SearchMode::HybridRerank, Some(vector.as_slice()))
            }
            BenchmarkRetrievalMode::CachedLexical | BenchmarkRetrievalMode::EngineAql => {
                unreachable!("cached and AQL modes handled separately")
            }
        };
        let search_query = SearchQuery {
            text: query,
            vector,
            limit: args.top_k,
            mode,
        };
        let mode_label = args.retrieval_mode.as_str();
        logger.status(
            "retrieve_questions",
            "running",
            "retrieve engine question row",
            Some(index),
            Some(questions.len()),
            &[
                ("current_question_id", json!(qid)),
                ("retrieval_mode", json!(mode_label)),
                ("rerank_mode", json!(args.rerank_mode.as_str())),
            ],
        );
        logger.log(&format!(
            "retrieve engine question {}/{} question_id={} mode={} rerank={}",
            index + 1,
            questions.len(),
            qid,
            mode_label,
            args.rerank_mode.as_str()
        ));
        let question_started = Instant::now();
        let payloads = if let (Some(source_payloads), Some(external_retrieval)) = (
            prefilter.source_payloads.as_deref_mut(),
            prefilter.external_retrieval,
        ) {
            let output = source_prefilter_payloads(
                source_payloads,
                prefilter.document_vectors,
                external_retrieval,
                qid,
                search_query,
                args.top_k,
                args.rerank_mode.is_enabled().then_some(&reranker),
            )?;
            if let Some(diagnostics) = &output.diversity_diagnostics {
                diversity.record(diagnostics);
            }
            output.payloads
        } else if let (Some(_), Some(doc_to_cell)) = (prefilter.index, doc_to_cell.as_ref()) {
            let output = search_prefilter_payloads(
                db,
                doc_to_cell,
                &mut prefilter,
                qid,
                search_query,
                args.top_k,
                args.rerank_mode.is_enabled().then_some(&reranker),
            );
            if let Some(diagnostics) = &output.diversity_diagnostics {
                diversity.record(diagnostics);
            }
            output.payloads
        } else if let Some(reusable_index) = &reusable_index {
            reusable_index.search_payloads(
                db,
                search_query,
                args.top_k,
                args.rerank_mode.is_enabled().then_some(&reranker),
            )
        } else if args.rerank_mode.is_enabled() {
            db.search_cells_with_reranker(search_query, &view, &reranker)
                .map_err(|error| {
                    format!("engine {mode_label} rerank search failed for {qid}: {error}")
                })?
                .into_iter()
                .map(|result| result.payload)
                .collect()
        } else {
            let outcome = db
                .search_cells_with_report(search_query, &view)
                .map_err(|error| format!("engine {mode_label} search failed for {qid}: {error}"))?;
            if let Some(diagnostics) = &outcome.diversity_diagnostics {
                diversity.record(diagnostics);
            }
            outcome
                .results
                .into_iter()
                .map(|result| result.payload)
                .collect()
        };
        rows.push(retrieval_row(qid, query, payloads));
        let question_duration_ms = round_ms(question_started.elapsed().as_secs_f64() * 1000.0);
        if args.progress_every > 0
            && ((index + 1).is_multiple_of(args.progress_every)
                || questions.len() <= args.progress_every)
        {
            logger.log(&format!(
                "retrieved {}/{} last_question_ms={question_duration_ms}",
                index + 1,
                questions.len()
            ));
            logger.status(
                "retrieve_questions",
                "running",
                "retrieve engine question rows",
                Some(index + 1),
                Some(questions.len()),
                &[
                    ("last_question_id", json!(qid)),
                    ("last_question_ms", json!(question_duration_ms)),
                    ("retrieval_mode", json!(mode_label)),
                    ("rerank_mode", json!(args.rerank_mode.as_str())),
                ],
            );
        }
    }
    Ok(SearchRetrievalOutput { rows, diversity })
}
