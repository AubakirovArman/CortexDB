use std::collections::BTreeSet;

use crate::context::dedup::weighted_jaccard_q16;
use crate::database::RetrievedCell;
use crate::search::tokenize;

pub(super) fn diversity_aware_order(
    cells: Vec<RetrievedCell>,
    query_terms: &[String],
) -> Vec<RetrievedCell> {
    if cells.len() <= 2 || query_terms.is_empty() {
        return cells;
    }
    let required_terms = query_terms
        .iter()
        .flat_map(|term| tokenize(term))
        .collect::<BTreeSet<_>>();
    if required_terms.is_empty() {
        return cells;
    }

    let mut remaining = cells
        .into_iter()
        .enumerate()
        .map(|(index, cell)| {
            let terms = cell
                .metadata()
                .weighted_lexical_terms()
                .into_keys()
                .collect::<BTreeSet<_>>();
            (index, cell, terms)
        })
        .collect::<Vec<_>>();
    let mut selected_terms = Vec::<BTreeSet<String>>::new();
    let mut ordered = Vec::with_capacity(remaining.len());

    while !remaining.is_empty() {
        let best = (0..remaining.len())
            .max_by_key(|candidate_index| {
                let (original_index, _, terms) = &remaining[*candidate_index];
                let relevance = term_coverage_q16(terms, &required_terms) as i64;
                let redundancy = selected_terms
                    .iter()
                    .map(|selected| weighted_jaccard_q16(terms, selected))
                    .max()
                    .unwrap_or(0) as i64;
                let mmr_score = (3 * relevance) - redundancy;
                (mmr_score, std::cmp::Reverse(*original_index))
            })
            .unwrap_or(0);
        let (_, cell, terms) = remaining.remove(best);
        selected_terms.push(terms);
        ordered.push(cell);
    }

    ordered
}

fn term_coverage_q16(terms: &BTreeSet<String>, required_terms: &BTreeSet<String>) -> u16 {
    if required_terms.is_empty() {
        return u16::MAX;
    }
    let covered = required_terms
        .iter()
        .filter(|term| terms.contains(*term))
        .count() as u64;
    ((covered * u64::from(u16::MAX)) / required_terms.len() as u64) as u16
}
