use cortex_aql::{AgentId, AgentView, BindError, MemoryType, PolicyError};
use cortex_core::{CellId, CommitSeq};

use crate::cell_ids::{agent_cell_id_slot, namespaced_agent_cell_id};
use crate::database::{Database, RetrievedCell};
use crate::error::{EngineError, EngineResult};
use crate::query::scope_id;

mod index;
mod payload;

pub(crate) use index::SessionIndex;
use payload::{session_payload, SessionCellKind, SessionPayload};

const SESSION_CELL_NAMESPACE: u64 = 0xa000_0000_0000_0000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentSession {
    pub session_id: String,
    pub agent_id: AgentId,
    pub scope: String,
    pub context_cell_id: CellId,
    pub commit_seq: CommitSeq,
    pub created_unix_seconds: u64,
    pub ttl_seconds: u64,
    pub expires_at_unix_seconds: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionMemory {
    pub session_id: String,
    pub cell_id: CellId,
    pub commit_seq: CommitSeq,
    pub ttl_seconds: u64,
}

impl AgentSession {
    pub fn is_expired(&self, now_unix_seconds: u64) -> bool {
        now_unix_seconds >= self.expires_at_unix_seconds
    }

    pub fn remaining_ttl_seconds(&self, now_unix_seconds: u64) -> Option<u64> {
        (!self.is_expired(now_unix_seconds))
            .then_some(
                self.expires_at_unix_seconds
                    .saturating_sub(now_unix_seconds),
            )
            .filter(|remaining| *remaining > 0)
    }
}

impl Database {
    pub fn start_agent_session(
        &mut self,
        view: &AgentView,
        scope: &str,
        context: impl AsRef<[u8]>,
        ttl_seconds: u64,
        now_unix_seconds: u64,
    ) -> EngineResult<AgentSession> {
        enforce_session_memory_policy(view, scope, MemoryType::WorkflowResult, ttl_seconds)?;
        let session_id = self.next_session_id(view.agent_id)?;
        let cell_id = self.next_session_cell_id(view.agent_id)?;
        let payload = session_payload(SessionPayload {
            scope,
            agent_id: view.agent_id,
            session_id: &session_id,
            kind: SessionCellKind::Context,
            memory_type: "workflow_result",
            ttl_seconds,
            created_unix_seconds: now_unix_seconds,
            body: context.as_ref(),
        });
        let commit_seq = self.put_cell(cell_id, payload)?;
        Ok(AgentSession {
            session_id,
            agent_id: view.agent_id,
            scope: scope.to_owned(),
            context_cell_id: cell_id,
            commit_seq,
            created_unix_seconds: now_unix_seconds,
            ttl_seconds,
            expires_at_unix_seconds: now_unix_seconds.saturating_add(ttl_seconds),
        })
    }

    pub fn remember_session_memory(
        &mut self,
        session: &AgentSession,
        view: &AgentView,
        content: impl AsRef<[u8]>,
        ttl_seconds: Option<u64>,
        now_unix_seconds: u64,
    ) -> EngineResult<SessionMemory> {
        let remaining = session
            .remaining_ttl_seconds(now_unix_seconds)
            .ok_or_else(|| EngineError::AgentSessionExpired(session.session_id.clone()))?;
        let ttl_seconds = ttl_seconds.unwrap_or(remaining);
        if ttl_seconds > remaining {
            return Err(policy_denied(PolicyError::TtlTooLong));
        }
        enforce_session_memory_policy(view, &session.scope, MemoryType::Observation, ttl_seconds)?;
        let cell_id = self.next_session_cell_id(view.agent_id)?;
        let payload = session_payload(SessionPayload {
            scope: &session.scope,
            agent_id: view.agent_id,
            session_id: &session.session_id,
            kind: SessionCellKind::TemporaryMemory,
            memory_type: "observation",
            ttl_seconds,
            created_unix_seconds: now_unix_seconds,
            body: content.as_ref(),
        });
        let commit_seq = self.put_cell(cell_id, payload)?;
        Ok(SessionMemory {
            session_id: session.session_id.clone(),
            cell_id,
            commit_seq,
            ttl_seconds,
        })
    }

    pub fn retrieve_session_cells(
        &self,
        session_id: &str,
        view: &AgentView,
        now_unix_seconds: u64,
    ) -> Vec<RetrievedCell> {
        let txn = self.read_txn();
        let mut cells = self.derived_stores.session_index.retrieve(
            session_id,
            view,
            now_unix_seconds,
            |cell_id, resident, _| {
                if let Some(payload) = resident {
                    return Some(payload.to_vec());
                }
                self.memtable
                    .read(txn, cell_id)
                    .and_then(|version| self.payload_for_version(version).ok())
            },
        );
        cells.sort_by_key(|cell| cell.cell_id);
        cells
    }

    fn next_session_id(&self, agent_id: AgentId) -> EngineResult<String> {
        let sequence = self
            .current_seq()
            .0
            .checked_add(1)
            .ok_or_else(session_id_overflow)?;
        Ok(format!("agent-{}-session-{sequence}", agent_id.0))
    }

    fn next_session_cell_id(&self, agent_id: AgentId) -> EngineResult<CellId> {
        let agent_slot = agent_cell_id_slot(agent_id).ok_or_else(session_id_overflow)?;
        let mut sequence = self
            .current_seq()
            .0
            .checked_add(1)
            .ok_or_else(session_id_overflow)?;
        let mut attempts = 0u64;
        loop {
            let cell_id = namespaced_agent_cell_id(SESSION_CELL_NAMESPACE, agent_slot, sequence)
                .ok_or_else(session_id_overflow)?;
            if self.get_latest_cell_descriptor(cell_id).is_none() {
                return Ok(cell_id);
            }
            attempts = attempts.checked_add(1).ok_or_else(session_id_overflow)?;
            if attempts > u64::from(u32::MAX) {
                return Err(session_id_overflow());
            }
            sequence = sequence.checked_add(1).ok_or_else(session_id_overflow)?;
        }
    }
}

fn enforce_session_memory_policy(
    view: &AgentView,
    scope: &str,
    memory_type: MemoryType,
    ttl_seconds: u64,
) -> EngineResult<()> {
    if ttl_seconds == 0 {
        return Err(EngineError::InvalidAgentSession(
            "session ttl_seconds must be greater than zero".to_owned(),
        ));
    }
    if !view.allow_remember {
        return Err(policy_denied(PolicyError::RememberNotAllowed));
    }
    if !view.can_write_scope(scope_id(scope)) {
        return Err(policy_denied(PolicyError::ScopeNotWritable));
    }
    if !view.can_remember_type(memory_type) {
        return Err(policy_denied(PolicyError::MemoryTypeNotAllowed));
    }
    if let Some(max_ttl) = view.max_ttl_seconds {
        if ttl_seconds > max_ttl {
            return Err(policy_denied(PolicyError::TtlTooLong));
        }
    }
    Ok(())
}

fn policy_denied(error: PolicyError) -> EngineError {
    EngineError::AqlBind(BindError::PolicyDenied(error))
}

fn session_id_overflow() -> EngineError {
    EngineError::StorageInvariant("session cell id space is exhausted".to_owned())
}
