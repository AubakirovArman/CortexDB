use std::collections::BTreeMap;
use std::time::Instant;

use serde_json::{json, Value};

use crate::args::BenchmarkRetrievalMode;
use crate::helpers::round_ms;
use crate::prefilter::{retrieve_search_questions, SearchPrefilterContext, SourcePayloadPrefilter};
use crate::question_retrieval::{
    retrieve_aql_questions, retrieve_cached_questions, retrieve_metadata_only_questions,
};
use crate::reporting::assess_vector_readiness;
use crate::retrieval::{BenchmarkMetadataIndex, BenchmarkRetrievalIndex};
use crate::vectors::load_query_vectors;

use super::PipelineState;

pub(super) fn run(state: &mut PipelineState) -> Result<Vec<Value>, String> {
    let query_vectors = load_query_vectors(state.args.query_vectors.as_ref())?;
    let vector_readiness = assess_vector_readiness(
        &state.db,
        &state.uuid_index,
        state.args.retrieval_mode,
        state.questions.len(),
        query_vectors.len(),
        state.document_vectors.len(),
        state.use_source_payload_prefilter,
    )?;
    if !vector_readiness.ready {
        state.logger.log(&format!(
            "vector readiness warning: {}",
            vector_readiness.warnings.join("; ")
        ));
    }
    state.logger.status(
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
    state.report_metrics.vector_readiness = Some(vector_readiness);
    state.logger.log("begin question retrieval");
    state.logger.status(
        "retrieve_questions",
        "running",
        "begin question retrieval",
        Some(0),
        Some(state.questions.len()),
        &[],
    );
    let retrieval_started = Instant::now();
    let metadata_index = BenchmarkMetadataIndex::from_uuid_index(&state.uuid_index);
    let ingest_prefilter_index = state
        .ingest_lexical
        .take()
        .map(|lexical| BenchmarkRetrievalIndex::from_lexical(lexical, &state.uuid_index));
    let checkpoint_search_prefilter_index =
        checkpoint_prefilter_index(state, &ingest_prefilter_index)?;
    let search_prefilter_index = if state.args.disable_search_prefilter {
        None
    } else {
        ingest_prefilter_index
            .as_ref()
            .or(checkpoint_search_prefilter_index.as_ref())
    };
    let rows = retrieve_rows(
        state,
        &metadata_index,
        &query_vectors,
        search_prefilter_index,
    )?;
    state.report_metrics.retrieval_duration_ms =
        Some(round_ms(retrieval_started.elapsed().as_secs_f64() * 1000.0));
    state.report_metrics.questions_retrieved = rows.len();
    state
        .logger
        .log(&format!("retrieved {} question rows", rows.len()));
    state.logger.status(
        "retrieve_questions",
        "done",
        "question retrieval done",
        Some(rows.len()),
        Some(state.questions.len()),
        &[],
    );
    Ok(rows)
}

fn checkpoint_prefilter_index(
    state: &PipelineState,
    ingest_prefilter_index: &Option<BenchmarkRetrievalIndex>,
) -> Result<Option<BenchmarkRetrievalIndex>, String> {
    if state.engine_search_mode
        && ingest_prefilter_index.is_none()
        && !state.args.skip_checkpoint
        && !state.args.disable_search_prefilter
        && !state.use_source_payload_prefilter
    {
        state
            .logger
            .log("load cached lexical prefilter index for engine search retrieval");
        Ok(Some(BenchmarkRetrievalIndex::load(
            &state.db,
            &state.uuid_index,
        )?))
    } else {
        Ok(None)
    }
}

fn retrieve_rows(
    state: &mut PipelineState,
    metadata_index: &BenchmarkMetadataIndex,
    query_vectors: &BTreeMap<String, Vec<i16>>,
    search_prefilter_index: Option<&BenchmarkRetrievalIndex>,
) -> Result<Vec<Value>, String> {
    if let Some(rows) = retrieve_metadata_only_questions(
        metadata_index,
        &state.questions,
        &state.args,
        &state.logger,
    )? {
        return Ok(rows);
    }
    let mut source_payload_prefilter = if state.use_source_payload_prefilter {
        Some(SourcePayloadPrefilter::new(
            &state.uuid_index,
            state.args.sources_dir.clone(),
        )?)
    } else {
        None
    };
    match state.args.retrieval_mode {
        BenchmarkRetrievalMode::CachedLexical => retrieve_cached(state),
        BenchmarkRetrievalMode::EngineAql => retrieve_aql(state, query_vectors),
        BenchmarkRetrievalMode::EngineKeyword
        | BenchmarkRetrievalMode::EngineHybrid
        | BenchmarkRetrievalMode::EngineHybridRerank => {
            let output = retrieve_search_questions(
                &state.db,
                &state.uuid_index,
                &state.questions,
                query_vectors,
                SearchPrefilterContext {
                    index: search_prefilter_index,
                    source_payloads: source_payload_prefilter.as_mut(),
                    document_vectors: &mut state.document_vectors,
                    external_retrieval: state.external_prefilter_retrieval.as_ref(),
                },
                &state.args,
                &state.logger,
            )?;
            state.report_metrics.diversity = output.diversity;
            Ok(output.rows)
        }
    }
}

fn retrieve_cached(state: &PipelineState) -> Result<Vec<Value>, String> {
    state
        .logger
        .log("load cached lexical retrieval index from checkpoint segments");
    state.logger.status(
        "load_retrieval_index",
        "running",
        "load cached lexical retrieval index",
        None,
        None,
        &[],
    );
    let retrieval_index = BenchmarkRetrievalIndex::load(&state.db, &state.uuid_index)?;
    state.logger.log("cached lexical retrieval index loaded");
    state.logger.status(
        "load_retrieval_index",
        "done",
        "cached lexical retrieval index loaded",
        None,
        None,
        &[],
    );
    retrieve_cached_questions(
        &retrieval_index,
        &state.questions,
        &state.args,
        &state.logger,
    )
}

fn retrieve_aql(
    state: &PipelineState,
    query_vectors: &BTreeMap<String, Vec<i16>>,
) -> Result<Vec<Value>, String> {
    state
        .logger
        .log("load cached lexical prefilter index for AQL retrieval");
    let retrieval_index = BenchmarkRetrievalIndex::load(&state.db, &state.uuid_index)?;
    retrieve_aql_questions(
        &state.db,
        &retrieval_index,
        &state.uuid_index,
        &state.questions,
        query_vectors,
        &state.args,
        &state.logger,
    )
}
