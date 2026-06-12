use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use crate::query::metadata::lexical_field_weight;

use super::{hnsw::DistanceMetric, ranked, tokenize, ScoredCandidate};

const MAX_PERSISTED_LEXICAL_QUERY_TERMS: usize = 8;

pub(super) struct PersistedLexicalSearchIndex<'a> {
    pub terms: &'a BTreeMap<String, BTreeSet<u32>>,
    pub doc_lengths: &'a BTreeMap<u32, u32>,
    pub term_frequencies: &'a BTreeMap<String, BTreeMap<u32, u32>>,
    pub field_doc_lengths: &'a BTreeMap<String, BTreeMap<u32, u32>>,
    pub field_term_frequencies: &'a BTreeMap<String, BTreeMap<String, BTreeMap<u32, u32>>>,
}

pub(super) fn search_persisted_lexical(
    index: PersistedLexicalSearchIndex<'_>,
    query: &str,
    allowed: &BTreeSet<u32>,
    limit: usize,
) -> Vec<ScoredCandidate> {
    let doc_count = doc_count(index.doc_lengths, allowed);
    if doc_count == 0 {
        return Vec::new();
    }
    let avg_len_q10 = average_len_q10(index.doc_lengths, allowed);
    let mut scores = BTreeMap::<u32, u64>::new();
    let all_allowed = !index.doc_lengths.is_empty() && allowed.len() == index.doc_lengths.len();
    let selected_terms = selected_query_terms(index.terms, query, allowed, all_allowed);
    trace_persisted_lexical(&format!(
        "query_terms selected=[{}] doc_count={} allowed={} all_allowed={}",
        selected_terms
            .iter()
            .map(|term| format!("{}:{}", term.term, term.visible_count))
            .collect::<Vec<_>>()
            .join(","),
        doc_count,
        allowed.len(),
        all_allowed
    ));
    for term_stats in selected_terms {
        let term = term_stats.term;
        let Some(posting) = index.terms.get(&term) else {
            continue;
        };
        let term_started = Instant::now();
        trace_persisted_lexical(&format!(
            "score term={} visible_count={}",
            term, term_stats.visible_count
        ));
        let idf_q10 = ((doc_count as u64 + 1) * 1024) / (term_stats.visible_count as u64 + 1);
        if all_allowed {
            for candidate in posting {
                add_lexical_score(&mut scores, &index, &term, *candidate, idf_q10, avg_len_q10);
            }
        } else {
            for candidate in posting.iter().filter(|id| allowed.contains(id)) {
                add_lexical_score(&mut scores, &index, &term, *candidate, idf_q10, avg_len_q10);
            }
        }
        trace_persisted_lexical(&format!(
            "score term={} done elapsed_ms={} accumulated_candidates={}",
            term,
            term_started.elapsed().as_millis(),
            scores.len()
        ));
    }
    ranked(scores, limit)
}

struct QueryTermStats {
    term: String,
    visible_count: usize,
}

fn selected_query_terms(
    terms: &BTreeMap<String, BTreeSet<u32>>,
    query: &str,
    allowed: &BTreeSet<u32>,
    all_allowed: bool,
) -> Vec<QueryTermStats> {
    let mut seen = BTreeSet::new();
    let mut selected = tokenize(query)
        .into_iter()
        .filter(|term| seen.insert(term.clone()))
        .filter_map(|term| {
            let posting = terms.get(&term)?;
            let visible_count = visible_posting_count(posting, allowed, all_allowed);
            (visible_count > 0).then_some(QueryTermStats {
                term,
                visible_count,
            })
        })
        .collect::<Vec<_>>();
    selected.sort_by(|left, right| {
        left.visible_count
            .cmp(&right.visible_count)
            .then_with(|| right.term.len().cmp(&left.term.len()))
            .then_with(|| left.term.cmp(&right.term))
    });
    selected.truncate(MAX_PERSISTED_LEXICAL_QUERY_TERMS);
    selected
}

fn visible_posting_count(
    posting: &BTreeSet<u32>,
    allowed: &BTreeSet<u32>,
    all_allowed: bool,
) -> usize {
    if all_allowed {
        posting.len()
    } else {
        posting.iter().filter(|id| allowed.contains(id)).count()
    }
}

