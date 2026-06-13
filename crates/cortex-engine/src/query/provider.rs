use std::collections::BTreeSet;

use cortex_aql::{AgentView, BitmapHandle, BitmapProvider};
use cortex_core::CellId;

use super::metadata::scope_handle;
use super::EngineAqlIndex;
use crate::database::CandidateResolver;
use crate::search::analyze_search_query;

#[derive(Clone, Debug)]
pub struct EngineAqlProvider {
    index: EngineAqlIndex,
    agent_allowed: BTreeSet<u32>,
}

impl EngineAqlProvider {
    pub fn new(index: EngineAqlIndex, view: &AgentView) -> Self {
        let mut agent_allowed = BTreeSet::new();
        for scope in &view.readable_scopes {
            if let Some(candidates) = index.bitmaps.get(&scope_handle(*scope)) {
                agent_allowed.extend(candidates.iter().copied());
            }
        }
        agent_allowed.retain(|candidate| index.universe.contains(candidate));
        Self {
            index,
            agent_allowed,
        }
    }

    pub fn new_with_allowed_candidates(
        index: EngineAqlIndex,
        view: &AgentView,
        allowed_candidates: &BTreeSet<u32>,
    ) -> Self {
        let mut provider = Self::new(index, view);
        provider
            .agent_allowed
            .retain(|candidate| allowed_candidates.contains(candidate));
        provider
    }
}

impl BitmapProvider for EngineAqlProvider {
    fn bitmap(&self, handle: BitmapHandle) -> Option<BTreeSet<u32>> {
        self.index.bitmap(handle)
    }

    fn agent_allowed(&self) -> BTreeSet<u32> {
        self.agent_allowed.clone()
    }

    fn live(&self) -> BTreeSet<u32> {
        self.index.live()
    }

    fn universe(&self) -> BTreeSet<u32> {
        self.index.universe()
    }
}

impl CandidateResolver for EngineAqlProvider {
    fn cell_id_for_candidate(&self, candidate: u32) -> Option<CellId> {
        self.index.cell_id_for_candidate(candidate)
    }

    fn lexical_candidates_for_terms(&self, terms: &[String]) -> Option<BTreeSet<u32>> {
        let mut matched = false;
        let mut candidates = BTreeSet::new();
        for term in terms {
            if let Some(term_candidates) = self.index.lexical.get(term) {
                matched = true;
                candidates.extend(term_candidates.iter().copied());
            }
        }
        matched.then_some(candidates)
    }

    fn ranked_candidates_for_task(&self, task: &str, candidates: &[u32]) -> Option<Vec<u32>> {
        let query_terms = analyze_search_query(task).weighted_terms;
        if query_terms.is_empty() || candidates.len() <= 1 {
            return None;
        }

        let candidate_set = candidates.iter().copied().collect::<BTreeSet<_>>();
        let avg_len_q10 = average_doc_len_q10(&self.index, candidates);
        let doc_count = candidates.len() as u64;
        let mut any_score = false;
        let mut ranked = candidates
            .iter()
            .copied()
            .enumerate()
            .map(|(index, candidate)| {
                let score = query_terms
                    .iter()
                    .map(|(term, query_weight)| {
                        candidate_term_score(
                            &self.index,
                            &candidate_set,
                            doc_count,
                            avg_len_q10,
                            candidate,
                            term,
                            *query_weight,
                        )
                    })
                    .sum::<u64>();
                any_score |= score > 0;
                (score, index, candidate)
            })
            .collect::<Vec<_>>();
        if !any_score {
            return None;
        }

        ranked.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
        Some(
            ranked
                .into_iter()
                .map(|(_, _, candidate)| candidate)
                .collect(),
        )
    }
}

fn average_doc_len_q10(index: &EngineAqlIndex, candidates: &[u32]) -> u64 {
    let mut total = 0u64;
    let mut count = 0u64;
    for candidate in candidates {
        if let Some(length) = index.lexical_doc_lengths.get(candidate) {
            total = total.saturating_add(u64::from((*length).max(1)));
            count += 1;
        }
    }
    total
        .saturating_mul(1024)
        .checked_div(count)
        .unwrap_or(1024)
}

fn candidate_term_score(
    index: &EngineAqlIndex,
    candidate_set: &BTreeSet<u32>,
    doc_count: u64,
    avg_len_q10: u64,
    candidate: u32,
    term: &str,
    query_weight: u32,
) -> u64 {
    let Some(frequencies) = index.lexical_term_frequencies.get(term) else {
        return 0;
    };
    let tf = u64::from(frequencies.get(&candidate).copied().unwrap_or_default());
    if tf == 0 {
        return 0;
    }
    let df = frequencies
        .keys()
        .filter(|candidate| candidate_set.contains(candidate))
        .count() as u64;
    let doc_len_q10 = u64::from(
        index
            .lexical_doc_lengths
            .get(&candidate)
            .copied()
            .unwrap_or(1)
            .max(1),
    )
    .saturating_mul(1024);
    let norm_q10 = 256 + (768 * doc_len_q10 / avg_len_q10.max(1));
    let idf_q10 = ((doc_count + 1) * 1024) / (df + 1);
    let tf_norm_q10 = (tf * 2048 * 1024) / (tf * 1024 + norm_q10).max(1);
    idf_q10
        .saturating_mul(tf_norm_q10)
        .saturating_mul(u64::from(query_weight))
}
