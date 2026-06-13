use cortex_aql::{AgentId, AgentView, BoundPlan, BoundRememberPlan, MemoryType};
use cortex_core::{CellId, CommitSeq, KnowledgeCell, KnowledgeCellMetadata, KnowledgeCellType};

use crate::database::Database;
use crate::error::{EngineError, EngineResult};
use crate::query::cache::AqlStatementKind;

mod adapters;
pub(crate) mod backpressure;
mod cells;
mod chunking;
mod dedup;
mod enrichment;
mod formats;
mod jobs;
mod pdf;
mod pdf_contracts;
mod pdf_ingest;
mod progress;
mod report;

pub use adapters::{
    CsvIngestOptions, EntityIngestOptions, IngestedCell, JsonIngestOptions, PdfIngestOptions,
    RelationIngestOptions, TextIngestOptions,
};
pub(crate) use backpressure::{default_ingestion_rate_state, IngestionRateState};
pub use backpressure::{IngestionBackpressurePolicy, IngestionBackpressureRequest};
pub use chunking::{
    split_text_chunks, stable_chunk_id, JsonChunkPolicy, TableChunkPolicy, TextChunk,
    TextChunkPolicy, TextOverlapPolicy,
};
pub use dedup::{stable_ingestion_hash_hex, DuplicateContentGroup, IngestionUpdatePolicy};
pub use pdf::{extract_pdf_text, PdfExtractedPageText, PdfExtractionStats};
pub use pdf_contracts::{
    validate_external_ocr_output, validate_external_ocr_request, DigitalPdfTextExtractor,
    DisabledExternalOcrAdapter, DisabledExternalPdfParserAdapter, ExternalOcrAdapter,
    ExternalOcrBoundingBox, ExternalOcrOutput, ExternalOcrPageImage, ExternalOcrPageText,
    ExternalOcrRequest, ExternalOcrTextBlock, ExternalPdfParserAdapter, ExternalPdfParserRequest,
    NativeDigitalPdfTextExtractor, PdfTextExtractionBoundary, ScannedPdfOcrRequest,
};
pub use progress::{
    IngestionJobId, IngestionJobStatus, IngestionProgress, IngestionProgressTracker,
};
pub use report::{
    IngestionSkippedItem, IngestionSourceRefReport, IngestionValidationIssue,
    IngestionValidationReport,
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

    /// Execute a `REMEMBER` AQL statement, creating a durable memory cell.
    ///
    /// # Example
    ///
    /// ```
    /// # use cortex_engine::Database;
    /// # use cortex_aql::{AgentId, AgentView, MemoryType, ScopeId};
    /// # let dir = tempfile::tempdir().unwrap();
    /// # let mut db = Database::open(dir.path()).unwrap();
    /// let view = AgentView {
    ///     agent_id: AgentId(1),
    ///     label: None,
    ///     readable_brains: Default::default(),
    ///     readable_scopes: Default::default(),
    ///     writable_scopes: [ScopeId(841510221546309118)].into_iter().collect(),
    ///     allowed_modes: Default::default(),
    ///     allowed_memory_types: [MemoryType::Decision].into_iter().collect(),
    ///     max_context_budget_tokens: 10000,
    ///     default_context_budget_tokens: 1000,
    ///     max_candidate_limit: 100,
    ///     default_candidate_limit: 10,
    ///     min_required_confidence_q16: Default::default(),
    ///     max_ttl_seconds: None,
    ///     allow_remember: true,
    ///     allow_verify_fact: false,
    ///     allow_audit_mode: false,
    ///     require_citations_by_default: false,
    ///     private_scope: None,
    /// };
    /// let result = db.remember_aql(r#"REMEMBER "use conservative budget" IN SCOPE default AS TYPE decision TTL 3600 SECONDS;"#, &view);
    /// assert!(result.is_ok(), "{}", result.unwrap_err());
    /// ```
    pub fn remember_aql(&mut self, aql: &str, view: &AgentView) -> EngineResult<RememberedCell> {
        let (cached, _) = self.bind_aql_cached(aql, view)?;
        if cached.statement_kind != AqlStatementKind::Remember {
            return Err(EngineError::InvalidOperation);
        }
        let BoundPlan::Remember(plan) = cached.bound_plan else {
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
            if self.get_latest_cell_descriptor(cell_id).is_none() {
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
