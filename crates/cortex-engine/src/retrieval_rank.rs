use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

use cortex_aql::RetrievalWeights;
use cortex_core::CellId;

use crate::database::RetrievedCell;
use crate::feedback::current_unix_seconds;
use crate::query::CellMetadata;
use crate::search::{analyze_search_query, bm25_term_score_q16, Bm25CorpusStats, Bm25TermInput};
use crate::source_trust::SourceTrust;

mod diversify;
mod memory_decay;
mod semantic;

pub(crate) use diversify::diversify_retrieved_cells;
use memory_decay::memory_decay_q16;
pub(crate) use semantic::{query_vector_from_task, semantic_dot_score};

pub(crate) fn rank_retrieved_cells(
    cells: Vec<RetrievedCell>,
    task: &str,
    weights: &RetrievalWeights,
) -> Vec<RetrievedCell> {
    rank_retrieved_cells_with_window(cells, task, weights, None)
}

pub(crate) fn rank_retrieved_cells_with_window(
    mut cells: Vec<RetrievedCell>,
    task: &str,
    weights: &RetrievalWeights,
    recency_window_seconds: Option<u64>,
) -> Vec<RetrievedCell> {
    if cells.len() <= 1 {
        return cells;
    }

    let metadata = cells
        .iter()
        .map(RetrievedCell::metadata)
        .collect::<Vec<_>>();
    let lexical_scores = lexical_bm25_scores_from_metadata(&metadata, task);
    let query_vector = query_vector_from_task(task);
    let semantic_scores = cells
        .iter()
        .map(|cell| {
            query_vector
                .as_deref()
                .map(|query| semantic_dot_score(&cell.payload, query))
                .unwrap_or(0)
        })
        .collect::<Vec<_>>();
    let recency_scores = recency_scores_q16_from_metadata(&metadata, recency_window_seconds);
    let trust_scores = metadata
        .iter()
        .map(|metadata| {
            u64::from(
                SourceTrust::from_metadata(metadata.source_trust_q16, metadata.source_trust_class)
                    .q16,
            )
        })
        .collect::<Vec<_>>();

    // Fusion normalization: the four raw component scales are incomparable
    // (BM25, a raw i16 dot product, and two Q16 signals), so blending them by
    // weight is only meaningful after each component is min-max normalized to a
    // common [0, Q16] range across the candidate set.
    let lexical = min_max_normalize_q16(&lexical_scores);
    let semantic = min_max_normalize_q16(&semantic_scores);
    let recency = min_max_normalize_q16(&recency_scores);
    let trust = min_max_normalize_q16(&trust_scores);

    let now = current_unix_seconds();
    let rank_keys = (0..cells.len())
        .map(|index| {
            let score = weighted_retrieval_score(
                lexical[index],
                semantic[index],
                recency[index],
                trust[index],
                weights,
            )
            .saturating_mul(memory_decay_q16(&metadata[index], now))
                / u64::from(u16::MAX);
            (score, index)
        })
        .collect::<Vec<_>>();
    let mut indexed = cells
        .drain(..)
        .enumerate()
        .map(|(index, cell)| (rank_keys[index], cell))
        .collect::<Vec<_>>();

    indexed.sort_by_key(|((score, index), _)| (Reverse(*score), *index));

    indexed.into_iter().map(|(_, cell)| cell).collect()
}

pub(crate) fn suppress_duplicate_content(cells: Vec<RetrievedCell>) -> Vec<RetrievedCell> {
    let mut seen = BTreeSet::<String>::new();
    cells
        .into_iter()
        .filter(|cell| {
            let metadata = cell.metadata();
            let Some(content_hash) = metadata.content_hash else {
                return true;
            };
            seen.insert(content_hash)
        })
        .collect()
}

