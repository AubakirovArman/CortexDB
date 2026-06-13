pub(super) mod prelude {
    pub use std::collections::{BTreeMap, BTreeSet};

    pub use cortex_aql::{
        AgentId, AgentView, BrainId, MemoryType, RetrievalMode, ScopeId, Q16_ZERO,
    };
    pub use cortex_core::{CellId, KnowledgeCell, KnowledgeCellMetadata, KnowledgeCellType};
    pub use cortex_engine::{
        scope_id, tokenize, Bm25Index, ClusterConfig, ConsensusState, Database, DatabaseOptions,
        HnswIndex, LogIndex, NodeId, PayloadResidency, ReplicatedEntry, SearchIndexes, SearchMode,
        SearchQuery, SearchRerankInput, SearchReranker, Term, VectorIndex,
    };
}

use prelude::*;

pub(super) fn view(scope: ScopeId) -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: None,
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope]),
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
        require_citations_by_default: false,
        private_scope: None,
    }
}
