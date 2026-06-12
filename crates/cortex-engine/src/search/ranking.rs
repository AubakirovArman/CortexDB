use std::cmp::Reverse;
use std::collections::BTreeMap;

use super::ScoredCandidate;

pub(super) fn ranked(scores: BTreeMap<u32, u64>, limit: usize) -> Vec<ScoredCandidate> {
    if limit == 0 {
        return Vec::new();
    }
    let mut values: Vec<_> = scores
        .into_iter()
        .map(|(cell_id, score)| ScoredCandidate { cell_id, score })
        .collect();
    if values.len() > limit {
        values.select_nth_unstable_by_key(limit, |candidate| {
            (Reverse(candidate.score), candidate.cell_id)
        });
        values.truncate(limit);
    }
    values.sort_by_key(|candidate| (Reverse(candidate.score), candidate.cell_id));
    values
}

#[cfg(test)]
mod tests {
    use super::ranked;
    use crate::search::ScoredCandidate;
    use std::collections::BTreeMap;

    #[test]
    fn ranked_returns_empty_for_zero_limit() {
        let results = ranked(BTreeMap::from([(1, 10), (2, 20)]), 0);
        assert!(results.is_empty());
    }

    #[test]
    fn ranked_keeps_deterministic_top_n_without_full_output() {
        let results = ranked(
            BTreeMap::from([(1, 10), (2, 40), (3, 40), (4, 20), (5, 30)]),
            3,
        );

        assert_eq!(
            results,
            vec![
                ScoredCandidate {
                    cell_id: 2,
                    score: 40,
                },
                ScoredCandidate {
                    cell_id: 3,
                    score: 40,
                },
                ScoredCandidate {
                    cell_id: 5,
                    score: 30,
                },
            ]
        );
    }
}
