//! F04-B6.3: HTTP surface for agent transactions + the durable handoff-ledger.
//!
//! `POST /v1/transactions` commits an optimistic-concurrency write batch through
//! `commit_agent_transaction` (idempotency ledger, F04-B1.3); `POST /v1/handoff`
//! validates + commits a `SharedSequenced` handoff through `commit_agent_handoff`
//! (F08-B6.1). Both require an authenticated agent and are gated on the default-off
//! `agent_transactions` feature, so the routes are inert unless it is enabled.

use cortex_api_types::{
    AgentHandoffRequestBody, AgentHandoffResponse, AgentTransactionConflictResponse,
    AgentTransactionRequestBody, AgentTransactionResponse, WriteOpRequest,
};
use cortex_aql::{AgentId, AgentView};
use cortex_core::{CellId, CommitSeq};
use cortex_engine::{
    AgentHandoffRequest, AgentTransactionConflictKind, AgentTransactionOutcome,
    AgentTransactionRequest, Database, WriteBatch,
};

use crate::authz;
use crate::responses::RouterError;

/// `POST /v1/transactions`: commit an optimistic-concurrency write batch as the
/// authenticated agent. A conflict is returned as a normal `200` response whose
/// `outcome` is `"conflict"` (the request was processed; the outcome is domain
/// data), so the body carries the full conflict detail rather than an error
/// envelope. Reused idempotency keys and a disabled feature surface as engine
/// errors mapped through the taxonomy.
pub fn handle_transactions_shared(
    db: &mut Database,
    body: &[u8],
    authenticated_view: Option<&AgentView>,
) -> Result<String, RouterError> {
    let request: AgentTransactionRequestBody = serde_json::from_slice(body).map_err(|error| {
        RouterError::BadRequest(format!("invalid transaction request body: {error}"))
    })?;
    let view = authenticated_view.ok_or_else(|| {
        RouterError::PermissionDenied(
            "agent authentication is required for a transaction".to_owned(),
        )
    })?;
    authz::require_write_scope(view, &request.scope)?;

    let mut batch = WriteBatch::new();
    for operation in request.operations {
        batch = match operation {
            WriteOpRequest::Put { cell_id, payload } => {
                batch.put_cell(CellId(cell_id), payload.into_bytes())
            }
            WriteOpRequest::Patch { cell_id, payload } => {
                batch.patch_cell(CellId(cell_id), payload.into_bytes())
            }
            WriteOpRequest::Tombstone { cell_id } => batch.tombstone_cell(CellId(cell_id)),
        };
    }

    let engine_request = AgentTransactionRequest {
        agent_id: view.agent_id,
        scope: request.scope,
        base_seq: CommitSeq(request.base_seq),
        batch,
        idempotency_key: request.idempotency_key,
    };
    let report = db.commit_agent_transaction(view, engine_request)?;

    let response = AgentTransactionResponse {
        outcome: match report.outcome {
            AgentTransactionOutcome::Committed => "committed",
            AgentTransactionOutcome::Conflict => "conflict",
        }
        .to_owned(),
        committed_seq: report.committed_seq.map(|seq| seq.0),
        idempotent_replay: report.idempotent_replay,
        conflicts: report
            .conflicts
            .iter()
            .map(|conflict| AgentTransactionConflictResponse {
                cell_id: conflict.cell_id.0,
                base_seq: conflict.base_seq.0,
                observed_seq: conflict.observed_seq.0,
                kind: match conflict.kind {
                    AgentTransactionConflictKind::StaleCell => "stale_cell",
                    AgentTransactionConflictKind::TombstonedCell => "tombstoned_cell",
                }
                .to_owned(),
            })
            .collect(),
        idempotency_key: report.idempotency_key,
    };
    Ok(serde_json::to_string(&response)?)
}

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
        receipt_pack_root: request.receipt_pack_root,
        receipt_signature_context: request.receipt_signature_context,
    };
    let committed = db.commit_agent_handoff(source, &target, engine_request)?;

    let response = AgentHandoffResponse {
        handoff_cell_id: committed.handoff_cell_id.0,
        committed_seq: committed.committed_seq.0,
        level: "shared_sequenced".to_owned(),
        visible_after_seq: committed.report.visible_after_seq.0,
        target_can_read: committed.report.target_can_read,
        receipt_pack_root: committed.report.receipt_pack_root.clone(),
        receipt_signature_context: committed.report.receipt_signature_context.clone(),
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

    fn tx_body(base_seq: u64, cell: u64, key: Option<&str>) -> Vec<u8> {
        let key_field = match key {
            Some(k) => format!(r#","idempotency_key":"{k}""#),
            None => String::new(),
        };
        format!(
            r#"{{"scope":"agent:one","base_seq":{base_seq},"operations":[{{"op":"put","cell_id":{cell},"payload":"scope=agent:one\nstatus=ready\ntype=fact\n\nbody {cell}"}}]{key_field}}}"#
        )
        .into_bytes()
    }

    #[test]
    fn transaction_route_commits_and_replays_idempotently() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = enabled_db(dir.path());
        let view = agent_view(1, "agent:one");

        let out =
            handle_transactions_shared(&mut db, &tx_body(0, 1, Some("k1")), Some(&view)).unwrap();
        assert!(out.contains(r#""outcome":"committed""#), "{out}");
        assert!(out.contains(r#""idempotent_replay":false"#), "{out}");

        // The identical request replays from the ledger without re-writing.
        let replay =
            handle_transactions_shared(&mut db, &tx_body(0, 1, Some("k1")), Some(&view)).unwrap();
        assert!(replay.contains(r#""idempotent_replay":true"#), "{replay}");
    }

    #[test]
    fn transaction_route_returns_conflict_as_a_200_body() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = enabled_db(dir.path());
        let view = agent_view(1, "agent:one");

        // First write creates cell 1 (advancing the sequence past base_seq 0).
        handle_transactions_shared(&mut db, &tx_body(0, 1, None), Some(&view)).unwrap();
        // A second write to cell 1 with the now-stale base_seq 0 conflicts — and is
        // reported as a normal 200 body with the conflict detail, not an error.
        let out = handle_transactions_shared(&mut db, &tx_body(0, 1, None), Some(&view)).unwrap();
        assert!(out.contains(r#""outcome":"conflict""#), "{out}");
        assert!(out.contains(r#""cell_id":1"#), "{out}");
        assert!(out.contains("stale_cell"), "{out}");
    }

    #[test]
    fn transaction_route_requires_authentication() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = enabled_db(dir.path());
        let anon =
            handle_transactions_shared(&mut db, &tx_body(0, 1, Some("k")), None).unwrap_err();
        assert!(matches!(anon, RouterError::PermissionDenied(_)));
    }

    #[test]
    fn transaction_route_is_inert_when_feature_disabled() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::open(dir.path()).unwrap(); // feature off
        let view = agent_view(1, "agent:one");
        let error = handle_transactions_shared(&mut db, &tx_body(0, 1, Some("k")), Some(&view))
            .unwrap_err();
        assert!(matches!(error, RouterError::BadRequest(_)));
    }
}
