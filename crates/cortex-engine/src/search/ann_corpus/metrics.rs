use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct RankingMetrics {
    pub reciprocal_rank_q16: u16,
    pub ndcg_q16: u16,
    pub exact_parity: bool,
}

pub(super) fn ranking_metrics_q16(results: &[u32], truth: &[u32], limit: usize) -> RankingMetrics {
    let denominator = truth.len().min(limit);
    if denominator == 0 {
        return RankingMetrics {
            reciprocal_rank_q16: 65_535,
            ndcg_q16: 65_535,
            exact_parity: true,
        };
    }

    let truth_prefix = truth.iter().take(limit).copied().collect::<Vec<_>>();
    let truth_set = truth_prefix.iter().copied().collect::<BTreeSet<_>>();
    let reciprocal_rank_q16 = results
        .iter()
        .take(limit)
        .position(|candidate| truth_set.contains(candidate))
        .map(|index| (65_535u64 / (index as u64 + 1)) as u16)
        .unwrap_or(0);

    let dcg = discounted_gain(results.iter().take(limit).copied(), &truth_set);
    let ideal = discounted_gain(truth_prefix.iter().copied(), &truth_set);
    let ndcg_q16 = if ideal == 0.0 {
        65_535
    } else {
        ((dcg / ideal * 65_535.0).round().clamp(0.0, 65_535.0)) as u16
    };

    RankingMetrics {
        reciprocal_rank_q16,
        ndcg_q16,
        exact_parity: results.iter().take(limit).copied().eq(truth_prefix),
    }
}

fn discounted_gain(candidates: impl Iterator<Item = u32>, truth: &BTreeSet<u32>) -> f64 {
    candidates
        .enumerate()
        .filter(|(_, candidate)| truth.contains(candidate))
        .map(|(index, _)| 1.0 / ((index + 2) as f64).log2())
        .sum()
}

#[cfg(test)]
mod tests {
    use super::ranking_metrics_q16;

    #[test]
    fn perfect_ranking_gets_full_scores() {
        let metrics = ranking_metrics_q16(&[10, 20, 30], &[10, 20, 30], 3);
        assert_eq!(metrics.reciprocal_rank_q16, 65_535);
        assert_eq!(metrics.ndcg_q16, 65_535);
        assert!(metrics.exact_parity);
    }

    #[test]
    fn delayed_first_hit_lowers_mrr_and_ndcg() {
        let metrics = ranking_metrics_q16(&[99, 20, 10], &[10, 20, 30], 3);
        assert_eq!(metrics.reciprocal_rank_q16, 32_767);
        assert!(metrics.ndcg_q16 < 65_535);
        assert!(!metrics.exact_parity);
    }
}
