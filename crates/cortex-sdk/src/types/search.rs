pub use cortex_api_types::search::{
    AnnEvaluationResponse, AnnNoFallbackDecision, AnnSearchReport, AnswerGroundingReportResponse,
    AnswerGroundingSpanResponse, HnswNoFallbackProfileResponse, SearchExplainItem,
    SearchExplainResponse, SearchExplainTermContribution, SearchResponse, SearchResult,
    SearchRoutingDecision,
};

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
