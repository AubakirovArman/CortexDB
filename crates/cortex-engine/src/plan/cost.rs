mod model;

pub use model::{choose_retrieve_path, estimate_bitmap_program_rows};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutionPath {
    BitmapFirst,
    LexicalFirst,
    VectorFirst,
    Hybrid,
    Pack,
}

impl ExecutionPath {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BitmapFirst => "bitmap-first",
            Self::LexicalFirst => "lexical-first",
            Self::VectorFirst => "vector-first",
            Self::Hybrid => "hybrid",
            Self::Pack => "pack",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CostModelOptions {
    pub forced_path: Option<ExecutionPath>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CostModelDecision {
    pub selected_path: ExecutionPath,
    pub reason: String,
    pub estimated_live_rows: u64,
    pub estimated_after_bitmap: Option<u64>,
    pub recommended_candidate_limit: u32,
    pub has_query_vector: bool,
    pub rarest_term: Option<TermDfEstimate>,
    pub estimates: Vec<CostModelEstimate>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TermDfEstimate {
    pub term: String,
    pub document_frequency: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CostModelEstimate {
    pub path: ExecutionPath,
    pub cost_units: u64,
}

#[cfg(test)]
mod tests;
