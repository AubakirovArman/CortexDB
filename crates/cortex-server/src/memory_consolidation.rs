//! F04-B4.4 (HTTP surface): the memory-consolidation plan/commit endpoints over
//! the engine's `semantic_compression_candidates` (B4.2) and
//! `commit_semantic_memory_compression` (B4.3). The reference MCP consolidation
//! worker (the agent that summarizes) drives these two steps: `plan` returns the
//! stale episodic groups to summarize; `commit` durably records the summary. Both
//! are gated on the default-off `semantic_compression` feature.

use std::time::{SystemTime, UNIX_EPOCH};

use cortex_api_types::{
    ConsolidateCandidate, ConsolidateCommitRequestBody, ConsolidateCommitResponse,
    ConsolidateGroup, ConsolidatePlanRequestBody, ConsolidatePlanResponse,
};
use cortex_aql::AgentView;
use cortex_core::CellId;
use cortex_engine::{Database, SemanticCompressionRequest, SemanticCompressionSourceRef};

use crate::authz;
use crate::responses::RouterError;

fn now_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// `POST /v1/memory/consolidate/plan`: the read-only "what to summarize" step.
pub fn handle_consolidate_plan_shared(
    db: &Database,
    body: &[u8],
    authenticated_view: Option<&AgentView>,
) -> Result<String, RouterError> {
    let request: ConsolidatePlanRequestBody = serde_json::from_slice(body).map_err(|error| {
        RouterError::BadRequest(format!("invalid consolidate plan request body: {error}"))
    })?;
    let view = authenticated_view.ok_or_else(|| {
        RouterError::PermissionDenied(
            "agent authentication is required for consolidation".to_owned(),
        )
    })?;
    authz::require_read_scope(view, &request.scope)?;

    let now = request.now_unix_seconds.unwrap_or_else(now_unix_seconds);
    let groups = db.semantic_compression_candidates(
        view,
        &request.scope,
        now,
        request.freshness_below_q16,
        request.max_groups as usize,
    )?;

    let response = ConsolidatePlanResponse {
        groups: groups
            .iter()
            .map(|group| ConsolidateGroup {
                scope: group.scope.clone(),
                memory_type: group.memory_type.clone(),
                candidates: group
                    .candidates
                    .iter()
                    .map(|candidate| ConsolidateCandidate {
                        cell_id: candidate.cell_id.0,
                        freshness_q16: candidate.freshness_q16,
                    })
                    .collect(),
            })
            .collect(),
    };
    Ok(serde_json::to_string(&response)?)
}