fn add_lexical_score(
    scores: &mut BTreeMap<u32, u64>,
    index: &PersistedLexicalSearchIndex<'_>,
    term: &str,
    candidate: u32,
    idf_q10: u64,
    avg_len_q10: u64,
) {
    let field_score = field_score_q10(
        index.field_doc_lengths,
        index.field_term_frequencies,
        term,
        candidate,
        idf_q10,
    );
    let score = if field_score > 0 {
        field_score
    } else {
        let tf = u64::from(term_frequency(index.term_frequencies, term, candidate));
        let len_q10 = u64::from(*index.doc_lengths.get(&candidate).unwrap_or(&1)) * 1024;
        let norm_q10 = 256 + (768 * len_q10 / avg_len_q10.max(1));
        let denom_q10 = (tf * 1024) + norm_q10;
        let tf_norm_q10 = (tf * 2048 * 1024) / denom_q10.max(1);
        idf_q10 * tf_norm_q10
    };
    *scores.entry(candidate).or_default() += score;
}

fn trace_persisted_lexical(message: &str) {
    if std::env::var_os("CORTEXDB_SEARCH_TRACE").is_some() {
        eprintln!("[cortexdb-persisted-lexical-trace] {message}");
    }
}

fn field_score_q10(
    field_doc_lengths: &BTreeMap<String, BTreeMap<u32, u32>>,
    field_term_frequencies: &BTreeMap<String, BTreeMap<String, BTreeMap<u32, u32>>>,
    term: &str,
    candidate: u32,
    idf_q10: u64,
) -> u64 {
    field_term_frequencies
        .iter()
        .filter_map(|(field, terms)| {
            let tf = terms
                .get(term)
                .and_then(|values| values.get(&candidate))
                .copied()?;
            Some((field, tf))
        })
        .map(|(field, tf)| {
            let tf = u64::from(tf);
            let len_q10 = field_doc_lengths
                .get(field)
                .and_then(|lengths| lengths.get(&candidate))
                .copied()
                .map(u64::from)
                .unwrap_or(1)
                * 1024;
            let avg_len_q10 = average_field_len_q10(field_doc_lengths, field);
            let norm_q10 = 256 + (768 * len_q10 / avg_len_q10.max(1));
            let denom_q10 = (tf * 1024) + norm_q10;
            let tf_norm_q10 = (tf * 2048 * 1024) / denom_q10.max(1);
            idf_q10
                .saturating_mul(tf_norm_q10)
                .saturating_mul(u64::from(lexical_field_weight(field)))
        })
        .sum()
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
    metric: &DistanceMetric,
) -> Vec<ScoredCandidate> {
    let scores = vectors
        .iter()
        .filter(|(candidate, _)| allowed.contains(candidate))
        .filter_map(|(candidate, vector)| {
            metric
                .distance(query, vector)
                .map(|score| (*candidate, score))
        })
        .collect();
    ranked(scores, limit)
}

