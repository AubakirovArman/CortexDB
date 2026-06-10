use std::cmp::Reverse;

use super::{analyze_search_query, tokenize};

#[derive(Clone, Copy, Debug)]
pub struct SearchRerankInput<'a> {
    pub query_text: &'a str,
    pub query_vector: Option<&'a [i16]>,
    pub candidate_id: u64,
    pub lexical_score: u64,
    pub vector_score: u64,
    pub base_score: u64,
    pub payload: Option<&'a [u8]>,
}

pub trait SearchReranker {
    fn rerank_score(&self, input: SearchRerankInput<'_>) -> u64;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WeightedScoreReranker {
    pub lexical_weight: u32,
    pub vector_weight: u32,
    pub anchor_payload_bonus: u64,
    pub source_hint_payload_bonus: u64,
}

impl Default for WeightedScoreReranker {
    fn default() -> Self {
        Self {
            lexical_weight: 2,
            vector_weight: 2,
            anchor_payload_bonus: 25_000,
            source_hint_payload_bonus: 10_000,
        }
    }
}

impl SearchReranker for WeightedScoreReranker {
    fn rerank_score(&self, input: SearchRerankInput<'_>) -> u64 {
        let lexical = input
            .lexical_score
            .saturating_mul(u64::from(self.lexical_weight));
        let vector = input
            .vector_score
            .saturating_mul(u64::from(self.vector_weight));
        input
            .base_score
            .saturating_add(lexical)
            .saturating_add(vector)
            .saturating_add(payload_signal_bonus(input, self))
    }
}

pub(crate) fn sort_reranked<T>(
    values: &mut [T],
    mut candidate_id: impl FnMut(&T) -> u32,
    mut score: impl FnMut(&T) -> u64,
) {
    values.sort_by_key(|value| (Reverse(score(value)), candidate_id(value)));
}

fn payload_signal_bonus(input: SearchRerankInput<'_>, reranker: &WeightedScoreReranker) -> u64 {
    let Some(payload) = input.payload else {
        return 0;
    };
    let payload = String::from_utf8_lossy(payload).to_lowercase();
    let analyzed = analyze_search_query(input.query_text);
    let mut bonus = 0u64;
    for anchor in analyzed.anchors {
        for term in anchor.terms {
            if payload.contains(&term) {
                bonus = bonus.saturating_add(reranker.anchor_payload_bonus);
            }
        }
    }
    for source in analyzed.source_hints {
        if payload.contains(&source) {
            bonus = bonus.saturating_add(reranker.source_hint_payload_bonus);
        }
    }
    for term in tokenize(input.query_text) {
        if payload.contains(&term) {
            bonus = bonus.saturating_add(1_000);
        }
    }
    bonus
}

#[cfg(test)]
mod tests {
    use super::{SearchRerankInput, SearchReranker, WeightedScoreReranker};

    #[test]
    fn weighted_reranker_rewards_anchor_payload_matches() {
        let reranker = WeightedScoreReranker::default();
        let matched = reranker.rerank_score(SearchRerankInput {
            query_text: "Which PR #42 fixed AUTH-123?",
            query_vector: None,
            candidate_id: 1,
            lexical_score: 1,
            vector_score: 0,
            base_score: 1,
            payload: Some(b"scope=project\n\nAUTH-123 was fixed by PR #42."),
        });
        let unmatched = reranker.rerank_score(SearchRerankInput {
            query_text: "Which PR #42 fixed AUTH-123?",
            query_vector: None,
            candidate_id: 2,
            lexical_score: 1,
            vector_score: 0,
            base_score: 1,
            payload: Some(b"scope=project\n\nGeneral engineering update."),
        });

        assert!(matched > unmatched);
    }
}
