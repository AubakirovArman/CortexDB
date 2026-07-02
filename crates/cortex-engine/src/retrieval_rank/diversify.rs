//! A5: deterministic Maximal Marginal Relevance (MMR) diversification.
//!
//! Reorders already-ranked candidates to trade relevance against redundancy so a
//! context pack does not spend its token budget on near-duplicate cells. All
//! arithmetic is integer Q16 — no floats, no wall clock (INV-3) — so a rebuild
//! is byte-identical, and every tie breaks to the lower original index.

use crate::database::RetrievedCell;
use crate::search::vector::vector_from_payload;

/// Q16 unit (1.0). Matches the `min_max_normalize_q16` range used across ranking.
const Q16_ONE: u64 = u16::MAX as u64;

/// Returns candidate indices reordered by MMR:
/// - the first pick is the highest-relevance candidate;
/// - each subsequent pick maximizes
///   `λ·relevance(i) + (1−λ)·(1 − max_sim(i, selected))`.
///
/// `lambda_q16` is Q16 in `[0, Q16_ONE]`: `λ = Q16_ONE` reproduces the pure
/// relevance order (diversity off); smaller `λ` favors diversity. Candidates
/// without a vector contribute zero similarity (treated as maximally diverse).
fn mmr_diversified_order(
    relevance_q16: &[u64],
    vectors: &[Option<&[i16]>],
    lambda_q16: u64,
) -> Vec<usize> {
    let n = relevance_q16.len();
    if n <= 1 {
        return (0..n).collect();
    }
    debug_assert_eq!(n, vectors.len());
    let lambda = lambda_q16.min(Q16_ONE);
    let sim = normalized_pairwise_sim_q16(vectors);

    let mut selected: Vec<usize> = Vec::with_capacity(n);
    let mut remaining = vec![true; n];
    let mut max_sim_to_selected = vec![0u64; n];

    for _ in 0..n {
        let mut best: Option<(u64, usize)> = None;
        for i in 0..n {
            if !remaining[i] {
                continue;
            }
            let marginal = if selected.is_empty() {
                relevance_q16[i].min(Q16_ONE)
            } else {
                let diversity = Q16_ONE - max_sim_to_selected[i].min(Q16_ONE);
                (lambda * relevance_q16[i].min(Q16_ONE) + (Q16_ONE - lambda) * diversity) / Q16_ONE
            };
            // Strictly-greater keeps the lower index on ties (deterministic).
            if best.is_none_or(|(best_marginal, _)| marginal > best_marginal) {
                best = Some((marginal, i));
            }
        }
        let pick = best.expect("at least one candidate remains").1;
        remaining[pick] = false;
        selected.push(pick);
        for (i, keep) in remaining.iter().enumerate() {
            if *keep {
                let pair = sim[pick * n + i];
                if pair > max_sim_to_selected[i] {
                    max_sim_to_selected[i] = pair;
                }
            }
        }
    }
    selected
}

/// Applies MMR diversification to already-ranked retrieved cells, preserving the
/// first (most relevant) cell. Vectors are read from each cell's payload; cells
/// without a vector keep their relative order among themselves. `lambda_q16`
/// equal to `Q16_ONE` is a no-op (returns the input order).
pub(crate) fn diversify_retrieved_cells(
    cells: Vec<RetrievedCell>,
    lambda_q16: u64,
) -> Vec<RetrievedCell> {
    if cells.len() <= 1 || lambda_q16 >= Q16_ONE {
        return cells;
    }
    // Input is already ranked, so use rank-derived relevance: rank 0 is best.
    // Map position -> descending Q16 relevance so the ordering is preserved when
    // diversity is off and only redundancy breaks ties otherwise.
    let n = cells.len();
    let relevance: Vec<u64> = (0..n)
        .map(|index| ((n - index) as u64 * Q16_ONE) / n as u64)
        .collect();
    let owned_vectors: Vec<Option<Vec<i16>>> = cells
        .iter()
        .map(|cell| vector_from_payload(&cell.payload))
        .collect();
    let vectors: Vec<Option<&[i16]>> = owned_vectors
        .iter()
        .map(|vector| vector.as_deref())
        .collect();
    let order = mmr_diversified_order(&relevance, &vectors, lambda_q16);

    let mut slots: Vec<Option<RetrievedCell>> = cells.into_iter().map(Some).collect();
    order
        .into_iter()
        .map(|index| slots[index].take().expect("each index selected once"))
        .collect()
}

