//! A7.2: deterministic two-stage retrieve — an exact dense rerank of a ranked
//! candidate pool.
//!
//! Stage 1 (candidate generation + ranking) produces a relevance-ordered pool;
//! this second stage reorders that pool by exact dense similarity to the query
//! vector, blended with the stage-1 order by a Q16 weight. Unlike the hybrid
//! candidate-generation path (which fuses two ranked lists by reciprocal rank),
//! this runs on the *final ranked cells* regardless of how stage 1 produced them
//! — so even a pure lexical-first retrieval becomes two-stage when a query vector
//! is present.
//!
//! All arithmetic is integer Q16 (no floats, no wall clock, INV-3), so a rebuild
//! is byte-identical and every tie breaks to the lower stage-1 index. It is off
//! by default: with no configured weight, a weight of zero, a query without a
//! vector, or a pool whose cells carry no matching vector, it is a no-op, so
//! existing retrieval output and its goldens are unchanged.

use super::{min_max_normalize_q16, query_vector_from_task, semantic_dot_score};
use crate::database::RetrievedCell;

/// Q16 unit (1.0). Matches the `min_max_normalize_q16` range used across ranking.
const Q16_ONE: u64 = u16::MAX as u64;

/// Reorders already-ranked cells by blending the stage-1 rank with an exact dense
/// score against the query vector. `rerank_weight_q16` is Q16 in `[0, Q16_ONE]`:
/// `0` keeps the stage-1 order (rerank off); `Q16_ONE` orders purely by dense
/// similarity. When the task has no query vector, fewer than two cells are
/// present, or no cell carries a matching vector, the input order is returned
/// unchanged.
pub(crate) fn two_stage_rerank(
    cells: Vec<RetrievedCell>,
    task: &str,
    rerank_weight_q16: u64,
) -> Vec<RetrievedCell> {
    let weight = rerank_weight_q16.min(Q16_ONE);
    if cells.len() <= 1 || weight == 0 {
        return cells;
    }
    let Some(query_vector) = query_vector_from_task(task) else {
        return cells;
    };
    let dense_raw = cells
        .iter()
        .map(|cell| semantic_dot_score(&cell.payload, &query_vector))
        .collect::<Vec<_>>();
    // No cell carries a vector matching the query dimension -> nothing to rerank
    // on -> keep the stage-1 order rather than collapse everything to a tie.
    if dense_raw.iter().all(|score| *score == 0) {
        return cells;
    }
    let dense = min_max_normalize_q16(&dense_raw);
    let order = blended_order(cells.len(), &dense, weight);

    let mut slots: Vec<Option<RetrievedCell>> = cells.into_iter().map(Some).collect();
    order
        .into_iter()
        .map(|index| slots[index].take().expect("each index selected once"))
        .collect()
}

