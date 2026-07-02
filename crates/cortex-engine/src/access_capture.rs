use cortex_aql::{AgentView, MemoryType, RetrievalMode};
use cortex_core::{CellDescriptor, CellId};
use cortex_crypto::{blake3_256_domain, hex_lower};
use serde_json::json;

use crate::canonical::canonical_json_bytes;
use crate::database::{CapturedAccessDecision, CapturedAccessDenial};
use crate::query::scope_id;

pub(crate) const CAPTURED_ACCESS_POLICY: &str = "agent_view_readable_scope";
pub(crate) const CAPTURED_ACCESS_POLICY_VERSION: &str = "agent_view_readable_scope.v1";
pub(crate) const MAX_CAPTURED_ACCESS_DENIALS: usize = 64;
const AGENT_VIEW_DIGEST_DOMAIN: &str = "cortexdb.agent_view.digest.v1";
const ACCESS_DENIAL_SCHEMA_VERSION: &str = "cortexdb.accountability.access_denial.v1";
const ACCESS_DENIAL_CELL_ID_HASH_DOMAIN: &str =
    "cortexdb.accountability.access_denial.cell_id_hash.v1";
const ACCESS_DENIAL_EVIDENCE_DIGEST_DOMAIN: &str =
    "cortexdb.accountability.access_denial.evidence_digest.v1";

pub(crate) fn captured_allowed_access_decision(
    cell_id: CellId,
    descriptor: &CellDescriptor,
    view: &AgentView,
) -> CapturedAccessDecision {
    let scope_id = scope_id(&descriptor.scope);
    CapturedAccessDecision {
        cell_id,
        policy: CAPTURED_ACCESS_POLICY.to_owned(),
        policy_version: CAPTURED_ACCESS_POLICY_VERSION.to_owned(),
        reason: "cell candidate survived AQL permission filtering before ContextPack packing"
            .to_owned(),
        scope: descriptor.scope.clone(),
        scope_id: scope_id.0,
        agent_id: Some(view.agent_id.0),
        agent_view_digest: agent_view_digest(view),
    }
}

pub(crate) fn captured_denied_access_decision(
    candidate: u32,
    cell_id: CellId,
    view: &AgentView,
) -> CapturedAccessDenial {
    let cell_id_hash = access_denial_cell_id_hash(cell_id);
    let agent_view_digest = agent_view_digest(view);
    let reason =
        "cell candidate was rejected by AQL agent access filtering before payload materialization"
            .to_owned();
    let canonical = canonical_json_bytes(&json!({
        "schema_version": ACCESS_DENIAL_SCHEMA_VERSION,
        "candidate": candidate,
        "cell_id_hash": cell_id_hash,
        "policy": CAPTURED_ACCESS_POLICY,
        "policy_version": CAPTURED_ACCESS_POLICY_VERSION,
        "reason": reason,
        "agent_id": view.agent_id.0,
        "agent_view_digest": agent_view_digest,
    }));
    let evidence_digest = hex_lower(&blake3_256_domain(
        ACCESS_DENIAL_EVIDENCE_DIGEST_DOMAIN,
        &canonical,
    ));
    CapturedAccessDenial {
        candidate,
        cell_id_hash,
        policy: CAPTURED_ACCESS_POLICY.to_owned(),
        policy_version: CAPTURED_ACCESS_POLICY_VERSION.to_owned(),
        reason,
        agent_id: Some(view.agent_id.0),
        agent_view_digest,
        evidence_digest,
    }
}

pub(crate) fn agent_view_digest(view: &AgentView) -> String {
    let canonical = canonical_json_bytes(&json!({
        "schema_version": "cortexdb.agent_view.projection.v1",
        "agent_id": view.agent_id.0,
        "label": view.label,
        "readable_brains": view.readable_brains.iter().map(|value| value.0).collect::<Vec<_>>(),
        "readable_scopes": view.readable_scopes.iter().map(|value| value.0).collect::<Vec<_>>(),
        "writable_scopes": view.writable_scopes.iter().map(|value| value.0).collect::<Vec<_>>(),
        "allowed_modes": view.allowed_modes.iter().map(|value| retrieval_mode_str(*value)).collect::<Vec<_>>(),
        "allowed_memory_types": view.allowed_memory_types.iter().map(|value| memory_type_str(*value)).collect::<Vec<_>>(),
        "max_context_budget_tokens": view.max_context_budget_tokens,
        "default_context_budget_tokens": view.default_context_budget_tokens,
        "max_candidate_limit": view.max_candidate_limit,
        "default_candidate_limit": view.default_candidate_limit,
        "min_required_confidence_q16": view.min_required_confidence_q16,
        "max_ttl_seconds": view.max_ttl_seconds,
        "allow_remember": view.allow_remember,
        "allow_verify_fact": view.allow_verify_fact,
        "allow_audit_mode": view.allow_audit_mode,
        "require_citations_by_default": view.require_citations_by_default,
        "private_scope": view.private_scope.map(|value| value.0),
    }));
    hex_lower(&blake3_256_domain(AGENT_VIEW_DIGEST_DOMAIN, &canonical))
}

fn access_denial_cell_id_hash(cell_id: CellId) -> String {
    let canonical = canonical_json_bytes(&json!({
        "schema_version": ACCESS_DENIAL_SCHEMA_VERSION,
        "cell_id": cell_id.0,
    }));
    hex_lower(&blake3_256_domain(
        ACCESS_DENIAL_CELL_ID_HASH_DOMAIN,
        &canonical,
    ))
}

fn retrieval_mode_str(value: RetrievalMode) -> &'static str {
    match value {
        RetrievalMode::Fast => "fast",
        RetrievalMode::Balanced => "balanced",
        RetrievalMode::Hybrid => "hybrid",
        RetrievalMode::Semantic => "semantic",
        RetrievalMode::Audit => "audit",
    }
}

fn memory_type_str(value: MemoryType) -> &'static str {
    match value {
        MemoryType::Decision => "decision",
        MemoryType::Preference => "preference",
        MemoryType::WorkflowResult => "workflow_result",
        MemoryType::ErrorLog => "error_log",
        MemoryType::Observation => "observation",
    }
}
