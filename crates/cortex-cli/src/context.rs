use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_engine::{scope_id, ContextPack};

pub(crate) fn view_for_scope(scope: &str) -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: Some("local-cli".to_owned()),
        readable_brains: std::collections::BTreeSet::from([BrainId(1)]),
        readable_scopes: std::collections::BTreeSet::from([scope_id(scope)]),
        writable_scopes: std::collections::BTreeSet::new(),
        allowed_modes: std::collections::BTreeSet::from([RetrievalMode::Balanced]),
        allowed_memory_types: std::collections::BTreeSet::from([MemoryType::Decision]),
        max_context_budget_tokens: 4_000,
        default_context_budget_tokens: 1_000,
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

pub(crate) fn format_context_pack(pack: &ContextPack) -> String {
    let mut lines = vec![format!(
        "cells={} estimated_tokens={} token_budget={} truncated={} anomalies={}",
        pack.cells.len(),
        pack.estimated_tokens,
        pack.token_budget_tokens,
        pack.truncated,
        pack.anomalies.len()
    )];
    lines.extend(pack.cells.iter().map(|cell| {
        format!(
            "cell_id={} estimated_tokens={} citation={} payload={}",
            cell.cell_id.0,
            cell.estimated_tokens,
            cell.citation.as_deref().unwrap_or("null"),
            String::from_utf8_lossy(&cell.payload)
        )
    }));
    lines.join("\n")
}