/// Returns stage-1 indices reordered by `(Q16_ONE - w)·stage1 + w·dense`, where
/// the stage-1 relevance is derived from the incoming rank position (position 0
/// is best, mapped to a descending Q16 score exactly as MMR diversification
/// does). Ties break to the lower original index, so the result is always a
/// deterministic permutation of `0..n`.
fn blended_order(n: usize, dense_q16: &[u64], weight_q16: u64) -> Vec<usize> {
    debug_assert_eq!(n, dense_q16.len());
    let mut scored = (0..n)
        .map(|index| {
            let stage1 = ((n - index) as u64 * Q16_ONE) / n as u64;
            let blended = ((Q16_ONE - weight_q16) * stage1
                + weight_q16 * dense_q16[index].min(Q16_ONE))
                / Q16_ONE;
            (blended, index)
        })
        .collect::<Vec<_>>();
    scored.sort_by_key(|(blended, index)| (std::cmp::Reverse(*blended), *index));
    scored.into_iter().map(|(_, index)| index).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(id: u64, vector: &str, body: &str) -> RetrievedCell {
        RetrievedCell::from_payload(
            cortex_core::CellId(id),
            format!("scope=x\nstatus=ready\nvector={vector}\n\n{body}").into_bytes(),
        )
    }

    #[test]
    fn weight_zero_keeps_stage1_order() {
        let order = blended_order(3, &[0, 65_535, 0], 0);
        assert_eq!(order, vec![0, 1, 2], "w=0 -> stage-1 order untouched");
    }

    #[test]
    fn full_weight_orders_by_dense() {
        // Pure dense: cell 2 has the highest dense score, cell 0 the lowest.
        let order = blended_order(3, &[0, 30_000, 65_535], Q16_ONE);
        assert_eq!(order, vec![2, 1, 0]);
    }

    #[test]
    fn full_weight_ties_break_to_lower_index() {
        // Equal dense scores must preserve the stage-1 (index) order.
        let order = blended_order(3, &[65_535, 65_535, 65_535], Q16_ONE);
        assert_eq!(order, vec![0, 1, 2]);
    }

    #[test]
    fn rerank_off_preserves_order() {
        let cells = vec![
            cell(1, "10,0", "alpha"),
            cell(2, "0,10", "beta"),
            cell(3, "0,10", "gamma"),
        ];
        let before: Vec<u64> = cells.iter().map(|c| c.cell_id.0).collect();
        // weight 0 -> off -> identity even though a query vector is present.
        let after = two_stage_rerank(cells, "find\nvector=0,10\n", 0);
        assert_eq!(after.iter().map(|c| c.cell_id.0).collect::<Vec<_>>(), before);
    }

    #[test]
    fn missing_query_vector_preserves_order() {
        let cells = vec![cell(1, "10,0", "alpha"), cell(2, "0,10", "beta")];
        let before: Vec<u64> = cells.iter().map(|c| c.cell_id.0).collect();
        // No `vector=` line in the task -> no-op even at full weight.
        let after = two_stage_rerank(cells, "just a text query", Q16_ONE);
        assert_eq!(after.iter().map(|c| c.cell_id.0).collect::<Vec<_>>(), before);
    }

    #[test]
    fn dense_rerank_promotes_the_semantic_match() {
        // Stage-1 order is [1, 2, 3], but only cell 3 matches the query vector.
        // At full weight the exact dense match must rise to the top.
        let cells = vec![
            cell(1, "1,0", "lexical hit, semantically far"),
            cell(2, "2,0", "lexical hit, semantically far"),
            cell(3, "0,9", "the semantically closest cell"),
        ];
        let out = two_stage_rerank(cells, "q\nvector=0,10\n", Q16_ONE);
        assert_eq!(out[0].cell_id.0, 3, "semantic match reranked to the top");
    }

    #[test]
    fn no_matching_vector_preserves_order() {
        // Cells carry a 2-dim vector but the query is 3-dim: dot scores are all
        // zero, so the rerank must keep the stage-1 order rather than tie-collapse.
        let cells = vec![cell(1, "10,0", "alpha"), cell(2, "0,10", "beta")];
        let before: Vec<u64> = cells.iter().map(|c| c.cell_id.0).collect();
        let after = two_stage_rerank(cells, "q\nvector=1,1,1\n", Q16_ONE);
        assert_eq!(after.iter().map(|c| c.cell_id.0).collect::<Vec<_>>(), before);
    }

    #[test]
    fn deterministic_and_total() {
        let cells: Vec<RetrievedCell> = (1..=4)
            .map(|id| cell(id, &format!("{id},1"), &format!("body {id}")))
            .collect();
        let a = two_stage_rerank(cells.clone(), "q\nvector=2,2\n", 40_000);
        let b = two_stage_rerank(cells, "q\nvector=2,2\n", 40_000);
        let ids_a: Vec<u64> = a.iter().map(|c| c.cell_id.0).collect();
        let ids_b: Vec<u64> = b.iter().map(|c| c.cell_id.0).collect();
        assert_eq!(ids_a, ids_b, "must be deterministic");
        let mut sorted = ids_a.clone();
        sorted.sort_unstable();
        assert_eq!(sorted, vec![1, 2, 3, 4], "must be a permutation of all inputs");
    }
}
