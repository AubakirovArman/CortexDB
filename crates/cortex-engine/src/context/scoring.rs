use std::cmp::Reverse;
use std::collections::BTreeMap;

use cortex_core::CellId;

use super::ContextScoreComponent;
use crate::database::RetrievedCell;
use crate::source_trust::SourceTrust;

pub(crate) fn score_components(
    base_bm25: u32,
    source_trust: SourceTrust,
    redundancy_penalty: u32,
) -> Vec<ContextScoreComponent> {
    let source_trust_bonus = source_trust.score_bonus();
    vec![
        ContextScoreComponent {
            name: "base_bm25".to_owned(),
            value: base_bm25,
            contribution: i32::try_from(base_bm25).unwrap_or(i32::MAX),
            reason: "keyword overlap between query terms and cell body".to_owned(),
        },
        ContextScoreComponent {
            name: "source_trust_bonus".to_owned(),
            value: source_trust_bonus,
            contribution: i32::try_from(source_trust_bonus).unwrap_or(i32::MAX),
            reason: source_trust.score_reason(),
        },
        ContextScoreComponent {
            name: "redundancy_penalty".to_owned(),
            value: redundancy_penalty,
            contribution: -i32::try_from(redundancy_penalty).unwrap_or(i32::MAX),
            reason: "weighted Jaccard overlap with already packed cells".to_owned(),
        },
    ]
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
