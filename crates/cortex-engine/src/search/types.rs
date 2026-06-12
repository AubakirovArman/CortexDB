#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScoredCandidate {
    pub cell_id: u32,
    pub score: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchMode {
    Keyword,
    Vector,
    VectorExact,
    Hybrid,
    HybridRerank,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchQuery<'a> {
    pub text: &'a str,
    pub vector: Option<&'a [i16]>,
    pub limit: usize,
    pub mode: SearchMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchResult {
    pub cell_id: u32,
    pub score: u64,
    pub lexical_score: u64,
    pub vector_score: u64,
}
