use cortex_aql::{AgentId, AgentView, BindError, PolicyError, ScopeId};
use cortex_core::CommitSeq;

use crate::database::Database;
use crate::error::{EngineError, EngineResult};
use crate::plan::PolicyRewrite;
use crate::query::scope_id;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryConsistencyLevel {
    PrivateReadYourWrites,
    SharedImmediate,
    SharedSequenced,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryVisibilityReport {
    pub level: MemoryConsistencyLevel,
    pub viewer_agent_id: AgentId,
    pub owner_agent_id: AgentId,
    pub scope: String,
    pub scope_id: ScopeId,
    pub readable: bool,
    pub writable: bool,
    pub visible_after_seq: CommitSeq,
    pub reason: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentHandoffRequest {
    pub source_agent_id: AgentId,
    pub target_agent_id: AgentId,
    pub scope: String,
    pub pack_hash: String,
    pub pack_seq: CommitSeq,
    pub required_after_seq: CommitSeq,
    pub idempotency_key: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentHandoffReport {
    pub level: MemoryConsistencyLevel,
    pub source_agent_id: AgentId,
    pub target_agent_id: AgentId,
    pub scope: String,
    pub scope_id: ScopeId,
    pub pack_hash: String,
    pub pack_seq: CommitSeq,
    pub required_after_seq: CommitSeq,
    pub visible_after_seq: CommitSeq,
    pub target_can_read: bool,
    pub idempotency_key: Option<String>,
}

pub fn classify_memory_visibility(
    view: &AgentView,
    owner_agent_id: AgentId,
    scope: &str,
    visible_after_seq: CommitSeq,
) -> MemoryVisibilityReport {
    let scope_id = scope_id(scope);
    let readable = PolicyRewrite::allows_scope(view, scope_id);
    let writable = view.can_write_scope(scope_id);
    let is_private_owner_scope =
        view.agent_id == owner_agent_id && view.private_scope == Some(scope_id);
    let level = if is_private_owner_scope {
        MemoryConsistencyLevel::PrivateReadYourWrites
    } else {
        MemoryConsistencyLevel::SharedImmediate
    };
    let reason = if !readable {
        "scope is not readable by AgentView"
    } else if is_private_owner_scope {
        "owner private scope is read-your-writes only"
    } else {
        "shared scope is immediately visible after commit sequence"
    };

    MemoryVisibilityReport {
        level,
        viewer_agent_id: view.agent_id,
        owner_agent_id,
        scope: scope.to_owned(),
        scope_id,
        readable,
        writable,
        visible_after_seq,
        reason,
    }
}

impl Database {
    pub fn plan_agent_handoff(
        &self,
        source_view: &AgentView,
        target_view: &AgentView,
        request: AgentHandoffRequest,
    ) -> EngineResult<AgentHandoffReport> {
        validate_handoff_view(source_view, request.source_agent_id, "source")?;
        validate_handoff_view(target_view, request.target_agent_id, "target")?;
        validate_handoff_sequence(self.current_seq(), &request)?;
        if request.pack_hash.trim().is_empty() {
            return invalid("agent handoff pack_hash is required");
        }

        let scope_id = scope_id(&request.scope);
        if !PolicyRewrite::allows_scope(source_view, scope_id) {
            return Err(policy_denied(PolicyError::ScopeNotReadable));
        }
        if !PolicyRewrite::allows_scope(target_view, scope_id) {
            return Err(policy_denied(PolicyError::ScopeNotReadable));
        }

        Ok(AgentHandoffReport {
            level: MemoryConsistencyLevel::SharedSequenced,
            source_agent_id: request.source_agent_id,
            target_agent_id: request.target_agent_id,
            scope: request.scope,
            scope_id,
            pack_hash: request.pack_hash,
            pack_seq: request.pack_seq,
            required_after_seq: request.required_after_seq,
            visible_after_seq: request.pack_seq,
            target_can_read: true,
            idempotency_key: request.idempotency_key,
        })
    }

    /// F08-B6.2: read-after-seq enforcement (the engine primitive).
    ///
    /// A `SharedSequenced` handoff consumer passes the handoff's
    /// `visible_after_seq`; this returns the current commit sequence if that
    /// sequence is visible in this snapshot, and otherwise fails hard with a typed
    /// [`EngineError::SequenceNotVisible`] instead of letting the consumer read
    /// silently-stale state. The HTTP `min_seq` (→ 409) and AQL surface build on
    /// this primitive.
    pub fn require_seq_visible(&self, required_after_seq: CommitSeq) -> EngineResult<CommitSeq> {
        let current = self.current_seq();
        if current >= required_after_seq {
            Ok(current)
        } else {
            Err(EngineError::SequenceNotVisible {
                required: required_after_seq,
                current,
            })
        }
    }
}

fn validate_handoff_view(
    view: &AgentView,
    expected_agent_id: AgentId,
    role: &str,
) -> EngineResult<()> {
    if view.agent_id != expected_agent_id {
        return invalid(&format!(
            "agent handoff {role}_agent_id does not match AgentView"
        ));
    }
    Ok(())
}

fn validate_handoff_sequence(
    current_seq: CommitSeq,
    request: &AgentHandoffRequest,
) -> EngineResult<()> {
    if request.required_after_seq > request.pack_seq {
        return invalid("agent handoff required_after_seq cannot exceed pack_seq");
    }
    if request.pack_seq > current_seq {
        return invalid("agent handoff pack_seq is ahead of current_seq");
    }
    Ok(())
}

fn policy_denied(error: PolicyError) -> EngineError {
    EngineError::AqlBind(BindError::PolicyDenied(error))
}

fn invalid<T>(message: &str) -> EngineResult<T> {
    Err(EngineError::InvalidAgentSession(message.to_owned()))
}
