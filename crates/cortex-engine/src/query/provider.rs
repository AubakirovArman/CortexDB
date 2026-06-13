use std::collections::BTreeSet;

use cortex_aql::{AgentView, BitmapHandle, BitmapProvider};
use cortex_core::CellId;

use super::metadata::scope_handle;
use super::EngineAqlIndex;
use crate::database::CandidateResolver;

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
}
