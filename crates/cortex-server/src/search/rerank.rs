use std::cmp::Reverse;

use cortex_engine::{
    DatabaseSearchResult, SearchRerankInput, SearchReranker, WeightedScoreReranker,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SearchRerankMode {
    None,
    Weighted,
}

impl SearchRerankMode {
    pub(super) fn candidate_limit(self, requested_limit: usize) -> usize {
        match self {
            Self::None => requested_limit,
            Self::Weighted => requested_limit.max(32),
        }
    }

    pub(super) fn response_label(self) -> Option<String> {
        match self {
            Self::None => None,
            Self::Weighted => Some("weighted".to_owned()),
        }
    }

    pub(super) fn apply(
        self,
        results: &mut Vec<DatabaseSearchResult>,
        query_text: &str,
        query_vector: Option<&[i16]>,
        requested_limit: usize,
    ) {
        if self == Self::Weighted {
            let reranker = WeightedScoreReranker::default();
            for result in results.iter_mut() {
                result.score = reranker.rerank_score(SearchRerankInput {
                    query_text,
                    query_vector,
                    candidate_id: result.cell_id.0,
                    lexical_score: result.lexical_score,
                    vector_score: result.vector_score,
                    base_score: result.score,
                    metadata: Some(&result.metadata),
                    payload: Some(&result.payload),
                });
            }
            results.sort_by_key(|result| (Reverse(result.score), result.cell_id.0));
        }
        results.truncate(requested_limit);
    }
}
