use std::collections::BTreeSet;

use crate::types::{AgentId, BrainId, MemoryType, RetrievalMode, ScopeId, Q16};

#[derive(Clone, Debug)]
pub struct AgentView {
    pub agent_id: AgentId,
    pub label: Option<String>,
    pub readable_brains: BTreeSet<BrainId>,
    pub readable_scopes: BTreeSet<ScopeId>,
    pub writable_scopes: BTreeSet<ScopeId>,
    pub allowed_modes: BTreeSet<RetrievalMode>,
    pub allowed_memory_types: BTreeSet<MemoryType>,
    pub max_context_budget_tokens: u32,
    pub default_context_budget_tokens: u32,
    pub max_candidate_limit: u32,
    pub default_candidate_limit: u32,
    pub min_required_confidence_q16: Q16,
    pub max_ttl_seconds: Option<u64>,
    pub allow_remember: bool,
    pub allow_verify_fact: bool,
    pub allow_audit_mode: bool,
    pub require_citations_by_default: bool,
    pub private_scope: Option<ScopeId>,
}

impl AgentView {
    pub fn can_read_brain(&self, brain: BrainId) -> bool {
        self.readable_brains.contains(&brain)
    }

    pub fn can_read_scope(&self, scope: ScopeId) -> bool {
        self.readable_scopes.contains(&scope)
    }

    pub fn can_write_scope(&self, scope: ScopeId) -> bool {
        self.writable_scopes.contains(&scope)
    }

    pub fn can_use_mode(&self, mode: RetrievalMode) -> bool {
        self.allowed_modes.contains(&mode)
    }

    pub fn can_remember_type(&self, memory_type: MemoryType) -> bool {
        self.allow_remember && self.allowed_memory_types.contains(&memory_type)
    }

    pub fn can_verify_fact(&self) -> bool {
        self.allow_verify_fact
    }

    pub fn can_use_audit_mode(&self) -> bool {
        self.allow_audit_mode && self.can_use_mode(RetrievalMode::Audit)
    }

    pub fn effective_budget(&self, requested: u32) -> u32 {
        requested.min(self.max_context_budget_tokens)
    }

    pub fn effective_candidate_limit(&self, requested: u32) -> u32 {
        requested.min(self.max_candidate_limit)
    }
}
