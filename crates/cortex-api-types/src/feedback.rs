use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FeedbackRecordResponse {
    pub seq: u64,
    pub cell_id: u64,
    pub source_cell_id: u64,
    pub useful: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FeedbackCellStatsResponse {
    pub source_cell_id: u64,
    pub useful: u32,
    pub not_useful: u32,
    pub score: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FeedbackStatsResponse {
    pub total: u32,
    pub useful: u32,
    pub not_useful: u32,
    pub by_source_cell: Vec<FeedbackCellStatsResponse>,
}
