use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BindError, PolicyError};
use cortex_core::{CellDescriptor, CellId, CommitSeq, KnowledgeCellType};

use crate::database::Database;
use crate::error::{EngineError, EngineResult};
use crate::plan::PolicyRewrite;
use crate::query::{scope_id, CellMetadata};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticCompressionSourceRef {
    pub source_cell_id: CellId,
    pub source_byte_start: usize,
    pub source_byte_end: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticCompressionRequest {
    pub agent_id: AgentId,
    pub scope: String,
    pub summary_cell_id: Option<CellId>,
    pub summary_payload: Vec<u8>,
    pub source_refs: Vec<SemanticCompressionSourceRef>,
    pub answerability_q16: u16,
    pub external_worker: String,
    pub idempotency_key: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticCompressionReport {
    pub agent_id: AgentId,
    pub scope: String,
    pub summary_cell_id: CellId,
    pub committed_seq: CommitSeq,
    pub source_cell_ids: Vec<CellId>,
    pub source_ref_count: usize,
    pub answerability_q16: u16,
    pub provenance_preserved: bool,
    pub auditable: bool,
    pub external_worker: String,
    pub idempotency_key: Option<String>,
}

impl Database {
    pub fn commit_semantic_memory_compression(
        &mut self,
        view: &AgentView,
        request: SemanticCompressionRequest,
    ) -> EngineResult<SemanticCompressionReport> {
        if !self.semantic_compression.enabled {
            return Err(EngineError::FeatureDisabled("semantic_compression"));
        }
        validate_view(view, &request)?;
        self.validate_semantic_compression_request(view, &request)?;

        let summary_cell_id = match request.summary_cell_id {
            Some(cell_id) => cell_id,
            None => self.allocate_cell_id()?,
        };
        let committed_seq = self.put_cell(summary_cell_id, request.summary_payload.clone())?;
        let source_cell_ids = unique_source_cell_ids(&request.source_refs);
        let metadata = CellMetadata::from_payload(&request.summary_payload);
        let provenance_preserved = provenance_preserved(&metadata, &source_cell_ids);
        let auditable = provenance_preserved
            && metadata.compression_answerability_q16 == Some(request.answerability_q16)
            && metadata.compression_worker.as_deref() == Some(request.external_worker.as_str());

        Ok(SemanticCompressionReport {
            agent_id: request.agent_id,
            scope: request.scope,
            summary_cell_id,
            committed_seq,
            source_cell_ids,
            source_ref_count: request.source_refs.len(),
            answerability_q16: request.answerability_q16,
            provenance_preserved,
            auditable,
            external_worker: request.external_worker,
            idempotency_key: request.idempotency_key,
        })
    }

    fn validate_semantic_compression_request(
        &self,
        view: &AgentView,
        request: &SemanticCompressionRequest,
    ) -> EngineResult<()> {
        if request.external_worker.trim().is_empty() {
            return invalid("external_worker is required");
        }
        if request.summary_payload.is_empty() {
            return invalid("summary_payload is required");
        }
        if request.source_refs.is_empty() {
            return invalid("source_refs are required");
        }
        if request.answerability_q16 < self.semantic_compression.min_answerability_q16 {
            return invalid("answerability_q16 is below semantic compression threshold");
        }
        validate_source_refs(&request.source_refs)?;
        self.validate_source_cells_are_readable(view, &request.source_refs)?;
        validate_summary_payload(request)
    }

    fn validate_source_cells_are_readable(
        &self,
        view: &AgentView,
        source_refs: &[SemanticCompressionSourceRef],
    ) -> EngineResult<()> {
        for cell_id in unique_source_cell_ids(source_refs) {
            let descriptor = self
                .memtable
                .read(self.read_txn(), cell_id)
                .map(|version| &version.descriptor)
                .ok_or(cortex_core::CoreError::CellNotFound(cell_id))?;
            if !PolicyRewrite::allows_scope(view, scope_id(&descriptor.scope)) {
                return Err(EngineError::AqlBind(BindError::PolicyDenied(
                    PolicyError::ScopeNotReadable,
                )));
            }
        }
        Ok(())
    }
}

fn validate_view(view: &AgentView, request: &SemanticCompressionRequest) -> EngineResult<()> {
    if view.agent_id != request.agent_id {
        return invalid("semantic compression agent_id does not match AgentView");
    }
    if !view.can_write_scope(scope_id(&request.scope)) {
        return Err(EngineError::AqlBind(BindError::PolicyDenied(
            PolicyError::ScopeNotWritable,
        )));
    }
    Ok(())
}

fn validate_source_refs(source_refs: &[SemanticCompressionSourceRef]) -> EngineResult<()> {
    let mut seen = BTreeSet::new();
    for source_ref in source_refs {
        if source_ref.source_byte_start > source_ref.source_byte_end {
            return invalid("source_ref byte range is invalid");
        }
        seen.insert(source_ref.source_cell_id);
    }
    if seen.is_empty() {
        return invalid("source_refs do not contain source cells");
    }
    Ok(())
}

fn validate_summary_payload(request: &SemanticCompressionRequest) -> EngineResult<()> {
    let descriptor = CellDescriptor::from_payload_lossy(&request.summary_payload);
    if descriptor.scope != request.scope {
        return invalid("summary_payload scope does not match request scope");
    }
    if descriptor.cell_type != KnowledgeCellType::Memory {
        return invalid("summary_payload must be type=memory");
    }
    let metadata = CellMetadata::from_payload(&request.summary_payload);
    if metadata.compression_kind.as_deref() != Some("semantic_summary") {
        return invalid("summary_payload must set compression_kind=semantic_summary");
    }
    if metadata.compression_answerability_q16 != Some(request.answerability_q16) {
        return invalid("summary_payload answerability metadata does not match request");
    }
    if metadata.compression_worker.as_deref() != Some(request.external_worker.as_str()) {
        return invalid("summary_payload worker metadata does not match request");
    }
    let source_cell_ids = unique_source_cell_ids(&request.source_refs);
    if !provenance_preserved(&metadata, &source_cell_ids) {
        return invalid("summary_payload compression_source_cells does not match source_refs");
    }
    Ok(())
}

fn unique_source_cell_ids(source_refs: &[SemanticCompressionSourceRef]) -> Vec<CellId> {
    source_refs
        .iter()
        .map(|source_ref| source_ref.source_cell_id)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn provenance_preserved(metadata: &CellMetadata, source_cell_ids: &[CellId]) -> bool {
    metadata.compression_source_cells == source_cell_ids
}

fn invalid<T>(message: &str) -> EngineResult<T> {
    Err(EngineError::InvalidSemanticCompression(message.to_owned()))
}