/// `POST /v1/memory/consolidate/commit`: durably record an externally-generated
/// summary over the source cells (the write step).
pub fn handle_consolidate_commit_shared(
    db: &mut Database,
    body: &[u8],
    authenticated_view: Option<&AgentView>,
) -> Result<String, RouterError> {
    let request: ConsolidateCommitRequestBody = serde_json::from_slice(body).map_err(|error| {
        RouterError::BadRequest(format!("invalid consolidate commit request body: {error}"))
    })?;
    let view = authenticated_view.ok_or_else(|| {
        RouterError::PermissionDenied(
            "agent authentication is required for consolidation".to_owned(),
        )
    })?;

    let engine_request = SemanticCompressionRequest {
        agent_id: view.agent_id,
        scope: request.scope,
        summary_cell_id: request.summary_cell_id.map(CellId),
        summary_payload: request.summary_payload.into_bytes(),
        source_refs: request
            .source_refs
            .into_iter()
            .map(|source| SemanticCompressionSourceRef {
                source_cell_id: CellId(source.source_cell_id),
                source_byte_start: source.source_byte_start as usize,
                source_byte_end: source.source_byte_end as usize,
            })
            .collect(),
        answerability_q16: request.answerability_q16,
        external_worker: request.external_worker,
        idempotency_key: request.idempotency_key,
    };
    let report = db.commit_semantic_memory_compression(view, engine_request)?;

    let response = ConsolidateCommitResponse {
        summary_cell_id: report.summary_cell_id.0,
        committed_seq: report.committed_seq.0,
        source_cell_ids: report.source_cell_ids.iter().map(|id| id.0).collect(),
        source_ref_count: report.source_ref_count as u64,
        answerability_q16: report.answerability_q16,
        provenance_preserved: report.provenance_preserved,
        auditable: report.auditable,
    };
    Ok(serde_json::to_string(&response)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cortex_api_types::ConsolidateSourceRef;
    use cortex_aql::{AgentId, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
    use cortex_engine::{scope_id, DatabaseOptions, SemanticCompressionOptions};
    use std::collections::BTreeSet;

    fn agent_view(agent_id: u64, scope: &str) -> AgentView {
        let scope = scope_id(scope);
        AgentView {
            agent_id: AgentId(agent_id),
            label: Some(format!("agent-{agent_id}")),
            readable_brains: BTreeSet::from([BrainId(1)]),
            readable_scopes: BTreeSet::from([scope]),
            writable_scopes: BTreeSet::from([scope]),
            allowed_modes: BTreeSet::from([RetrievalMode::Balanced]),
            allowed_memory_types: BTreeSet::from([MemoryType::Observation]),
            max_context_budget_tokens: 4_000,
            default_context_budget_tokens: 1_000,
            max_candidate_limit: 32,
            default_candidate_limit: 20,
            min_required_confidence_q16: Q16_ZERO,
            max_ttl_seconds: Some(3_600),
            allow_remember: true,
            allow_verify_fact: true,
            allow_audit_mode: false,
            require_citations_by_default: false,
            private_scope: None,
        }
    }

    fn enabled_db(path: &std::path::Path) -> Database {
        Database::open_with_options(
            path,
            DatabaseOptions {
                semantic_compression: SemanticCompressionOptions {
                    enabled: true,
                    min_answerability_q16: 32_768,
                },
                ..DatabaseOptions::default()
            },
        )
        .unwrap()
    }

    fn memory_cell(scope: &str, memory_type: &str, ttl: Option<u64>, body: &str) -> Vec<u8> {
        let ttl_line = ttl
            .map(|t| format!("created_unix_seconds=1000\nttl_seconds={t}\n"))
            .unwrap_or_default();
        format!("scope={scope}\nstatus=ready\ntype=memory\nmemory_type={memory_type}\n{ttl_line}source=test\n\n{body}")
            .into_bytes()
    }

    #[test]
    fn plan_returns_stale_episodic_groups() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = enabled_db(dir.path());
        db.put_cell(
            CellId(10),
            memory_cell("agent:one", "observation", Some(100), "stale"),
        )
        .unwrap();
        let view = agent_view(1, "agent:one");

        let body = ConsolidatePlanRequestBody {
            scope: "agent:one".to_owned(),
            freshness_below_q16: 32_768,
            max_groups: 8,
            now_unix_seconds: Some(1_080),
        };
        let out =
            handle_consolidate_plan_shared(&db, &serde_json::to_vec(&body).unwrap(), Some(&view))
                .unwrap();
        assert!(out.contains(r#""cell_id":10"#), "{out}");
        assert!(out.contains("observation"), "{out}");
    }

    #[test]
    fn plan_requires_authentication() {
        let dir = tempfile::tempdir().unwrap();
        let db = enabled_db(dir.path());
        let body = ConsolidatePlanRequestBody {
            scope: "agent:one".to_owned(),
            freshness_below_q16: 32_768,
            max_groups: 8,
            now_unix_seconds: Some(1_080),
        };
        let error = handle_consolidate_plan_shared(&db, &serde_json::to_vec(&body).unwrap(), None)
            .unwrap_err();
        assert!(matches!(error, RouterError::PermissionDenied(_)));
    }

    #[test]
    fn commit_records_summary_over_sources() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = enabled_db(dir.path());
        db.put_cell(
            CellId(1),
            memory_cell("agent:one", "observation", None, "source one"),
        )
        .unwrap();
        db.put_cell(
            CellId(2),
            memory_cell("agent:one", "observation", None, "source two"),
        )
        .unwrap();
        let view = agent_view(1, "agent:one");

        let summary_payload = "scope=agent:one\nstatus=ready\ntype=memory\nmemory_type=observation\ncompression_kind=semantic_summary\ncompression_source_cells=1,2\ncompression_answerability_q16=60000\ncompression_worker=mcp-summary-v1\nsource=mcp-summary-v1\n\nConsolidated summary.".to_owned();
        let body = ConsolidateCommitRequestBody {
            scope: "agent:one".to_owned(),
            summary_cell_id: Some(99),
            summary_payload,
            source_refs: vec![
                ConsolidateSourceRef {
                    source_cell_id: 1,
                    source_byte_start: 0,
                    source_byte_end: 32,
                },
                ConsolidateSourceRef {
                    source_cell_id: 2,
                    source_byte_start: 0,
                    source_byte_end: 32,
                },
            ],
            answerability_q16: 60_000,
            external_worker: "mcp-summary-v1".to_owned(),
            idempotency_key: None,
        };
        let out = handle_consolidate_commit_shared(
            &mut db,
            &serde_json::to_vec(&body).unwrap(),
            Some(&view),
        )
        .unwrap();
        assert!(out.contains(r#""summary_cell_id":99"#), "{out}");
        assert!(out.contains(r#""provenance_preserved":true"#), "{out}");
        assert!(out.contains(r#""auditable":true"#), "{out}");
    }

    #[test]
    fn consolidate_is_inert_when_feature_disabled() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(dir.path()).unwrap(); // feature off
        let view = agent_view(1, "agent:one");
        let body = ConsolidatePlanRequestBody {
            scope: "agent:one".to_owned(),
            freshness_below_q16: 32_768,
            max_groups: 8,
            now_unix_seconds: Some(1_080),
        };
        let error =
            handle_consolidate_plan_shared(&db, &serde_json::to_vec(&body).unwrap(), Some(&view))
                .unwrap_err();
        assert!(matches!(error, RouterError::BadRequest(_)));
    }
}
