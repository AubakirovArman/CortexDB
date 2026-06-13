use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::memtable::CellVersion;
use cortex_core::{CellDescriptor, CellId, CommitSeq, KnowledgeCellType};

use super::evidence::{contradiction_for_version, evidence_for_version, MaterializedVersion};
use crate::query::scope_id;

#[test]
fn evidence_for_version_respects_descriptor_scope_over_payload_scope() {
    let version = mismatched_version(
        CellId(7),
        b"scope=project:visible\nstatus=ready\ntype=fact\nsource=payload-source\n\nABC budget approved",
        KnowledgeCellType::Fact,
    );

    let candidate = materialized_version(&version);

    assert!(
        evidence_for_version(&candidate, &view("project:visible"), "ABC budget approved").is_none()
    );
    assert!(
        evidence_for_version(&candidate, &view("tenant:private"), "ABC budget approved").is_some()
    );
}

#[test]
fn contradiction_for_version_respects_descriptor_scope_over_payload_scope() {
    let version = mismatched_version(
        CellId(8),
        b"scope=project:visible\nstatus=ready\ntype=fact\nsource=payload-source\n\ncontradicts=ABC budget approved\nABC budget rejected",
        KnowledgeCellType::Fact,
    );

    let candidate = materialized_version(&version);

    assert!(
        contradiction_for_version(&candidate, &view("project:visible"), "ABC budget approved",)
            .is_none()
    );
    assert!(
        contradiction_for_version(&candidate, &view("tenant:private"), "ABC budget approved",)
            .is_some()
    );
}

fn materialized_version(version: &CellVersion) -> MaterializedVersion<'_> {
    MaterializedVersion {
        version,
        payload: version.payload.clone(),
    }
}

fn mismatched_version(
    cell_id: CellId,
    payload: &[u8],
    cell_type: KnowledgeCellType,
) -> CellVersion {
    let descriptor = CellDescriptor {
        scope: "tenant:private".to_owned(),
        status: "ready".to_owned(),
        cell_type,
        source: Some("descriptor-source".to_owned()),
        source_trust_q16: Some(60_000),
        ..CellDescriptor::default()
    };
    CellVersion::new_with_descriptor(cell_id, CommitSeq(1), payload.to_vec(), 0, descriptor)
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
