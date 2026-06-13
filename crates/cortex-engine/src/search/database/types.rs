use cortex_core::CellId;

use crate::query::CellMetadata;

use super::super::ann::AnnSearchReport;
use super::super::{ScoredCandidate, SearchQueryIntent};

pub(crate) struct PersistedSearchCandidate {
    pub(super) candidate_id: u32,
    pub(super) score: u64,
    pub(super) lexical_score: u64,
    pub(super) vector_score: u64,
}

impl PersistedSearchCandidate {
    pub(crate) fn from_lexical(candidate: ScoredCandidate) -> Self {
        Self {
            candidate_id: candidate.cell_id,
            score: candidate.score,
            lexical_score: candidate.score,
            vector_score: 0,
        }
    }

    pub(crate) fn from_vector(candidate: ScoredCandidate) -> Self {
        Self {
            candidate_id: candidate.cell_id,
            score: candidate.score,
            lexical_score: 0,
            vector_score: candidate.score,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchLimit(pub usize);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatabaseSearchResult {
    pub cell_id: CellId,
    pub score: u64,
    pub lexical_score: u64,
    pub vector_score: u64,
    pub metadata: CellMetadata,
    pub payload: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatabaseSearchOutcome {
    pub results: Vec<DatabaseSearchResult>,
    pub ann_report: Option<AnnSearchReport>,
    pub view_traces: Vec<SearchViewTrace>,
    pub diversity_diagnostics: Option<SearchDiversityDiagnostics>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchViewTrace {
    pub cell_id: CellId,
    pub candidate_id: u32,
    pub vector_view: Option<String>,
    pub vector_score: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchDiversityDiagnostics {
    pub intent: SearchQueryIntent,
    pub diversity_enabled: bool,
    pub lambda_q16: u16,
    pub input_candidates: usize,
    pub output_candidates: usize,
    pub skipped_candidates: usize,
    pub max_payload_similarity_q16: u64,
    pub max_cluster_similarity_q16: u64,
    pub selected_with_payload_similarity: usize,
    pub selected_with_cluster_similarity: usize,
}
