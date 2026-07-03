//! F04-B6.3 (handoff route): HTTP surface for the durable agent handoff-ledger.
//!
//! `POST /v1/handoff` validates + commits a `SharedSequenced` handoff through the
//! engine's `commit_agent_handoff` (F08-B6.1) and returns the durable record. The
//! caller must be authenticated as the source agent; the target agent's view is
//! resolved from the engine's agent-view store. The engine gates the write on the
//! default-off `agent_transactions` feature, so this route is inert unless enabled.

use cortex_api_types::{AgentHandoffRequestBody, AgentHandoffResponse};
use cortex_aql::{AgentId, AgentView};
use cortex_core::CommitSeq;
use cortex_engine::{AgentHandoffRequest, Database};

use crate::responses::RouterError;

pub fn handle_handoff_shared(
    db: &mut Database,
    body: &[u8],
    authenticated_view: Option<&AgentView>,
) -> Result<String, RouterError> {
    let request: AgentHandoffRequestBody = serde_json::from_slice(body).map_err(|error| {
        RouterError::BadRequest(format!("invalid handoff request body: {error}"))
    })?;

    // The caller must be authenticated as the source agent.
    let source = authenticated_view.ok_or_else(|| {
        RouterError::PermissionDenied("agent authentication is required for a handoff".to_owned())
    })?;
    if source.agent_id != AgentId(request.source_agent_id) {
        return Err(RouterError::PermissionDenied(
            "source_agent_id does not match the authenticated agent".to_owned(),
        ));
    }

    // Resolve the target agent's view from the engine's store.
    let target = db
        .load_agent_view(AgentId(request.target_agent_id))?
        .ok_or_else(|| RouterError::BadRequest("target agent view not found".to_owned()))?;

    let engine_request = AgentHandoffRequest {
        source_agent_id: AgentId(request.source_agent_id),
        target_agent_id: AgentId(request.target_agent_id),
        scope: request.scope,
        pack_hash: request.pack_hash,
        pack_seq: CommitSeq(request.pack_seq),
        required_after_seq: CommitSeq(request.required_after_seq),
        idempotency_key: request.idempotency_key,
    };
    let committed = db.commit_agent_handoff(source, &target, engine_request)?;

    let response = AgentHandoffResponse {
        handoff_cell_id: committed.handoff_cell_id.0,
        committed_seq: committed.committed_seq.0,
        level: "shared_sequenced".to_owned(),
        visible_after_seq: committed.report.visible_after_seq.0,
        target_can_read: committed.report.target_can_read,
    };
    Ok(serde_json::to_string(&response)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cortex_aql::{BrainId, MemoryType, RetrievalMode, Q16_ZERO};
    use cortex_engine::{scope_id, AgentTransactionOptions, DatabaseOptions};
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
            allowed_memory_types: BTreeSet::from([MemoryType::Decision]),
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
                agent_transactions: AgentTransactionOptions { enabled: true },
                ..DatabaseOptions::default()
            },
        )
        .unwrap()
    }

    fn body(source: u64, target: u64) -> Vec<u8> {
        format!(
            r#"{{"source_agent_id":{source},"target_agent_id":{target},"scope":"shared:project","pack_hash":"ctxpack:v1:gamma","pack_seq":0,"required_after_seq":0}}"#
        )
        .into_bytes()
    }

    #[test]
    fn handoff_route_commits_and_returns_record() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = enabled_db(dir.path());
        let source = agent_view(1, "shared:project");
        db.save_agent_view(&agent_view(2, "shared:project"))
            .unwrap();

        let out = handle_handoff_shared(&mut db, &body(1, 2), Some(&source)).unwrap();
        assert!(out.contains(r#""level":"shared_sequenced""#), "{out}");
        assert!(out.contains(r#""target_can_read":true"#), "{out}");
        assert!(out.contains(r#""handoff_cell_id":"#), "{out}");
    }

    #[test]
    fn handoff_route_requires_authenticated_source_match() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = enabled_db(dir.path());
        db.save_agent_view(&agent_view(2, "shared:project"))
            .unwrap();

        // Authenticated as agent 9 but the body claims source 1.
        let mismatch =
            handle_handoff_shared(&mut db, &body(1, 2), Some(&agent_view(9, "shared:project")))
                .unwrap_err();
        assert!(matches!(mismatch, RouterError::PermissionDenied(_)));
        // No authenticated agent at all.
        let anon = handle_handoff_shared(&mut db, &body(1, 2), None).unwrap_err();
        assert!(matches!(anon, RouterError::PermissionDenied(_)));
    }

    #[test]
    fn handoff_route_unknown_target_is_bad_request() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = enabled_db(dir.path());
        // Target agent 2 was never provisioned.
        let error =
            handle_handoff_shared(&mut db, &body(1, 2), Some(&agent_view(1, "shared:project")))
                .unwrap_err();
        assert!(matches!(error, RouterError::BadRequest(_)));
    }

    #[test]
    fn handoff_route_is_inert_when_feature_disabled() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::open(dir.path()).unwrap(); // feature off
        db.save_agent_view(&agent_view(2, "shared:project"))
            .unwrap();
        let error =
            handle_handoff_shared(&mut db, &body(1, 2), Some(&agent_view(1, "shared:project")))
                .unwrap_err();
        // The engine's FeatureDisabled maps through the error taxonomy to a client error.
        assert!(matches!(error, RouterError::BadRequest(_)));
    }
}
