use std::collections::BTreeSet;

use cortex_aql::{
    AgentId, AgentView, BitmapHandle, BrainId, CellTypeId, MemoryType, RetrievalMode, ScopeId,
    StatusId, Q16_ZERO,
};

pub(crate) struct DeterministicRng(u64);

impl DeterministicRng {
    pub(crate) fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub(crate) fn index(&mut self, upper: usize) -> usize {
        (self.next_u32() as usize) % upper
    }

    pub(crate) fn chance(&mut self, numerator: u32, denominator: u32) -> bool {
        self.next_u32() % denominator < numerator
    }

    fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 32) as u32
    }
}

pub(crate) fn scope_id(index: usize) -> ScopeId {
    ScopeId(10 + index as u64)
}

pub(crate) fn status_id(index: usize) -> StatusId {
    StatusId(20 + index as u64)
}

pub(crate) fn cell_type_id(index: usize) -> CellTypeId {
    CellTypeId(30 + index as u64)
}

pub(crate) fn scope_handle(index: usize) -> BitmapHandle {
    BitmapHandle(100 + index as u64)
}

pub(crate) fn status_handle(index: usize) -> BitmapHandle {
    BitmapHandle(200 + index as u64)
}

pub(crate) fn cell_type_handle(index: usize) -> BitmapHandle {
    BitmapHandle(300 + index as u64)
}

pub(crate) fn agent_view(readable_scope_indexes: &BTreeSet<usize>) -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: None,
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: readable_scope_indexes
            .iter()
            .copied()
            .map(scope_id)
            .collect(),
        writable_scopes: BTreeSet::new(),
        allowed_modes: BTreeSet::from([RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::from([MemoryType::Decision]),
        max_context_budget_tokens: 2_000,
        default_context_budget_tokens: 1_000,
        max_candidate_limit: 100,
        default_candidate_limit: 40,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: Some(3_600),
        allow_remember: false,
        allow_verify_fact: false,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: None,
    }
}