fn normalized_pairwise_sim_q16(vectors: &[Option<&[i16]>]) -> Vec<u64> {
    let n = vectors.len();
    let mut raw = vec![0i64; n * n];
    let mut present = vec![false; n * n];
    let mut min = i64::MAX;
    let mut max = i64::MIN;
    for i in 0..n {
        for j in (i + 1)..n {
            if let (Some(a), Some(b)) = (vectors[i], vectors[j]) {
                if a.len() != b.len() {
                    continue;
                }
                let dot = a
                    .iter()
                    .zip(b)
                    .map(|(&x, &y)| i64::from(x) * i64::from(y))
                    .sum::<i64>();
                raw[i * n + j] = dot;
                raw[j * n + i] = dot;
                present[i * n + j] = true;
                present[j * n + i] = true;
                min = min.min(dot);
                max = max.max(dot);
            }
        }
    }
    let mut sim = vec![0u64; n * n];
    if max > min {
        let span = i128::from(max - min);
        for (k, present) in present.iter().enumerate() {
            if *present {
                sim[k] = ((i128::from(raw[k] - min) * i128::from(Q16_ONE)) / span) as u64;
            }
        }
    } else {
        // All present sims equal -> treat present pairs as fully similar.
        for (k, present) in present.iter().enumerate() {
            if *present {
                sim[k] = Q16_ONE;
            }
        }
    }
    sim
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lambda_one_is_pure_relevance_order() {
        let relevance = vec![30_000, 65_000, 10_000];
        let vectors = vec![None, None, None];
        // λ = Q16_ONE -> diversity off -> order by relevance desc.
        let order = mmr_diversified_order(&relevance, &vectors, Q16_ONE);
        assert_eq!(order, vec![1, 0, 2]);
    }

    #[test]
    fn diversity_demotes_a_near_duplicate() {
        // Cells 0 and 1 are near-identical; 2 is orthogonal. High relevance on 0
        // and 1, lower on 2. With diversity on, 2 should be promoted above 1.
        let relevance = vec![65_000, 60_000, 40_000];
        let v0 = [100i16, 0];
        let v1 = [99i16, 1];
        let v2 = [0i16, 100];
        let vectors = vec![Some(&v0[..]), Some(&v1[..]), Some(&v2[..])];
        let order = mmr_diversified_order(&relevance, &vectors, 20_000);
        assert_eq!(order[0], 0, "most relevant stays first");
        let pos = |id: usize| order.iter().position(|&x| x == id).unwrap();
        assert!(
            pos(2) < pos(1),
            "orthogonal cell 2 must outrank the near-duplicate cell 1: {order:?}"
        );
    }

    #[test]
    fn deterministic_and_total() {
        let relevance = vec![50_000, 50_000, 50_000, 50_000];
        let v = [[1i16, 2], [2, 1], [3, 3], [4, 0]];
        let vectors = v.iter().map(|x| Some(&x[..])).collect::<Vec<_>>();
        let a = mmr_diversified_order(&relevance, &vectors, 30_000);
        let b = mmr_diversified_order(&relevance, &vectors, 30_000);
        assert_eq!(a, b, "must be deterministic");
        let mut sorted = a.clone();
        sorted.sort_unstable();
        assert_eq!(
            sorted,
            vec![0, 1, 2, 3],
            "must be a permutation of all inputs"
        );
    }

    #[test]
    fn handles_missing_vectors() {
        let relevance = vec![40_000, 30_000];
        let vectors = vec![None, None];
        let order = mmr_diversified_order(&relevance, &vectors, 10_000);
        assert_eq!(order, vec![0, 1]);
    }

    fn cell(id: u64, vector: &str, body: &str) -> RetrievedCell {
        RetrievedCell::from_payload(
            cortex_core::CellId(id),
            format!("scope=x\nstatus=ready\nvector={vector}\n\n{body}").into_bytes(),
        )
    }

    #[test]
    fn diversify_retrieved_cells_demotes_near_duplicate() {
        // Input is already ranked [1, 2, 3]; cells 1 and 2 are near-identical,
        // cell 3 is orthogonal. Diversification must keep 1 first and lift 3
        // above the near-duplicate 2.
        let cells = vec![
            cell(1, "100,0", "alpha"),
            cell(2, "99,1", "alpha near duplicate"),
            cell(3, "0,100", "beta orthogonal"),
        ];
        let out = diversify_retrieved_cells(cells, 20_000);
        assert_eq!(out[0].cell_id.0, 1, "top-relevance cell stays first");
        let pos = |id: u64| out.iter().position(|c| c.cell_id.0 == id).unwrap();
        assert!(
            pos(3) < pos(2),
            "orthogonal cell 3 must outrank near-duplicate cell 2"
        );
    }

    #[test]
    fn diversify_off_preserves_order() {
        let cells: Vec<RetrievedCell> = (1..=3)
            .map(|id| cell(id, &format!("1,{id}"), &format!("body {id}")))
            .collect();
        let before: Vec<u64> = cells.iter().map(|c| c.cell_id.0).collect();
        // lambda = Q16_ONE -> diversity off -> identity.
        let after = diversify_retrieved_cells(cells, Q16_ONE);
        assert_eq!(
            after.iter().map(|c| c.cell_id.0).collect::<Vec<_>>(),
            before
        );
    }
}
