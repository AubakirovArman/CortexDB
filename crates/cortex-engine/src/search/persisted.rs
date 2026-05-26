use std::collections::{BTreeMap, BTreeSet};

use super::{dot_nonnegative, ranked, tokenize, ScoredCandidate};

pub(super) fn search_persisted_lexical(
    terms: &BTreeMap<String, BTreeSet<u32>>,
    doc_lengths: &BTreeMap<u32, u32>,
    term_frequencies: &BTreeMap<String, BTreeMap<u32, u32>>,
    query: &str,
    allowed: &BTreeSet<u32>,
    limit: usize,
) -> Vec<ScoredCandidate> {
    let doc_count = doc_count(terms, doc_lengths, allowed) as u64;
    let avg_len_q10 = average_len_q10(doc_lengths, allowed);
    let mut scores = BTreeMap::<u32, u64>::new();
    for term in tokenize(query) {
        let Some(posting) = terms.get(&term) else {
            continue;
        };
        let visible_count = posting.iter().filter(|id| allowed.contains(id)).count() as u64;
        if visible_count == 0 {
            continue;
        }
        let idf_q10 = ((doc_count + 1) * 1024) / (visible_count + 1);
        for candidate in posting.iter().filter(|id| allowed.contains(id)) {
            let tf = u64::from(term_frequency(term_frequencies, &term, *candidate));
            let len_q10 = u64::from(*doc_lengths.get(candidate).unwrap_or(&1)) * 1024;
            let norm_q10 = 256 + (768 * len_q10 / avg_len_q10.max(1));
            let denom_q10 = (tf * 1024) + norm_q10;
            let tf_norm_q10 = (tf * 2048 * 1024) / denom_q10.max(1);
            *scores.entry(*candidate).or_default() += idf_q10 * tf_norm_q10;
        }
    }
    ranked(scores, limit)
}

fn term_frequency(
    term_frequencies: &BTreeMap<String, BTreeMap<u32, u32>>,
    term: &str,
    candidate: u32,
) -> u32 {
    term_frequencies
        .get(term)
        .and_then(|values| values.get(&candidate))
        .copied()
        .unwrap_or(1)
}

pub(super) fn search_persisted_vectors(
    vectors: &BTreeMap<u32, Vec<i16>>,
    query: &[i16],
    allowed: &BTreeSet<u32>,
    limit: usize,
) -> Vec<ScoredCandidate> {
    let scores = vectors
        .iter()
        .filter(|(candidate, _)| allowed.contains(candidate))
        .filter_map(|(candidate, vector)| {
            dot_nonnegative(query, vector).map(|score| (*candidate, score))
        })
        .collect();
    ranked(scores, limit)
}

fn doc_count(
    terms: &BTreeMap<String, BTreeSet<u32>>,
    doc_lengths: &BTreeMap<u32, u32>,
    allowed: &BTreeSet<u32>,
) -> usize {
    let mut ids = doc_lengths.keys().copied().collect::<BTreeSet<_>>();
    for posting in terms.values() {
        ids.extend(posting.iter().copied());
    }
    ids.into_iter().filter(|id| allowed.contains(id)).count()
}

fn average_len_q10(doc_lengths: &BTreeMap<u32, u32>, allowed: &BTreeSet<u32>) -> u64 {
    let mut count = 0u64;
    let mut total = 0u64;
    for candidate in allowed {
        if let Some(length) = doc_lengths.get(candidate) {
            count += 1;
            total += u64::from(*length);
        }
    }
    total
        .saturating_mul(1024)
        .checked_div(count)
        .unwrap_or(1024)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persisted_lexical_search_filters_allowed_candidates() {
        let results = search_persisted_lexical(
            &BTreeMap::from([("budget".to_owned(), BTreeSet::from([1, 2]))]),
            &BTreeMap::from([(1, 3), (2, 3)]),
            &BTreeMap::new(),
            "budget",
            &BTreeSet::from([2]),
            10,
        );

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].cell_id, 2);
    }

    #[test]
    fn persisted_lexical_search_uses_term_frequencies() {
        let results = search_persisted_lexical(
            &BTreeMap::from([("budget".to_owned(), BTreeSet::from([1, 2]))]),
            &BTreeMap::from([(1, 1), (2, 3)]),
            &BTreeMap::from([("budget".to_owned(), BTreeMap::from([(1, 1), (2, 3)]))]),
            "budget",
            &BTreeSet::from([1, 2]),
            2,
        );

        assert_eq!(results[0].cell_id, 2);
    }

    #[test]
    fn persisted_vector_search_filters_allowed_candidates() {
        let results = search_persisted_vectors(
            &BTreeMap::from([(1, vec![9, 0]), (2, vec![0, 9])]),
            &[0, 2],
            &BTreeSet::from([2]),
            10,
        );

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].cell_id, 2);
    }

    #[test]
    fn persisted_vector_search_skips_dimension_mismatches() {
        let results = search_persisted_vectors(
            &BTreeMap::from([(1, vec![9]), (2, vec![0, 9])]),
            &[0, 3],
            &BTreeSet::from([1, 2]),
            10,
        );

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].cell_id, 2);
        assert_eq!(results[0].score, 27);
    }
}
