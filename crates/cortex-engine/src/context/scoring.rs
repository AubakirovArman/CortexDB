use std::cmp::Reverse;
use std::collections::BTreeMap;

use cortex_core::CellId;

use super::explain::extract_query_terms;
use super::freshness::SourceFreshness;
use super::ContextScoreComponent;
use crate::database::RetrievedCell;
use crate::search::{analyze_search_query, bm25_term_score_q16, Bm25CorpusStats, Bm25TermInput};
use crate::source_trust::SourceTrust;

pub(crate) fn score_components(
    base_bm25: u32,
    source_trust: SourceTrust,
    source_freshness: SourceFreshness,
    redundancy_penalty: u32,
    feedback_bonus: i32,
) -> Vec<ContextScoreComponent> {
    let source_trust_bonus = source_trust.score_bonus();
    let source_freshness_bonus = source_freshness.score_bonus();
    let mut components = vec![
        ContextScoreComponent {
            name: "base_bm25".to_owned(),
            value: base_bm25,
            contribution: i32::try_from(base_bm25).unwrap_or(i32::MAX),
            reason: "canonical BM25 score from query terms and cell metadata".to_owned(),
        },
        ContextScoreComponent {
            name: "source_trust_bonus".to_owned(),
            value: source_trust_bonus,
            contribution: i32::try_from(source_trust_bonus).unwrap_or(i32::MAX),
            reason: source_trust.score_reason(),
        },
        ContextScoreComponent {
            name: "source_freshness_bonus".to_owned(),
            value: source_freshness_bonus,
            contribution: i32::try_from(source_freshness_bonus).unwrap_or(i32::MAX),
            reason: source_freshness.score_reason(),
        },
        ContextScoreComponent {
            name: "redundancy_penalty".to_owned(),
            value: redundancy_penalty,
            contribution: -i32::try_from(redundancy_penalty).unwrap_or(i32::MAX),
            reason: "weighted Jaccard overlap with already packed cells".to_owned(),
        },
    ];
    if feedback_bonus != 0 {
        components.push(ContextScoreComponent {
            name: "feedback_bonus".to_owned(),
            value: feedback_bonus.unsigned_abs(),
            contribution: feedback_bonus,
            reason: "decayed useful/not-useful context feedback signal".to_owned(),
        });
    }
    components
}

pub(crate) fn order_by_feedback(
    cells: Vec<RetrievedCell>,
    feedback_scores: &BTreeMap<CellId, i32>,
) -> Vec<RetrievedCell> {
    let mut indexed = cells.into_iter().enumerate().collect::<Vec<_>>();
    indexed.sort_by_key(|(index, cell)| {
        (
            Reverse(*feedback_scores.get(&cell.cell_id).unwrap_or(&0)),
            *index,
        )
    });
    indexed.into_iter().map(|(_, cell)| cell).collect()
}

pub(crate) fn apply_feedback_bonus(score: u32, feedback_bonus: i32) -> u32 {
    if feedback_bonus >= 0 {
        score.saturating_add(feedback_bonus as u32)
    } else {
        score.saturating_sub(feedback_bonus.unsigned_abs())
    }
}

pub(crate) fn context_base_bm25_scores(
    cells: &[RetrievedCell],
    query: &str,
) -> BTreeMap<CellId, u32> {
    let query_text = extract_query_terms(query).join(" ");
    let query_terms = analyze_search_query(&query_text).weighted_terms;
    if query_terms.is_empty() || cells.is_empty() {
        return BTreeMap::new();
    }
    let docs = cells
        .iter()
        .map(|cell| (cell.cell_id, cell.metadata().weighted_lexical_terms()))
        .collect::<Vec<_>>();
    let doc_lengths = docs
        .iter()
        .map(|(_, terms)| terms.values().copied().sum::<u32>().max(1))
        .collect::<Vec<_>>();
    let avg_len_q16 = average_len_q16(&doc_lengths);
    let doc_count = docs.len();
    let mut scores = BTreeMap::new();
    for (index, (cell_id, doc)) in docs.iter().enumerate() {
        let score = query_terms
            .iter()
            .map(|(term, query_weight)| {
                let tf = *doc.get(term).unwrap_or(&0);
                let df = docs
                    .iter()
                    .filter(|(_, candidate)| candidate.contains_key(term))
                    .count();
                bm25_term_score_q16(
                    Bm25TermInput {
                        tf,
                        doc_len: doc_lengths[index],
                        avg_len_q16,
                        query_weight: *query_weight,
                        field_weight: 1,
                    },
                    Bm25CorpusStats {
                        doc_count,
                        doc_freq: df,
                    },
                    Default::default(),
                )
            })
            .sum::<u64>()
            .min(u64::from(u32::MAX)) as u32;
        scores.insert(*cell_id, score);
    }
    scores
}

fn average_len_q16(doc_lengths: &[u32]) -> u64 {
    if doc_lengths.is_empty() {
        return 65_536;
    }
    let total = doc_lengths.iter().copied().map(u64::from).sum::<u64>();
    total.saturating_mul(65_536) / doc_lengths.len() as u64
}
