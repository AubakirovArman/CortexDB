use cortex_aql::{AgentId, AgentView, BindError, PolicyError, ScopeId};
use cortex_core::{CellId, CommitSeq, KnowledgeCell, KnowledgeCellMetadata, KnowledgeCellType};
use cortex_crypto::hex_lower;

use crate::cell_ids::{agent_cell_id_slot, namespaced_agent_cell_id};
use crate::database::Database;
use crate::error::{EngineError, EngineResult};
use crate::idempotency::decode_hex;
use crate::plan::PolicyRewrite;
use crate::query::scope_id;

const HANDOFF_CELL_NAMESPACE: u64 = 0xc000_0000_0000_0000;
const HANDOFF_LEDGER_SCOPE: &str = "__cortex_handoff__";

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

/// F08-B6.1: a handoff report plus where it was durably recorded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommittedAgentHandoff {
    pub report: AgentHandoffReport,
    pub handoff_cell_id: CellId,
    pub committed_seq: CommitSeq,
}

fn encode_handoff(report: &AgentHandoffReport) -> Vec<u8> {
    // Every free-form field is hex-encoded so arbitrary bytes can't break parsing.
    format!(
        "source_agent_id={}\ntarget_agent_id={}\nscope_hex={}\npack_hash_hex={}\npack_seq={}\nrequired_after_seq={}\nvisible_after_seq={}\ntarget_can_read={}\nidempotency_key_hex={}\n",
        report.source_agent_id.0,
        report.target_agent_id.0,
        hex_lower(report.scope.as_bytes()),
        hex_lower(report.pack_hash.as_bytes()),
        report.pack_seq.0,
        report.required_after_seq.0,
        report.visible_after_seq.0,
        report.target_can_read,
        report
            .idempotency_key
            .as_deref()
            .map(|key| hex_lower(key.as_bytes()))
            .unwrap_or_default(),
    )
    .into_bytes()
}

fn parse_handoff(payload: &[u8]) -> Option<AgentHandoffReport> {
    let text = std::str::from_utf8(payload).ok()?;
    let mut source = None;
    let mut target = None;
    let mut scope = None;
    let mut pack_hash = None;
    let mut pack_seq = None;
    let mut required_after_seq = None;
    let mut visible_after_seq = None;
    let mut target_can_read = None;
    let mut idempotency_key = None;
    let mut saw_idempotency_field = false;
    for line in text.lines() {
        let Some((field, value)) = line.split_once('=') else {
            continue;
        };
        match field {
            "source_agent_id" => source = value.parse::<u64>().ok().map(AgentId),
            "target_agent_id" => target = value.parse::<u64>().ok().map(AgentId),
            "scope_hex" => {
                scope = decode_hex(value).and_then(|bytes| String::from_utf8(bytes).ok())
            }
            "pack_hash_hex" => {
                pack_hash = decode_hex(value).and_then(|bytes| String::from_utf8(bytes).ok())
            }
            "pack_seq" => pack_seq = value.parse::<u64>().ok().map(CommitSeq),
            "required_after_seq" => required_after_seq = value.parse::<u64>().ok().map(CommitSeq),
            "visible_after_seq" => visible_after_seq = value.parse::<u64>().ok().map(CommitSeq),
            "target_can_read" => target_can_read = value.parse::<bool>().ok(),
            "idempotency_key_hex" => {
                saw_idempotency_field = true;
                if !value.is_empty() {
                    idempotency_key =
                        decode_hex(value).and_then(|bytes| String::from_utf8(bytes).ok());
                }
            }
            _ => {}
        }
    }
    // A well-formed record must carry the idempotency field (possibly empty).
    saw_idempotency_field.then_some(())?;
    let scope = scope?;
    let scope_id = scope_id(&scope);
    Some(AgentHandoffReport {
        level: MemoryConsistencyLevel::SharedSequenced,
        source_agent_id: source?,
        target_agent_id: target?,
        scope,
        scope_id,
        pack_hash: pack_hash?,
        pack_seq: pack_seq?,
        required_after_seq: required_after_seq?,
        visible_after_seq: visible_after_seq?,
        target_can_read: target_can_read?,
        idempotency_key,
    })
}

fn handoff_id_overflow() -> EngineError {
    EngineError::StorageInvariant("agent handoff cell id space is exhausted".to_owned())
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

    /// F08-B6.1: commit a `SharedSequenced` handoff to a durable, auditable ledger.
    ///
    /// Runs the same validation as [`Database::plan_agent_handoff`], then persists
    /// the resulting report as a cell in a reserved namespace (`0xc`) with a scope
    /// no `AgentView` can read — so the record is durable and re-readable via
    /// [`Database::read_agent_handoff`] but never surfaces in retrieval and never
    /// appears in a golden (the write only happens under the default-off
    /// `agent_transactions` feature).
    pub fn commit_agent_handoff(
        &mut self,
        source_view: &AgentView,
        target_view: &AgentView,
        request: AgentHandoffRequest,
    ) -> EngineResult<CommittedAgentHandoff> {
        if !self.agent_transactions.enabled {
            return Err(EngineError::FeatureDisabled("agent_transactions"));
        }
        let report = self.plan_agent_handoff(source_view, target_view, request)?;
        let handoff_cell_id = self.next_handoff_cell_id(report.source_agent_id)?;
        let cell = KnowledgeCell::new(
            KnowledgeCellMetadata {
                scope: HANDOFF_LEDGER_SCOPE.to_owned(),
                status: "ready".to_owned(),
                cell_type: KnowledgeCellType::Feedback,
                memory_type: None,
                ttl_seconds: None,
                created_unix_seconds: None,
                source_trust_q16: None,
                source: None,
            },
            encode_handoff(&report),
        );
        let committed_seq = self.put_knowledge_cell(handoff_cell_id, cell)?;
        Ok(CommittedAgentHandoff {
            report,
            handoff_cell_id,
            committed_seq,
        })
    }

    /// F08-B6.1: read a persisted handoff record back for audit. Returns `None`
    /// when no cell exists or the cell is not a well-formed handoff record.
    pub fn read_agent_handoff(&self, cell_id: CellId) -> EngineResult<Option<AgentHandoffReport>> {
        Ok(self
            .get_latest_cell(cell_id)
            .and_then(|payload| parse_handoff(&payload)))
    }

    fn next_handoff_cell_id(&self, agent_id: AgentId) -> EngineResult<CellId> {
        let agent_slot = agent_cell_id_slot(agent_id).ok_or_else(handoff_id_overflow)?;
        let mut sequence = self
            .current_seq()
            .0
            .checked_add(1)
            .ok_or_else(handoff_id_overflow)?;
        // Probe for a free id, mirroring feedback/session allocation.
        let mut attempts = 0u64;
        loop {
            let cell_id = namespaced_agent_cell_id(HANDOFF_CELL_NAMESPACE, agent_slot, sequence)
                .ok_or_else(handoff_id_overflow)?;
            if self.get_latest_cell_descriptor(cell_id).is_none() {
                return Ok(cell_id);
            }
            attempts = attempts.checked_add(1).ok_or_else(handoff_id_overflow)?;
            if attempts > u64::from(u32::MAX) {
                return Err(handoff_id_overflow());
            }
            sequence = sequence.checked_add(1).ok_or_else(handoff_id_overflow)?;
        }
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
