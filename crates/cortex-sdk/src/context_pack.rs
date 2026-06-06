use crate::types::{
    ContextPackAnomalyResponse, ContextPackCellResponse, ContextPackResponse, ExplainResponse,
    SourceRefResponse,
};

pub type ContextPackV1 = ContextPackResponse;
pub type ContextPackCellV1 = ContextPackCellResponse;
pub type ContextPackSourceRefV1 = SourceRefResponse;
pub type ContextPackExplainV1 = ExplainResponse;
pub type ContextPackAnomalyV1 = ContextPackAnomalyResponse;

impl ContextPackResponse {
    pub const SCHEMA_VERSION_V1: &'static str = "context_pack.v1";

    pub fn is_v1(&self) -> bool {
        self.schema_version == Self::SCHEMA_VERSION_V1
    }

    pub fn cell_ids(&self) -> impl Iterator<Item = u64> + '_ {
        self.cells.iter().map(|cell| cell.cell_id)
    }

    pub fn citation_count(&self) -> usize {
        self.cells
            .iter()
            .filter(|cell| {
                cell.citation
                    .as_deref()
                    .is_some_and(|citation| !citation.is_empty())
            })
            .count()
    }

    pub fn anomaly_count(&self, code: &str) -> usize {
        self.anomalies
            .iter()
            .filter(|anomaly| anomaly.code == code)
            .count()
    }

    pub fn is_over_budget(&self) -> bool {
        self.estimated_tokens > self.token_budget_tokens
    }
}
