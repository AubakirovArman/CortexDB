use cortex_aql::{AgentId, AgentView, BindError, PolicyError};
use cortex_core::{CellDescriptor, CellId, CommitSeq};

use crate::database::Database;
use crate::error::{EngineError, EngineResult};
use crate::operation::{WriteBatch, WriteBatchOperation};
use crate::query::scope_id;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentTransactionRequest {
    pub agent_id: AgentId,
    pub scope: String,
    pub base_seq: CommitSeq,
    pub batch: WriteBatch,
    pub idempotency_key: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentTransactionOutcome {
    Committed,
    Conflict,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentTransactionConflictKind {
    StaleCell,
    TombstonedCell,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentTransactionConflict {
    pub cell_id: CellId,
    pub base_seq: CommitSeq,
    pub observed_seq: CommitSeq,
    pub kind: AgentTransactionConflictKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentTransactionReport {
    pub outcome: AgentTransactionOutcome,
    pub agent_id: AgentId,
    pub scope: String,
    pub base_seq: CommitSeq,
    pub committed_seq: Option<CommitSeq>,
    pub conflicts: Vec<AgentTransactionConflict>,
    pub idempotency_key: Option<String>,
    /// True when this outcome was replayed from the idempotency ledger (F04-B1.3)
    /// rather than freshly executed — the write was not repeated.
    pub idempotent_replay: bool,
}

impl Database {
    pub fn commit_agent_transaction(
        &mut self,
        view: &AgentView,
        request: AgentTransactionRequest,
    ) -> EngineResult<AgentTransactionReport> {
        if !self.agent_transactions.enabled {
            return Err(EngineError::FeatureDisabled("agent_transactions"));
        }
        validate_agent_transaction_view(view, &request)?;
        self.validate_agent_transaction_scope(&request)?;

        // F04-B1.3: consult the durable idempotency ledger BEFORE any conflict
        // check or write. A replay short-circuits with the recorded outcome; a
        // reused key with a different request is rejected.
        let idempotency_insert = match &request.idempotency_key {
            Some(key) => {
                let digest = crate::idempotency::request_digest(
                    request.agent_id,
                    &request.scope,
                    &request.batch,
                );
                match self.resolve_idempotency(request.agent_id, key, &digest)? {
                    crate::idempotency::LedgerResolution::Replay { committed_seq } => {
                        return Ok(AgentTransactionReport {
                            outcome: AgentTransactionOutcome::Committed,
                            agent_id: request.agent_id,
                            scope: request.scope,
                            base_seq: request.base_seq,
                            committed_seq: Some(committed_seq),
                            conflicts: Vec::new(),
                            idempotency_key: request.idempotency_key,
                            idempotent_replay: true,
                        });
                    }
                    crate::idempotency::LedgerResolution::Conflict => {
                        return Err(EngineError::InvalidAgentSession(
                            "idempotency key reused with a different request".to_owned(),
                        ));
                    }
                    crate::idempotency::LedgerResolution::Fresh { insert_cell_id } => {
                        Some((key.clone(), digest, insert_cell_id))
                    }
                }
            }
            None => None,
        };

        let conflicts = self.agent_transaction_conflicts(&request);
        if !conflicts.is_empty() {
            return Ok(AgentTransactionReport {
                outcome: AgentTransactionOutcome::Conflict,
                agent_id: request.agent_id,
                scope: request.scope,
                base_seq: request.base_seq,
                committed_seq: None,
                conflicts,
                idempotency_key: request.idempotency_key,
                idempotent_replay: false,
            });
        }

        let agent_id = request.agent_id;
        let scope = request.scope.clone();
        let base_seq = request.base_seq;
        let idempotency_key = request.idempotency_key.clone();
        let operation_count = u64::try_from(request.batch.len()).map_err(|_| {
            EngineError::StorageInvariant("agent transaction batch is too large".to_owned())
        })?;
        let committed_seq = CommitSeq(
            self.current_seq()
                .0
                .checked_add(operation_count)
                .ok_or_else(|| {
                    EngineError::StorageInvariant(
                        "agent transaction commit sequence overflow".to_owned(),
                    )
                })?,
        );
        let mut operations = request.batch.into_operations();

        // The ledger entry is the final operation in the SAME WAL batch as the
        // guarded writes. Recovery therefore exposes both or neither; there is
        // no crash window where a committed mutation can be retried as fresh.
        if let Some((key, digest, insert_cell_id)) = idempotency_insert {
            operations.push(crate::idempotency::idempotency_ledger_operation(
                insert_cell_id,
                agent_id,
                &key,
                &digest,
                committed_seq,
            ));
        }
        self.write_batch(WriteBatch::from_operations(operations))?;

        Ok(AgentTransactionReport {
            outcome: AgentTransactionOutcome::Committed,
            agent_id,
            scope,
            base_seq,
            committed_seq: Some(committed_seq),
            conflicts: Vec::new(),
            idempotency_key,
            idempotent_replay: false,
        })
    }

    fn validate_agent_transaction_scope(
        &self,
        request: &AgentTransactionRequest,
    ) -> EngineResult<()> {
        if request.base_seq > self.current_seq() {
            return Err(EngineError::InvalidAgentSession(
                "agent transaction base_seq is ahead of current_seq".to_owned(),
            ));
        }
        for operation in request.batch.operations() {
            match operation {
                WriteBatchOperation::PutCell { payload, .. }
                | WriteBatchOperation::PatchCell { payload, .. } => {
                    validate_payload_scope(payload, &request.scope)?;
                }
                WriteBatchOperation::TombstoneCell { cell_id } => {
                    let descriptor = self
                        .memtable
                        .read(self.read_txn(), *cell_id)
                        .map(|version| &version.descriptor)
                        .ok_or(cortex_core::CoreError::CellNotFound(*cell_id))?;
                    validate_descriptor_scope(descriptor, &request.scope)?;
                }
            }
        }
        Ok(())
    }

    fn agent_transaction_conflicts(
        &self,
        request: &AgentTransactionRequest,
    ) -> Vec<AgentTransactionConflict> {
        let mut conflicts = Vec::new();
        for cell_id in transaction_target_cells(&request.batch) {
            if let Some(version) = self.memtable.read(self.read_txn(), cell_id) {
                if version.created_seq > request.base_seq {
                    conflicts.push(AgentTransactionConflict {
                        cell_id,
                        base_seq: request.base_seq,
                        observed_seq: version.created_seq,
                        kind: AgentTransactionConflictKind::StaleCell,
                    });
                    continue;
                }
            }
            if let Some((_, deleted_seq)) = self
                .memtable
                .tombstones_after(request.base_seq)
                .into_iter()
                .find(|(deleted_cell_id, _)| *deleted_cell_id == cell_id)
            {
                conflicts.push(AgentTransactionConflict {
                    cell_id,
                    base_seq: request.base_seq,
                    observed_seq: deleted_seq,
                    kind: AgentTransactionConflictKind::TombstonedCell,
                });
            }
        }
        conflicts
    }
}

fn validate_agent_transaction_view(
    view: &AgentView,
    request: &AgentTransactionRequest,
) -> EngineResult<()> {
    if view.agent_id != request.agent_id {
        return Err(EngineError::InvalidAgentSession(
            "agent transaction agent_id does not match AgentView".to_owned(),
        ));
    }
    if !view.can_write_scope(scope_id(&request.scope)) {
        return Err(EngineError::AqlBind(BindError::PolicyDenied(
            PolicyError::ScopeNotWritable,
        )));
    }
    Ok(())
}

fn validate_payload_scope(payload: &[u8], expected_scope: &str) -> EngineResult<()> {
    let descriptor = CellDescriptor::from_payload_lossy(payload);
    validate_descriptor_scope(&descriptor, expected_scope)
}

fn validate_descriptor_scope(
    descriptor: &CellDescriptor,
    expected_scope: &str,
) -> EngineResult<()> {
    if descriptor.scope != expected_scope {
        return Err(EngineError::InvalidAgentSession(format!(
            "agent transaction operation scope {:?} does not match transaction scope {:?}",
            descriptor.scope, expected_scope
        )));
    }
    Ok(())
}

fn transaction_target_cells(batch: &WriteBatch) -> Vec<CellId> {
    let mut cells = batch
        .operations()
        .iter()
        .map(|operation| match operation {
            WriteBatchOperation::PutCell { cell_id, .. }
            | WriteBatchOperation::PatchCell { cell_id, .. }
            | WriteBatchOperation::TombstoneCell { cell_id } => *cell_id,
        })
        .collect::<Vec<_>>();
    cells.sort();
    cells.dedup();
    cells
}
