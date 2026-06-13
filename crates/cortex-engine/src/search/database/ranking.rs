use std::cmp::Reverse;

use crate::query::CellMetadata;
use crate::source_trust::SourceTrust;

use super::super::conditions::extract_query_conditions;
use super::super::hnsw::DistanceMetric;
use super::super::routing::{
    classify_search_query_intent, routed_candidate_limit, routed_result_limit, SearchQueryIntent,
};
use super::super::vector::{vector_from_payload, vectors_from_payload};
use super::super::{SearchMode, SearchQuery, SearchRerankInput, SearchReranker};
use super::DatabaseSearchResult;

pub(super) fn database_index_query(query: SearchQuery<'_>) -> SearchQuery<'_> {
    if query.mode == SearchMode::HybridRerank {
        SearchQuery {
            mode: SearchMode::Hybrid,
            limit: search_rerank_candidate_limit(query),
            ..query
        }
    } else {
        query
    }
}

pub(super) fn search_rerank_candidate_limit(query: SearchQuery<'_>) -> usize {
    routed_candidate_limit(query.text, query.limit).max(query.limit.max(32))
}

pub(super) fn search_rerank_result_limit(query: SearchQuery<'_>) -> usize {
    routed_result_limit(query.text, query.limit)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BestPayloadVector {
    pub(super) view_name: Option<String>,
    pub(super) vector: Vec<i16>,
    pub(super) score: u64,
}

pub(super) fn best_payload_vector_for_query(
    payload: &[u8],
    query_vector: Option<&[i16]>,
) -> Option<BestPayloadVector> {
    let Some(query_vector) = query_vector else {
        return vector_from_payload(payload).map(|vector| BestPayloadVector {
            view_name: Some("body".to_owned()),
            vector,
            score: 0,
        });
    };
    vectors_from_payload(payload)
        .into_iter()
        .filter(|view| view.vector.len() == query_vector.len())
        .filter_map(|view| {
            let score = DistanceMetric::DotProduct.distance(query_vector, &view.vector)?;
            Some(BestPayloadVector {
                view_name: Some(view.name),
                vector: view.vector,
                score,
            })
        })
        .max_by_key(|view| view.score)
}

pub(super) fn rerank_database_results(
    results: &mut [DatabaseSearchResult],
    query: SearchQuery<'_>,
    reranker: &dyn SearchReranker,
) {
    let recency_scores = search_result_recency_scores_q16(results);
    for (index, result) in results.iter_mut().enumerate() {
        result.score = reranker
            .rerank_score(SearchRerankInput {
                query_text: query.text,
                query_vector: query.vector,
                candidate_id: result.cell_id.0,
                lexical_score: result.lexical_score,
                vector_score: result.vector_score,
                base_score: result.score,
                metadata: Some(&result.metadata),
                payload: Some(&result.payload),
            })
            .saturating_add(search_metadata_rerank_bonus(
                query.text,
                &result.metadata,
                recency_scores.get(index).copied().unwrap_or(0),
            ));
    }
    results.sort_by_key(|result| (Reverse(result.score), result.cell_id.0));
}

fn search_result_recency_scores_q16(results: &[DatabaseSearchResult]) -> Vec<u64> {
    let created = results
        .iter()
        .map(|result| result.metadata.created_unix_seconds)
        .collect::<Vec<_>>();
    let mut timestamps = created.iter().filter_map(|value| *value);
    let Some(first) = timestamps.next() else {
        return vec![0; results.len()];
    };
    let (min, max) = timestamps.fold((first, first), |(min, max), value| {
        (min.min(value), max.max(value))
    });
    if max <= min {
        return created
            .into_iter()
            .map(|value| value.map(|_| u64::from(u16::MAX)).unwrap_or(0))
            .collect();
    }
    let span = max - min;
    created
        .into_iter()
        .map(|value| {
            value
                .map(|created| (created.saturating_sub(min).min(span) * u64::from(u16::MAX)) / span)
                .unwrap_or(0)
        })
        .collect()
}

fn search_metadata_rerank_bonus(
    query_text: &str,
    metadata: &CellMetadata,
    recency_q16: u64,
) -> u64 {
    let trust_q16 = u64::from(
        SourceTrust::from_metadata(metadata.source_trust_q16, metadata.source_trust_class).q16,
    );
    let (trust_weight_q16, freshness_weight_q16) = metadata_rerank_weights(query_text);
    weighted_metadata_component(trust_q16, trust_weight_q16).saturating_add(
        weighted_metadata_component(recency_q16, freshness_weight_q16),
    )
}

fn metadata_rerank_weights(query_text: &str) -> (u64, u64) {
    let temporal = extract_query_conditions(query_text)
        .temporal_range
        .is_some()
        || looks_temporal_or_current(query_text);
    match classify_search_query_intent(query_text) {
        SearchQueryIntent::ConflictingInfo => (18_000, 24_000),
        SearchQueryIntent::Constrained if temporal => (12_000, 22_000),
        _ if temporal => (8_000, 18_000),
        SearchQueryIntent::InfoNotFound => (4_000, 2_000),
        _ => (3_000, 3_000),
    }
}

fn looks_temporal_or_current(query_text: &str) -> bool {
    let lower = query_text.to_ascii_lowercase();
    [
        "latest",
        "current",
        "newest",
        "recent",
        "updated",
        "as of",
        "previously",
        "before",
        "after",
        "changed",
        "change",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn weighted_metadata_component(value_q16: u64, weight_q16: u64) -> u64 {
    value_q16.saturating_mul(1024).saturating_mul(weight_q16) / u64::from(u16::MAX)
}
