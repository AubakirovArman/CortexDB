mod ingest_stage;
mod output;
mod retrieve_stage;
mod startup;

use std::collections::BTreeMap;
use std::time::Instant;

use cortex_engine::Database;
use cortex_storage::indexes::LexicalIndex;
use serde_json::Value;

use crate::args::Args;
use crate::logger::RunLogger;
use crate::metrics::RunReportMetrics;
use crate::prefilter::ExternalPrefilterRetrieval;
use crate::vectors::DocumentVectorLookup;

pub(crate) fn run() -> Result<(), String> {
    let started = Instant::now();
    let mut state = startup::load(started)?;
    ingest_stage::run(&mut state)?;
    let rows = retrieve_stage::run(&mut state)?;
    output::write(&mut state, started, &rows)?;
    Ok(())
}

pub(super) struct PipelineState {
    pub(super) args: Args,
    pub(super) logger: RunLogger,
    pub(super) report_metrics: RunReportMetrics,
    pub(super) questions: Vec<Value>,
    pub(super) uuid_index: BTreeMap<String, String>,
    pub(super) db: Database,
    pub(super) engine_search_mode: bool,
    pub(super) document_vectors: DocumentVectorLookup,
    pub(super) external_prefilter_retrieval: Option<ExternalPrefilterRetrieval>,
    pub(super) use_source_payload_prefilter: bool,
    pub(super) ingest_lexical: Option<LexicalIndex>,
}
