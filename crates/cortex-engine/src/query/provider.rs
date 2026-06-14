use std::collections::BTreeSet;

use cortex_aql::{AgentView, BitmapHandle, BitmapProvider, RoaringBitmap};
use cortex_core::CellId;

use super::metadata::scope_handle;
use super::EngineAqlIndex;
use crate::database::CandidateResolver;
use crate::plan::PolicyRewrite;
use crate::search::{analyze_search_query, bm25_term_score_q16, Bm25CorpusStats, Bm25TermInput};

#[derive(Clone, Debug)]
pub struct EngineAqlProvider {
    index: EngineAqlIndex,
    agent_allowed: BTreeSet<u32>,
}

impl EngineAqlProvider {
    pub fn new(index: EngineAqlIndex, view: &AgentView) -> Self {
        let mut agent_allowed = BTreeSet::new();
        let policy = PolicyRewrite::new(view);
        for scope in policy.readable_scopes() {
            if let Some(candidates) = index.bitmaps.get(&scope_handle(scope)) {
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
    fn bitmap(&self, handle: BitmapHandle) -> Option<RoaringBitmap> {
        Some(
            self.index
                .bitmaps
                .get(&handle)
                .map(set_to_bitmap)
                .unwrap_or_default(),
        )
    }

    fn agent_allowed(&self) -> RoaringBitmap {
        set_to_bitmap(&self.agent_allowed)
    }

    fn live(&self) -> RoaringBitmap {
        set_to_bitmap(&self.index.universe)
    }

    fn universe(&self) -> RoaringBitmap {
        set_to_bitmap(&self.index.universe)
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
        let avg_len_q16 = average_doc_len_q16(&self.index, candidates);
        let doc_count = candidates.len();
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
                            avg_len_q16,
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

fn average_doc_len_q16(index: &EngineAqlIndex, candidates: &[u32]) -> u64 {
    let mut total = 0u64;
    let mut count = 0u64;
    for candidate in candidates {
        if let Some(length) = index.lexical_doc_lengths.get(candidate) {
            total = total.saturating_add(u64::from((*length).max(1)));
            count += 1;
        }
    }
    total
        .saturating_mul(65_536)
        .checked_div(count)
        .unwrap_or(65_536)
}

fn candidate_term_score(
    index: &EngineAqlIndex,
    candidate_set: &BTreeSet<u32>,
    doc_count: usize,
    avg_len_q16: u64,
    candidate: u32,
    term: &str,
    query_weight: u32,
) -> u64 {
    let Some(frequencies) = index.lexical_term_frequencies.get(term) else {
        return 0;
    };
    let tf = frequencies.get(&candidate).copied().unwrap_or_default();
    if tf == 0 {
        return 0;
    }
    let df = frequencies
        .keys()
        .filter(|candidate| candidate_set.contains(candidate))
        .count();
    bm25_term_score_q16(
        Bm25TermInput {
            tf,
            doc_len: index
                .lexical_doc_lengths
                .get(&candidate)
                .copied()
                .unwrap_or(1),
            avg_len_q16,
            query_weight,
            field_weight: 1,
        },
        Bm25CorpusStats {
            doc_count,
            doc_freq: df,
        },
        Default::default(),
    )
}

fn set_to_bitmap(values: &BTreeSet<u32>) -> RoaringBitmap {
    values.iter().copied().collect()
}