pub(crate) fn expand_parent_context(cells: Vec<RetrievedCell>) -> Vec<RetrievedCell> {
    if cells.len() <= 1 {
        return cells;
    }

    let mut parent_key_to_index = BTreeMap::<String, usize>::new();
    let metadata = cells
        .iter()
        .map(RetrievedCell::metadata)
        .collect::<Vec<_>>();
    for (index, metadata) in metadata.iter().enumerate() {
        if let Some(chunk_id) = &metadata.chunk_id {
            parent_key_to_index.entry(chunk_id.clone()).or_insert(index);
        }
        if is_parent_context_metadata(metadata) {
            if let Some(document_id) = &metadata.document_id {
                parent_key_to_index
                    .entry(document_id.clone())
                    .or_insert(index);
            }
        }
    }

    let mut expanded = Vec::with_capacity(cells.len());
    let mut emitted = BTreeSet::<CellId>::new();
    for (index, cell) in cells.iter().enumerate() {
        if emitted.insert(cell.cell_id) {
            expanded.push(cell.clone());
        }
        let Some(parent_id) = metadata[index].parent_id.as_ref() else {
            continue;
        };
        let Some(parent_index) = parent_key_to_index.get(parent_id).copied() else {
            continue;
        };
        let parent = &cells[parent_index];
        if parent.cell_id != cell.cell_id && emitted.insert(parent.cell_id) {
            expanded.push(parent.clone());
        }
    }
    expanded
}

fn is_parent_context_metadata(metadata: &CellMetadata) -> bool {
    metadata
        .chunk_role
        .as_deref()
        .map(|role| {
            role.eq_ignore_ascii_case("parent")
                || role.eq_ignore_ascii_case("document")
                || role.eq_ignore_ascii_case("summary")
        })
        .unwrap_or(false)
}

