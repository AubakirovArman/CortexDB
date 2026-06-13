use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub struct ConflictRecordResponse {
    pub cell_id: u64,
    pub relation_cell_id: Option<u64>,
    pub source_cell_id: Option<u64>,
    pub fact: String,
    pub entity: Option<String>,
    pub metric: Option<String>,
    pub source: Option<String>,
    pub source_trust_q16: u16,
    pub source_trust_category: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct ConflictIndexResponse {
    pub schema_version: &'static str,
    pub scope: String,
    pub conflict_count: usize,
    pub conflicts: Vec<ConflictRecordResponse>,
}
