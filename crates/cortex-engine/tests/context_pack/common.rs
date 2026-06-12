use std::collections::BTreeSet;

pub(super) mod prelude {
    pub use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
    pub use cortex_core::CellId;
    pub use cortex_engine::feedback::ContextFeedback;
    pub use cortex_engine::{
        scope_id, ContextPack, ContextPackAnomalyCode, ContextPackExportFormat, ContextPackOptions,
        Database, RetrievedCell, SourceTrustCategory, DEFAULT_CITATION_OVERHEAD_TOKENS,
        INTERNAL_SOURCE_TRUST_Q16,
    };
}

use prelude::*;

pub(super) fn query() -> &'static str {
    r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
WHERE space = project:investments AND status = "ready" LIMIT 10 CANDIDATES;"#
}

pub(super) fn retrieved(cell_id: u64, payload: &str) -> RetrievedCell {
    RetrievedCell::from_payload(CellId(cell_id), payload.as_bytes().to_vec())
}

pub(super) fn view(require_citations: bool) -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: None,
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope_id("project:investments")]),
        writable_scopes: BTreeSet::new(),
        allowed_modes: BTreeSet::from([RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::from([MemoryType::Decision]),
        max_context_budget_tokens: 1_000,
        default_context_budget_tokens: 400,
        max_candidate_limit: 100,
        default_candidate_limit: 20,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: Some(3_600),
        allow_remember: false,
        allow_verify_fact: false,
        allow_audit_mode: false,
        require_citations_by_default: require_citations,
        private_scope: None,
    }
}
