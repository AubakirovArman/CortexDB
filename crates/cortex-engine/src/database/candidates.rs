use std::collections::BTreeSet;

use cortex_aql::BitmapProvider;
use cortex_core::{CellDescriptor, CellId};

use super::{CapturedAccessDecision, CapturedAccessDenial};

pub trait CandidateResolver: BitmapProvider {
    fn cell_id_for_candidate(&self, candidate: u32) -> Option<CellId>;

    fn captured_access_decision_for_candidate(
        &self,
        _candidate: u32,
        _cell_id: CellId,
        _descriptor: &CellDescriptor,
    ) -> Option<CapturedAccessDecision> {
        None
    }

    fn captured_access_denial_for_candidate(
        &self,
        _candidate: u32,
    ) -> Option<CapturedAccessDenial> {
        None
    }

    fn lexical_candidates_for_terms(&self, _terms: &[String]) -> Option<BTreeSet<u32>> {
        None
    }

    fn ranked_candidates_for_task(&self, _task: &str, _candidates: &[u32]) -> Option<Vec<u32>> {
        None
    }
}