fn doc_count(doc_lengths: &BTreeMap<u32, u32>, allowed: &BTreeSet<u32>) -> usize {
    let count = doc_lengths.keys().filter(|id| allowed.contains(id)).count();
    if count > 0 || allowed.is_empty() {
        count
    } else {
        allowed.len()
    }
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

fn average_field_len_q10(
    field_doc_lengths: &BTreeMap<String, BTreeMap<u32, u32>>,
    field: &str,
) -> u64 {
    let Some(lengths) = field_doc_lengths.get(field) else {
        return 1024;
    };
    let total = lengths.values().copied().map(u64::from).sum::<u64>();
    if lengths.is_empty() {
        1024
    } else {
        total * 1024 / lengths.len() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persisted_lexical_search_filters_allowed_candidates() {
        let terms = BTreeMap::from([("budget".to_owned(), BTreeSet::from([1, 2]))]);
        let doc_lengths = BTreeMap::from([(1, 3), (2, 3)]);
        let term_frequencies = BTreeMap::new();
        let field_doc_lengths = BTreeMap::new();
        let field_term_frequencies = BTreeMap::new();
        let results = search_persisted_lexical(
            PersistedLexicalSearchIndex {
                terms: &terms,
                doc_lengths: &doc_lengths,
                term_frequencies: &term_frequencies,
                field_doc_lengths: &field_doc_lengths,
                field_term_frequencies: &field_term_frequencies,
            },
            "budget",
            &BTreeSet::from([2]),
            10,
        );

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].cell_id, 2);
    }

    #[test]
    fn persisted_lexical_search_uses_term_frequencies() {
        let terms = BTreeMap::from([("budget".to_owned(), BTreeSet::from([1, 2]))]);
        let doc_lengths = BTreeMap::from([(1, 1), (2, 3)]);
        let term_frequencies =
            BTreeMap::from([("budget".to_owned(), BTreeMap::from([(1, 1), (2, 3)]))]);
        let field_doc_lengths = BTreeMap::new();
        let field_term_frequencies = BTreeMap::new();
        let results = search_persisted_lexical(
            PersistedLexicalSearchIndex {
                terms: &terms,
                doc_lengths: &doc_lengths,
                term_frequencies: &term_frequencies,
                field_doc_lengths: &field_doc_lengths,
                field_term_frequencies: &field_term_frequencies,
            },
            "budget",
            &BTreeSet::from([1, 2]),
            2,
        );

        assert_eq!(results[0].cell_id, 2);
    }

    #[test]
    fn persisted_lexical_search_uses_field_weights_when_available() {
        let terms = BTreeMap::from([("apollo".to_owned(), BTreeSet::from([1, 2]))]);
        let doc_lengths = BTreeMap::from([(1, 8), (2, 8)]);
        let term_frequencies =
            BTreeMap::from([("apollo".to_owned(), BTreeMap::from([(1, 1), (2, 3)]))]);
        let field_doc_lengths = BTreeMap::from([
            ("title".to_owned(), BTreeMap::from([(1, 1)])),
            ("body".to_owned(), BTreeMap::from([(2, 3)])),
        ]);
        let field_term_frequencies = BTreeMap::from([
            (
                "title".to_owned(),
                BTreeMap::from([("apollo".to_owned(), BTreeMap::from([(1, 1)]))]),
            ),
            (
                "body".to_owned(),
                BTreeMap::from([("apollo".to_owned(), BTreeMap::from([(2, 3)]))]),
            ),
        ]);
        let results = search_persisted_lexical(
            PersistedLexicalSearchIndex {
                terms: &terms,
                doc_lengths: &doc_lengths,
                term_frequencies: &term_frequencies,
                field_doc_lengths: &field_doc_lengths,
                field_term_frequencies: &field_term_frequencies,
            },
            "apollo",
            &BTreeSet::from([1, 2]),
            2,
        );

        assert_eq!(results[0].cell_id, 1);
    }

    #[test]
    fn persisted_doc_count_uses_allowed_doc_lengths() {
        let doc_lengths = BTreeMap::from([(1, 8), (2, 9)]);
        let allowed = BTreeSet::from([1, 2, 99]);

        assert_eq!(doc_count(&doc_lengths, &allowed), 2);
    }

    #[test]
    fn persisted_lexical_search_filters_allowed_even_with_extra_allowed_ids() {
        let terms = BTreeMap::from([("budget".to_owned(), BTreeSet::from([1, 2]))]);
        let doc_lengths = BTreeMap::from([(1, 3)]);
        let term_frequencies = BTreeMap::new();
        let field_doc_lengths = BTreeMap::new();
        let field_term_frequencies = BTreeMap::new();
        let results = search_persisted_lexical(
            PersistedLexicalSearchIndex {
                terms: &terms,
                doc_lengths: &doc_lengths,
                term_frequencies: &term_frequencies,
                field_doc_lengths: &field_doc_lengths,
                field_term_frequencies: &field_term_frequencies,
            },
            "budget",
            &BTreeSet::from([1, 99]),
            10,
        );

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].cell_id, 1);
    }

    #[test]
    fn persisted_query_term_selection_prefers_rare_terms() {
        let terms = BTreeMap::from([
            ("common".to_owned(), BTreeSet::from([1, 2, 3, 4])),
            ("rare".to_owned(), BTreeSet::from([4])),
        ]);
        let allowed = BTreeSet::from([1, 2, 3, 4]);
        let selected = selected_query_terms(&terms, "common rare common", &allowed, true);

        assert_eq!(selected.len(), 2);
        assert_eq!(selected[0].term, "rare");
        assert_eq!(selected[1].term, "common");
    }

    #[test]
    fn persisted_vector_search_filters_allowed_candidates() {
        let results = search_persisted_vectors(
            &BTreeMap::from([(1, vec![9, 0]), (2, vec![0, 9])]),
            &[0, 2],
            &BTreeSet::from([2]),
            10,
            &DistanceMetric::default(),
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
            &DistanceMetric::default(),
        );

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].cell_id, 2);
        assert_eq!(results[0].score, 27);
    }
}
