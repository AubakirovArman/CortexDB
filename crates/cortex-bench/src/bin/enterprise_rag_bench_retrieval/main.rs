use std::process::ExitCode;

mod args;
mod constants;
mod document;
mod helpers;
mod ingest;
mod io;
mod logger;
mod metrics;
mod pipeline;
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
#[cfg(test)]
pub(crate) use helpers::throughput_per_sec;
pub(crate) use helpers::{doc_id_to_cell_id, retrieval_row, round_ms};
use io::read_jsonl;
#[cfg(test)]
use logger::RunLogger;
#[cfg(test)]
use metrics::DiversityRunMetrics;
use pipeline::run;
#[cfg(test)]
use prefilter::SourcePayloadPrefilter;
#[cfg(test)]
use prefilter::{
    inferred_source_types_from_query, merge_prefilter_candidates, prefilter_default_doc_limit,
    prefilter_evidence_score, prefilter_lexical_head_count, select_diverse_prefilter_candidates,
    source_prefilter_payloads, ExternalPrefilterRetrieval, PrefilterCandidate,
};
#[cfg(test)]
pub(crate) use reporting::doc_id_from_payload;
pub(crate) use reporting::required_str;
#[cfg(test)]
use reporting::{bench_view, build_benchmark_aql, parse_status_kib, quote_aql_string};
#[cfg(test)]
use reporting::{load_external_prefilter_retrieval, reject_oracle_fields};
#[cfg(test)]
use vectors::load_document_vectors;
#[cfg(test)]
pub(crate) use vectors::DocumentVectorLookup;
#[cfg(test)]
use vectors::{
    body_vector_from_payload, load_id_vectors, parse_query_vector, payload_has_vector,
    vector_dot_score, BenchmarkSearchIndex,
};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
