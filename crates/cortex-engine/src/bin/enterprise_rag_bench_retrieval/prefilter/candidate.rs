use cortex_core::CellId;

use super::super::retrieval::BenchmarkRetrievalIndex;
use super::super::vectors::DocumentVectorLookup;
use super::external::ExternalPrefilterRetrieval;
use super::source_payload::SourcePayloadPrefilter;
use cortex_engine::search::SearchDiversityDiagnostics;

#[derive(Clone)]
pub(crate) struct PrefilterCandidate {
    pub(crate) cell_id: CellId,
    pub(crate) payload: Vec<u8>,
    pub(crate) score: u64,
    pub(crate) lexical_score: u64,
    pub(crate) vector_score: u64,
    pub(crate) evidence_score: u32,
}

pub(crate) struct PrefilterSearchOutput {
    pub(crate) payloads: Vec<Vec<u8>>,
    pub(crate) diversity_diagnostics: Option<SearchDiversityDiagnostics>,
}

pub(crate) struct SearchPrefilterContext<'a> {
    pub(crate) index: Option<&'a BenchmarkRetrievalIndex>,
    pub(crate) source_payloads: Option<&'a mut SourcePayloadPrefilter>,
    pub(crate) document_vectors: &'a mut DocumentVectorLookup,
    pub(crate) external_retrieval: Option<&'a ExternalPrefilterRetrieval>,
}
