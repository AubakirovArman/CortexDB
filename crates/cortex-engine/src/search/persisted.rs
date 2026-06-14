use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use crate::query::metadata::lexical_field_weight;

mod scoring;

use super::{
    bm25_idf_q16, bm25_term_score_with_idf_q16, hnsw::DistanceMetric, query_understanding, ranked,
    Bm25TermInput, ScoredCandidate,
};
use scoring::{average_field_len_q16, average_len_q16};

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
    let avg_len_q16 = average_len_q16(index.doc_lengths, allowed);
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
        let term = term_stats.term.as_str();
        let Some(posting) = index.terms.get(term) else {
            continue;
        };
        let term_started = Instant::now();
        trace_persisted_lexical(&format!(
            "score term={} visible_count={}",
            term, term_stats.visible_count
        ));
        let idf_q16 = bm25_idf_q16(doc_count, term_stats.visible_count);
        let filter = AllowedFilter {
            candidates: allowed,
            all_allowed,
        };
        if all_allowed {
            for candidate in posting {
                add_lexical_score(
                    &mut scores,
                    &index,
                    &term_stats,
                    *candidate,
                    idf_q16,
                    avg_len_q16,
                    filter,
                );
            }
        } else {
            for candidate in posting.iter().filter(|id| allowed.contains(id)) {
                add_lexical_score(
                    &mut scores,
                    &index,
                    &term_stats,
                    *candidate,
                    idf_q16,
                    avg_len_q16,
                    filter,
                );
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
    query_weight: u32,
}

#[derive(Clone, Copy)]
struct AllowedFilter<'a> {
    candidates: &'a BTreeSet<u32>,
    all_allowed: bool,
}

fn selected_query_terms(
    terms: &BTreeMap<String, BTreeSet<u32>>,
    query: &str,
    allowed: &BTreeSet<u32>,
    all_allowed: bool,
) -> Vec<QueryTermStats> {
    let mut seen = BTreeSet::new();
    let mut selected = query_understanding::analyze_search_query(query)
        .weighted_terms
        .into_iter()
        .filter(|(term, _)| seen.insert(term.clone()))
        .filter_map(|(term, query_weight)| {
            let posting = terms.get(&term)?;
            let visible_count = visible_posting_count(posting, allowed, all_allowed);
            (visible_count > 0).then_some(QueryTermStats {
                term,
                visible_count,
                query_weight,
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
    term_stats: &QueryTermStats,
    candidate: u32,
    idf_q16: u64,
    avg_len_q16: u64,
    filter: AllowedFilter<'_>,
) {
    let term = term_stats.term.as_str();
    let field_score = field_score_q16(
        index.field_doc_lengths,
        index.field_term_frequencies,
        term,
        candidate,
        idf_q16,
        term_stats.query_weight,
        filter,
    );
    let score = if field_score > 0 {
        field_score
    } else {
        bm25_term_score_with_idf_q16(
            Bm25TermInput {
                tf: term_frequency(index.term_frequencies, term, candidate),
                doc_len: *index.doc_lengths.get(&candidate).unwrap_or(&1),
                avg_len_q16,
                query_weight: term_stats.query_weight,
                field_weight: 1,
            },
            idf_q16,
            Default::default(),
        )
    };
    *scores.entry(candidate).or_default() += score;
}

fn trace_persisted_lexical(message: &str) {
    if std::env::var_os("CORTEXDB_SEARCH_TRACE").is_some() {
        eprintln!("[cortexdb-persisted-lexical-trace] {message}");
    }
}

fn field_score_q16(
    field_doc_lengths: &BTreeMap<String, BTreeMap<u32, u32>>,
    field_term_frequencies: &BTreeMap<String, BTreeMap<String, BTreeMap<u32, u32>>>,
    term: &str,
    candidate: u32,
    idf_q16: u64,
    query_weight: u32,
    filter: AllowedFilter<'_>,
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
            let length = field_doc_lengths
                .get(field)
                .and_then(|lengths| lengths.get(&candidate))
                .copied()
                .unwrap_or(1);
            let avg_len_q16 = average_field_len_q16(
                field_doc_lengths,
                field,
                filter.candidates,
                filter.all_allowed,
            );
            bm25_term_score_with_idf_q16(
                Bm25TermInput {
                    tf,
                    doc_len: length,
                    avg_len_q16,
                    query_weight,
                    field_weight: lexical_field_weight(field),
                },
                idf_q16,
                Default::default(),
            )
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

#[cfg(test)]
mod tests;
