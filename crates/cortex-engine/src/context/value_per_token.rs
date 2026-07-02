use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet};

use cortex_core::CellId;

use super::dedup::weighted_jaccard_q16;
use super::freshness::{SourceFreshness, SourceFreshnessRange};
use super::large_cell::estimate_cell_tokens;
use super::ContextPackOptions;
use crate::context::scoring::apply_feedback_bonus;
use crate::database::RetrievedCell;
use crate::search::{frozen_weights, tokenize};
use crate::source_trust::SourceTrust;

pub(crate) fn order_by_value_per_token(
    cells: Vec<RetrievedCell>,
    query_terms: &[String],
    citations_required: bool,
    options: &ContextPackOptions,
    base_bm25_scores: &BTreeMap<CellId, u32>,
    source_freshness_range: SourceFreshnessRange,
    feedback_scores: &BTreeMap<CellId, i32>,
) -> Vec<RetrievedCell> {
    let required_terms = required_terms(query_terms);
    if cells.len() <= 1 || required_terms.is_empty() {
        return cells;
    }

    let mut remaining = cells
        .into_iter()
        .enumerate()
        .map(|(index, cell)| {
            let metadata = cell.metadata();
            let terms = metadata
                .weighted_lexical_terms()
                .into_keys()
                .collect::<BTreeSet<_>>();
            let token_cost = estimate_cell_tokens(
                &cell.payload,
                metadata.citation(),
                citations_required,
                options,
            )
            .max(1);
            CandidatePlan {
                original_index: index,
                cell,
                terms,
                token_cost,
            }
        })
        .collect::<Vec<_>>();
    let mut selected_terms = Vec::<BTreeSet<String>>::new();
    let mut covered_terms = BTreeSet::new();
    let mut ordered = Vec::with_capacity(remaining.len());

    while !remaining.is_empty() {
        let best_index = (0..remaining.len())
            .max_by_key(|candidate_index| {
                let candidate = &remaining[*candidate_index];
                let rank = value_per_token_rank(
                    candidate,
                    RankContext {
                        required_terms: &required_terms,
                        covered_terms: &covered_terms,
                        selected_terms: &selected_terms,
                        base_bm25_scores,
                        source_freshness_range,
                        feedback_scores,
                        citations_required,
                    },
                );
                (rank, Reverse(candidate.original_index))
            })
            .unwrap_or(0);
        let candidate = remaining.remove(best_index);
        for term in candidate.terms.intersection(&required_terms) {
            covered_terms.insert(term.clone());
        }
        selected_terms.push(candidate.terms);
        ordered.push(candidate.cell);
    }

    ordered
}

struct CandidatePlan {
    original_index: usize,
    cell: RetrievedCell,
    terms: BTreeSet<String>,
    token_cost: u32,
}

struct RankContext<'a> {
    required_terms: &'a BTreeSet<String>,
    covered_terms: &'a BTreeSet<String>,
    selected_terms: &'a [BTreeSet<String>],
    base_bm25_scores: &'a BTreeMap<CellId, u32>,
    source_freshness_range: SourceFreshnessRange,
    feedback_scores: &'a BTreeMap<CellId, i32>,
    citations_required: bool,
}

fn required_terms(query_terms: &[String]) -> BTreeSet<String> {
    query_terms
        .iter()
        .flat_map(|term| tokenize(term))
        .collect::<BTreeSet<_>>()
}

fn value_per_token_rank(candidate: &CandidatePlan, context: RankContext<'_>) -> i128 {
    let metadata = candidate.cell.metadata();
    let matched_terms = candidate.terms.intersection(context.required_terms).count() as i64;
    let new_terms = candidate
        .terms
        .intersection(context.required_terms)
        .filter(|term| !context.covered_terms.contains(*term))
        .count() as i64;
    let source_trust =
        SourceTrust::from_metadata(metadata.source_trust_q16, metadata.source_trust_class);
    let source_freshness = SourceFreshness::from_created_unix_seconds(
        metadata.created_unix_seconds,
        context.source_freshness_range,
    );
    let mut value = new_terms
        .saturating_mul(frozen_weights::VALUE_PER_TOKEN_COVERAGE_VALUE)
        .saturating_add(
            matched_terms.saturating_mul(frozen_weights::VALUE_PER_TOKEN_MATCHED_TERM_VALUE),
        )
        .saturating_add(i64::from(
            *context
                .base_bm25_scores
                .get(&candidate.cell.cell_id)
                .unwrap_or(&0),
        ))
        .saturating_add(i64::from(source_trust.score_bonus()))
        .saturating_add(i64::from(source_freshness.score_bonus()));
    if context.citations_required && metadata.citation().is_some() {
        value = value.saturating_add(frozen_weights::VALUE_PER_TOKEN_CITATION_VALUE);
    }
    let feedback_bonus = *context
        .feedback_scores
        .get(&candidate.cell.cell_id)
        .unwrap_or(&0);
    value = i64::from(apply_feedback_bonus(clamp_to_u32(value), feedback_bonus));

    let redundancy = context
        .selected_terms
        .iter()
        .map(|selected| weighted_jaccard_q16(&candidate.terms, selected))
        .max()
        .unwrap_or(0) as i64;
    value = value.saturating_sub(
        (redundancy.saturating_mul(frozen_weights::VALUE_PER_TOKEN_REDUNDANCY_VALUE))
            / i64::from(frozen_weights::Q16_SCALE_U32),
    );

    i128::from(value.max(0)).saturating_mul(frozen_weights::Q16_SCALE_I128)
        / i128::from(candidate.token_cost)
}

fn clamp_to_u32(value: i64) -> u32 {
    u32::try_from(value.max(0)).unwrap_or(u32::MAX)
}
