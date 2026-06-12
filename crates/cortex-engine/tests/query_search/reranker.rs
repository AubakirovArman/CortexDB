use super::common::prelude::*;

#[test]
fn search_indexes_support_pluggable_reranker() {
    struct PromoteCandidate(u64);

    impl SearchReranker for PromoteCandidate {
        fn rerank_score(&self, input: SearchRerankInput<'_>) -> u64 {
            if input.candidate_id == self.0 {
                input.base_score.saturating_add(10_000_000)
            } else {
                input.base_score
            }
        }
    }

    let mut indexes = SearchIndexes::default();
    indexes.add_document(1, "budget budget budget");
    indexes.add_document(2, "budget");

    let results = indexes.search_with_reranker(
        SearchQuery {
            text: "budget",
            vector: None,
            limit: 1,
            mode: SearchMode::Keyword,
        },
        &PromoteCandidate(2),
    );

    assert_eq!(results[0].cell_id, 2);
    assert!(results[0].score > results[0].lexical_score);
}

#[test]
fn search_with_reranker_uses_route_policy_candidate_depth() {
    struct PromoteCandidate(u64);

    impl SearchReranker for PromoteCandidate {
        fn rerank_score(&self, input: SearchRerankInput<'_>) -> u64 {
            if input.candidate_id == self.0 {
                input.base_score.saturating_add(10_000_000)
            } else {
                input.base_score
            }
        }
    }

    let mut indexes = SearchIndexes::default();
    for id in 1..=35 {
        indexes.add_document(id, "project blocker");
    }

    let results = indexes.search_with_reranker(
        SearchQuery {
            text: "List all project blockers",
            vector: None,
            limit: 5,
            mode: SearchMode::Keyword,
        },
        &PromoteCandidate(35),
    );

    assert_eq!(results[0].cell_id, 35);
}

#[test]
fn search_with_reranker_uses_adaptive_result_limit() {
    struct IdentityReranker;

    impl SearchReranker for IdentityReranker {
        fn rerank_score(&self, input: SearchRerankInput<'_>) -> u64 {
            input.base_score
        }
    }

    let mut indexes = SearchIndexes::default();
    for id in 1..=12 {
        indexes.add_document(id, &format!("invoice q4 payment record {id}"));
    }
    for id in 13..=24 {
        indexes.add_document(id, &format!("project blockers evidence {id}"));
    }

    let lookup_results = indexes.search_with_reranker(
        SearchQuery {
            text: "Find invoice Q4",
            vector: None,
            limit: 10,
            mode: SearchMode::Keyword,
        },
        &IdentityReranker,
    );

    let broad_results = indexes.search_with_reranker(
        SearchQuery {
            text: "List all project blockers",
            vector: None,
            limit: 10,
            mode: SearchMode::Keyword,
        },
        &IdentityReranker,
    );

    assert_eq!(lookup_results.len(), 5);
    assert_eq!(broad_results.len(), 10);
}
