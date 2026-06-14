use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, ScopeId, Q16_ZERO};
use cortex_engine::scope_id;
use serde_json::Value;

pub(super) fn payload(index: usize) -> Vec<u8> {
    format!(
        "scope=bench\nstatus=ready\ntype=fact\nsource=concurrent-read-{index}\n\nconcurrent read benchmark ready budget cell {index}"
    )
    .into_bytes()
}

pub(super) fn bench_view() -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: Some("concurrent-read-benchmark".to_owned()),
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope_id("bench")]),
        writable_scopes: BTreeSet::from([scope_id("bench")]),
        allowed_modes: BTreeSet::from([RetrievalMode::Fast, RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::from([
            MemoryType::Decision,
            MemoryType::Preference,
            MemoryType::WorkflowResult,
            MemoryType::ErrorLog,
            MemoryType::Observation,
        ]),
        max_context_budget_tokens: 10_000,
        default_context_budget_tokens: 1_000,
        max_candidate_limit: 1_000,
        default_candidate_limit: 10,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: None,
        allow_remember: true,
        allow_verify_fact: true,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: Some(ScopeId(999)),
    }
}

pub(super) fn ensure_aql_response(response: &str) -> Result<(), String> {
    let value = serde_json::from_str::<Value>(response)
        .map_err(|error| format!("invalid AQL response JSON: {error}"))?;
    let cells = value
        .get("cells")
        .and_then(Value::as_array)
        .ok_or_else(|| "AQL response missing cells".to_owned())?;
    if cells.is_empty() {
        return Err("AQL response returned no cells".to_owned());
    }
    Ok(())
}

pub(super) fn ensure_write_response(response: &str) -> Result<(), String> {
    let value = serde_json::from_str::<Value>(response)
        .map_err(|error| format!("invalid write response JSON: {error}"))?;
    if value.get("seq").and_then(Value::as_u64).is_none() {
        return Err("write response missing seq".to_owned());
    }
    Ok(())
}