fn lexical_bm25_scores_from_metadata(metadata: &[CellMetadata], query: &str) -> Vec<u64> {
    let query_terms = analyze_search_query(query).weighted_terms;
    if query_terms.is_empty() || metadata.is_empty() {
        return vec![0; metadata.len()];
    }

    let docs = metadata
        .iter()
        .map(CellMetadata::weighted_lexical_terms)
        .collect::<Vec<_>>();
    let doc_lengths = docs
        .iter()
        .map(|terms| terms.values().copied().sum::<u32>().max(1))
        .collect::<Vec<_>>();
    let avg_len_q16 = average_len_q16(&doc_lengths);
    let doc_count = docs.len();
    let mut doc_frequency = BTreeMap::<String, usize>::new();
    for term in query_terms.keys() {
        let count = docs.iter().filter(|doc| doc.contains_key(term)).count();
        doc_frequency.insert(term.clone(), count);
    }

    docs.iter()
        .enumerate()
        .map(|(index, doc)| {
            query_terms
                .iter()
                .map(|(term, query_weight)| {
                    let tf = *doc.get(term).unwrap_or(&0);
                    if tf == 0 {
                        return 0;
                    }
                    let df = *doc_frequency.get(term).unwrap_or(&0);
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
                .sum()
        })
        .collect()
}

fn average_len_q16(doc_lengths: &[u32]) -> u64 {
    if doc_lengths.is_empty() {
        return 65_536;
    }
    let total = doc_lengths.iter().copied().map(u64::from).sum::<u64>();
    total * 65_536 / doc_lengths.len() as u64
}

fn recency_scores_q16_from_metadata(
    metadata: &[CellMetadata],
    recency_window_seconds: Option<u64>,
) -> Vec<u64> {
    let created = metadata
        .iter()
        .map(|metadata| metadata.created_unix_seconds)
        .collect::<Vec<_>>();
    let mut timestamps = created.iter().filter_map(|value| *value);
    let Some(first) = timestamps.next() else {
        return vec![0; metadata.len()];
    };
    let (min, max) = timestamps.fold((first, first), |(min, max), value| {
        (min.min(value), max.max(value))
    });
    let relative: Vec<u64> = if max <= min {
        created
            .iter()
            .map(|value| value.map(|_| u64::from(u16::MAX)).unwrap_or(0))
            .collect()
    } else {
        let span = max - min;
        created
            .iter()
            .map(|value| {
                value
                    .map(|created| {
                        (created.saturating_sub(min).min(span) * u64::from(u16::MAX)) / span
                    })
                    .unwrap_or(0)
            })
            .collect()
    };
    // Temporal window (A4.1): cells created within `window` seconds of the
    // newest candidate are treated as fully fresh. The reference time is the
    // data-derived max created timestamp, so this stays deterministic and
    // wall-clock-free (INV-3). Off by default.
    match recency_window_seconds {
        Some(window) => apply_recency_window(&created, relative, max, window),
        None => relative,
    }
}

/// Overrides `relative` recency scores so any cell created within `window`
/// seconds of `newest` scores as fully fresh (`u16::MAX`). Pure and
/// deterministic; extracted for testing without building a full `CellMetadata`.
fn apply_recency_window(
    created: &[Option<u64>],
    relative: Vec<u64>,
    newest: u64,
    window: u64,
) -> Vec<u64> {
    let threshold = newest.saturating_sub(window);
    created
        .iter()
        .zip(relative)
        .map(|(value, base)| {
            if value.is_some_and(|created| created >= threshold) {
                u64::from(u16::MAX)
            } else {
                base
            }
        })
        .collect()
}

fn weighted_retrieval_score(
    lexical: u64,
    semantic: u64,
    recency: u64,
    trust: u64,
    weights: &RetrievalWeights,
) -> u64 {
    // Each component is already min-max normalized to [0, Q16], so the weighted
    // sum is a true weighted blend in [0, Q16]; no per-component rescaling here.
    weighted_component(lexical, weights.lexical_q16)
        .saturating_add(weighted_component(semantic, weights.semantic_q16))
        .saturating_add(weighted_component(recency, weights.recency_q16))
        .saturating_add(weighted_component(trust, weights.trust_q16))
}

fn weighted_component(value: u64, weight_q16: u16) -> u64 {
    value.saturating_mul(u64::from(weight_q16)) / u64::from(u16::MAX)
}

/// Min-max normalizes a component to `[0, u16::MAX]` across the candidate set so
/// that weighted fusion of otherwise-incomparable scales (BM25, i16 dot product,
/// Q16 signals) is meaningful. A component with no spread (all-equal) maps every
/// candidate to `u16::MAX`: it adds the same constant to every base score, so it
/// does not change relative ordering, while keeping the base score non-zero so a
/// downstream multiplier (e.g. memory decay) still differentiates candidates.
fn min_max_normalize_q16(values: &[u64]) -> Vec<u64> {
    let (min, max) = values.iter().fold((u64::MAX, 0u64), |(lo, hi), &value| {
        (lo.min(value), hi.max(value))
    });
    if max <= min {
        return vec![u64::from(u16::MAX); values.len()];
    }
    let span = u128::from(max - min);
    values
        .iter()
        .map(|&value| (u128::from(value - min) * u128::from(u16::MAX) / span) as u64)
        .collect()
}

#[cfg(test)]
mod temporal_tests {
    use super::apply_recency_window;

    #[test]
    fn recency_window_marks_recent_cells_fully_fresh() {
        // Newest is 1000; window 100 -> cells created >= 900 are fully fresh.
        let created = vec![Some(1_000), Some(950), Some(800), None];
        let relative = vec![65_535, 40_000, 10_000, 0];
        let out = apply_recency_window(&created, relative, 1_000, 100);
        assert_eq!(out[0], 65_535, "newest stays fresh");
        assert_eq!(out[1], 65_535, "within window -> boosted to fresh");
        assert_eq!(out[2], 10_000, "outside window -> keeps relative recency");
        assert_eq!(out[3], 0, "missing timestamp stays 0");
    }

    #[test]
    fn zero_window_only_boosts_the_newest() {
        let created = vec![Some(500), Some(500), Some(499)];
        let relative = vec![20_000, 20_000, 10_000];
        let out = apply_recency_window(&created, relative, 500, 0);
        assert_eq!(out, vec![65_535, 65_535, 10_000]);
    }
}
