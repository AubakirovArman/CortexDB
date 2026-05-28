use cortex_aql::{parse_aql, AgentId, AgentView, Binder, BoundPlan, BoundRememberPlan, MemoryType};
use cortex_core::{CellId, CommitSeq, KnowledgeCell, KnowledgeCellMetadata, KnowledgeCellType};

use crate::database::Database;
use crate::error::{EngineError, EngineResult};

mod adapters;
mod pdf;
mod progress;

pub use adapters::{
    CsvIngestOptions, EntityIngestOptions, IngestedCell, JsonIngestOptions, PdfIngestOptions,
    RelationIngestOptions, TextIngestOptions,
};
pub use pdf::{extract_pdf_text, PdfExtractionStats};
pub use progress::{
    IngestionJobId, IngestionJobStatus, IngestionProgress, IngestionProgressTracker,
};

const MEMORY_CELL_NAMESPACE: u64 = 0x8000_0000_0000_0000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RememberedCell {
    pub cell_id: CellId,
    pub commit_seq: CommitSeq,
    pub ttl_seconds: Option<u64>,
}

impl Database {
    pub fn put_knowledge_cell(
        &mut self,
        cell_id: CellId,
        cell: KnowledgeCell,
    ) -> EngineResult<CommitSeq> {
        let metadata = cell.metadata.encode_wal_section();
        self.append_then_apply_with_metadata(
            crate::operation::DbOperation::PutCell {
                cell_id,
                payload: cell.encode_payload(),
            },
            metadata,
        )
    }

    pub fn remember_aql(&mut self, aql: &str, view: &AgentView) -> EngineResult<RememberedCell> {
        let statement = parse_aql(aql).map_err(|error| EngineError::AqlParse(error.to_string()))?;
        let index = self.try_aql_index()?;
        let bound = Binder::new(&index, view).bind_statement(&statement)?;
        let BoundPlan::Remember(plan) = bound else {
            return Err(EngineError::InvalidOperation);
        };
        let BoundRememberPlan {
            scope_name,
            memory_type,
            content,
            ttl_seconds,
            ..
        } = *plan;
        let cell_id = self.next_memory_cell_id(view.agent_id)?;
        let cell = KnowledgeCell::new(
            KnowledgeCellMetadata {
                scope: scope_name,
                status: "ready".to_owned(),
                cell_type: KnowledgeCellType::Memory,
                memory_type: Some(memory_type_name(memory_type).to_owned()),
                ttl_seconds,
                created_unix_seconds: Some(unix_now()),
                source_trust_q16: None,
                source: Some(format!("agent:{}", view.agent_id.0)),
            },
            content.into_bytes(),
        );
        let commit_seq = self.put_knowledge_cell(cell_id, cell)?;
        Ok(RememberedCell {
            cell_id,
            commit_seq,
            ttl_seconds,
        })
    }

    fn next_memory_cell_id(&self, agent_id: AgentId) -> EngineResult<CellId> {
        let agent_bits = (agent_id.0 & 0x7fff_ffff) << 32;
        let mut sequence = self
            .current_seq
            .0
            .checked_add(1)
            .ok_or_else(memory_id_overflow)?;
        let mut attempts = 0u64;
        loop {
            let cell_id = CellId(MEMORY_CELL_NAMESPACE | agent_bits | (sequence & 0xffff_ffff));
            if self.get_latest_cell(cell_id).is_none() {
                return Ok(cell_id);
            }
            attempts = attempts.checked_add(1).ok_or_else(memory_id_overflow)?;
            if attempts > u64::from(u32::MAX) {
                return Err(memory_id_overflow());
            }
            sequence = sequence.checked_add(1).ok_or_else(memory_id_overflow)?;
        }
    }
}

fn memory_type_name(memory_type: MemoryType) -> &'static str {
    match memory_type {
        MemoryType::Decision => "decision",
        MemoryType::Preference => "preference",
        MemoryType::WorkflowResult => "workflow_result",
        MemoryType::ErrorLog => "error_log",
        MemoryType::Observation => "observation",
    }
}

fn memory_id_overflow() -> EngineError {
    EngineError::StorageInvariant("memory cell id space is exhausted".to_owned())
}

fn unix_now() -> u64 {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => duration.as_secs(),
        Err(_) => 0,
    }
}
