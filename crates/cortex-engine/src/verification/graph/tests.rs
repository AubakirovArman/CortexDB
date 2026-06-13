use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::{CellDescriptor, CommitSeq, KnowledgeCellType};

use super::*;

#[test]
fn graph_contradiction_for_version_respects_descriptor_scope() {
    let payload = b"scope=project:visible\nstatus=ready\ntype=relation\nsource=payload-source\n\nsubject=cell:1\npredicate=fact_contradicts_fact\nobject=ABC budget approved"
        .to_vec();
    let descriptor = CellDescriptor {
        scope: "tenant:private".to_owned(),
        status: "ready".to_owned(),
        cell_type: KnowledgeCellType::Relation,
        source: Some("descriptor-source".to_owned()),
        source_trust_q16: Some(60_000),
        ..CellDescriptor::default()
    };
    let version =
        CellVersion::new_with_descriptor(CellId(10), CommitSeq(1), payload, 0, descriptor);
    let existing = BTreeSet::new();

    assert!(graph_contradiction_for_version(
        &version,
        "ABC budget approved",
        &view("project:visible"),
        &existing,
    )
    .is_none());
    assert!(graph_contradiction_for_version(
        &version,
        "ABC budget approved",
        &view("tenant:private"),
        &existing,
    )
    .is_some());
}

fn view(scope: &str) -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: None,
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope_id(scope)]),
        writable_scopes: BTreeSet::new(),
        allowed_modes: BTreeSet::from([RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::from([MemoryType::Decision]),
        max_context_budget_tokens: 1_000,
        default_context_budget_tokens: 400,
        max_candidate_limit: 100,
        default_candidate_limit: 20,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: None,
        allow_remember: false,
        allow_verify_fact: true,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: None,
    }
}
