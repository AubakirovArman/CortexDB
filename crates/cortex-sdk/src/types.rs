use serde::Deserialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VectorAlgorithm {
    Ann,
    Exact,
}

impl VectorAlgorithm {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Ann => "ann",
            Self::Exact => "exact",
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct AnnSearchReport {
    pub path: String,
    pub fallback_reason: Option<String>,
    pub requested_limit: usize,
    pub allowed_candidates: usize,
    pub graph_nodes: usize,
    pub returned_candidates: usize,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct AnnEvaluationResponse {
    pub available: bool,
    pub reason: Option<String>,
    pub ann_report: Option<AnnSearchReport>,
    pub exact_top_k: Vec<u32>,
    pub ann_top_k: Vec<u32>,
    pub overlap_count: usize,
    pub recall_q16: u16,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct SearchResult {
    pub cell_id: u64,
    pub score: u64,
    pub lexical_score: u64,
    pub vector_score: u64,
    pub payload: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct SearchResponse {
    pub search_mode: String,
    pub ann_report: Option<AnnSearchReport>,
    pub results: Vec<SearchResult>,
}
