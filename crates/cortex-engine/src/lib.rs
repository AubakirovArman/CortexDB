//! Embedded CortexDB engine facade.
//!
//! The stable embedded API is the crate-root facade: `Database`,
//! `DatabaseOptions`, `EngineConfig`, `EngineResult`, error/report structs, and
//! typed operation structs. Implementation modules may remain public for current
//! integration tests and internal tooling, but the documented compatibility
//! boundary is the crate-root facade described in `docs/ENGINE_API.md`.
//!
//! # Example
//!
//! ```
//! use cortex_core::CellId;
//! use cortex_engine::{Database, EngineResult};
//!
//! fn main() -> EngineResult<()> {
//!     let dir = tempfile::tempdir().unwrap();
//!     let mut db = Database::open(dir.path())?;
//!     db.put_cell(CellId(1), b"scope=docs\nstatus=ready\nhello".to_vec())?;
//!     assert_eq!(
//!         db.get_latest_cell(CellId(1)),
//!         Some(b"scope=docs\nstatus=ready\nhello".to_vec())
//!     );
//!     db.close()?;
//!     Ok(())
//! }
//! ```
pub mod agent_views;
pub mod backup;
pub mod bundle;
mod cell_ids;
pub mod checkpoint;
mod cleanup;
pub mod compatibility;
pub mod concurrency;
mod config;
pub mod context;
pub mod database;
mod database_files;
pub mod distributed;
pub mod embedding_pipeline;
pub mod error;
pub mod exec;
pub mod feedback;
pub mod graph;
pub mod graph_retrieval;
pub mod ingestion;
pub mod legal;
mod lock;
pub mod memory;
mod memory_accounting;
pub use memory::{ExpiredMemoryCell, MemoryDecayScore};
pub mod operation;
mod operation_descriptor;
mod options;
pub mod plan;
pub mod query;
pub mod repair;
pub mod replay;
pub mod replication;
mod retrieval_quality;
mod retrieval_rank;
pub mod search;
pub mod session;
pub mod source_trust;
pub mod tool_registry;
pub mod typed_body;
pub mod validation;
pub mod vector_rebuild;
pub mod verification;
pub use backup::{
    BackupDrillReport, BackupReport, BackupRetentionPlan, BackupRetentionReport,
    BackupVerifyReport, EncryptedBackupReport, EncryptedRestoreReport,
    LocalFilesystemOffsiteAdapter, OffsiteBackupAdapter, OffsiteBackupStageReport,
    OffsiteBackupTransferReport, RestoreDryRunReport, RestoreReport,
};
pub use bundle::{RetiredSegmentGc, SegmentBundle};
pub use checkpoint::compactor::{CompactionDecision, CompactionStats};
pub use compatibility::{
    compatibility_summary, ApiCompatibility, CompatibilitySummary, MigrationCompatibility,
    MigrationFormatRegistryEntry, MigrationReleaseRegistryEntry, MigrationVersionRegistry,
    SdkCompatibility, StorageFormatCompatibility,
};
pub use config::{EngineConfig, EngineConfigError};
pub use context::{
    estimate_tokens, estimate_tokens_for_profile, AnswerGroundingOptions, AnswerGroundingReport,
    AnswerGroundingSpan, ContextAccessDecision, ContextAccessDecisionOutcome, ContextCellExplain,
    ContextCellExplainOutcome, ContextExplain, ContextLargeCellPolicy, ContextPack,
    ContextPackAnomaly, ContextPackAnomalyCode, ContextPackCell, ContextPackExportFormat,
    ContextPackOptions, ContextPackWithTools, ContextPipelineCellTrace, ContextPipelineStageTrace,
    ContextPipelineTrace, ContextPipelineVerificationTrace, ContextScoreComponent,
    ContextScoreComponentTrace, ContextSpanProvenance, ContextTokenProfile,
    SourceFreshnessCategory, DEFAULT_CITATION_OVERHEAD_TOKENS, DEFAULT_GROUNDING_THRESHOLD_Q16,
};
pub use database::{
    CandidateResolver, CheckpointStats, Database, PayloadCacheStats, RetrievedCell,
};
pub use distributed::*;
pub use embedding_pipeline::{
    embedding_coverage_report_from_expected_items, embedding_coverage_report_from_files,
    embedding_coverage_report_from_jsonl, embedding_debt_report_from_versions,
    embedding_expected_items_from_versions, embedding_retry_ids_from_expected_items,
    embedding_retry_ids_from_jsonl, embedding_text_hash, EmbeddingBackfillOptions,
    EmbeddingBackfillProvider, EmbeddingBackfillReport, EmbeddingClientConfig,
    EmbeddingCoverageConfig, EmbeddingCoverageReport, EmbeddingDebtItem, EmbeddingDebtReport,
    EmbeddingExpectedItem, QueryEmbeddingProvider, DEFAULT_MIN_EMBEDDING_COVERAGE_BPS,
    EMBEDDING_PIPELINE_REPORT_SCHEMA,
};
pub use error::{EngineError, EngineErrorCategory, EngineErrorCode, EngineResult};
pub use exec::{PhysicalOp, PhysicalOperatorTrace, RetrieveExecutionReport};
pub use graph::{
    GraphEdge, GraphEdgeKind, GraphEntity, GraphSourceRef, KnowledgeGraphIndex, ToolCell,
};
pub use graph_retrieval::{
    GraphRetrievalHit, GraphRetrievalReport, DEFAULT_GRAPH_RETRIEVAL_VISIT_BUDGET,
};
pub use ingestion::{
    count_csv_ingest_cells, count_json_ingest_cells, count_text_chunks, extract_pdf_text,
    split_text_chunks, stable_chunk_id, stable_ingestion_hash_hex, validate_external_ocr_output,
    validate_external_ocr_request, CsvIngestOptions, DigitalPdfTextExtractor,
    DisabledExternalOcrAdapter, DisabledExternalPdfParserAdapter, DuplicateContentGroup,
    EntityIngestOptions, ExternalOcrAdapter, ExternalOcrBoundingBox, ExternalOcrOutput,
    ExternalOcrPageImage, ExternalOcrPageText, ExternalOcrRequest, ExternalOcrTextBlock,
    ExternalPdfParserAdapter, ExternalPdfParserRequest, IngestedCell, IngestionBackpressurePolicy,
    IngestionBackpressureRequest, IngestionJobId, IngestionJobStatus, IngestionProgress,
    IngestionProgressTracker, IngestionSkippedItem, IngestionSourceRefReport,
    IngestionUpdatePolicy, IngestionValidationIssue, IngestionValidationReport, JsonChunkPolicy,
    JsonIngestOptions, NativeDigitalPdfTextExtractor, PdfExtractedPageText, PdfExtractionStats,
    PdfIngestOptions, PdfTextExtractionBoundary, RelationIngestOptions, RememberedCell,
    ScannedPdfOcrRequest, TableChunkPolicy, TextChunk, TextChunkPolicy, TextIngestOptions,
    TextOverlapPolicy,
};
pub use legal::{
    evaluate_legal_report_contract, evaluate_legal_verification_boundary, LegalOutputBoundary,
    LegalRefusalReason, LegalReportContract, LegalReportContractIssue, LegalReportRetention,
    LegalVerificationPolicy, LegalVerificationRequest, LegalVerificationReview,
};
pub use operation::{
    decode_cell_core, decode_cell_id, decoded_operation_from_wal_record,
    decoded_write_batch_marker, descriptor_from_decoded_wal_record, encode_cell_core,
    encode_cell_id, metadata_from_decoded_wal_record, operation_from_decoded_wal_record,
    wal_record_from_operation, wal_record_from_operation_with_metadata,
    wal_record_from_operation_with_seq, wal_record_from_write_batch_begin,
    wal_record_from_write_batch_commit, DbOperation, DbOperationKind, DecodedCellCore,
    DecodedDbOperation, DecodedWriteBatchMarker, OperationDecoder, OperationEncoder, WriteBatch,
    WriteBatchMarker, WriteBatchOperation,
};
pub use options::{
    CompactionPolicy, DatabaseOptions, EngineFeature, EngineFeatureFlags, PayloadResidency,
    RecoveryMode, StaleLockPolicy,
};
pub use plan::{
    CostModelDecision, CostModelEstimate, CostModelOptions, ExecutionPath, LogicalPlan,
    LogicalPlanNode, LogicalPlanReport, PolicyRewrite, ReadSurface, TermDfEstimate, READ_SURFACES,
};
pub use query::{
    scope_id, AqlCandidateCounts, AqlExecutionTraceReport, AqlExplainFilter, AqlExplainReport,
    AqlQueryCacheStats, AqlVerifyExplainReport, CandidateId, CellMetadata, DatabaseStatistics,
    EngineAqlIndex, SourceRef,
};
pub use repair::RepairReport;
pub use replay::{
    replay_wal, replay_wal_best_effort, replay_wal_best_effort_into, replay_wal_into,
    ReplayMetrics, ReplayResult,
};
pub use replication::*;
pub use search::*;
pub use search::{Language, TextAnalyzer, TextAnalyzerConfig};
pub use session::{AgentSession, SessionMemory};
pub use source_trust::{
    parse_source_trust_class, SourceTrust, SourceTrustBasis, SourceTrustCategory, SourceTrustClass,
    DEFAULT_SOURCE_TRUST_Q16, EXTRACTED_SOURCE_TRUST_Q16, INFERRED_SOURCE_TRUST_Q16,
    INTERNAL_SOURCE_TRUST_Q16, OFFICIAL_SOURCE_TRUST_Q16,
};
pub use tool_registry::{RegisteredTool, ToolDescriptor, ToolPermission, ToolRecommendation};
pub use typed_body::{EntityBody, FactBody, RelationBody};
pub use validation::{
    StorageRecoveryAction, StorageStats, StorageValidation, StorageValidationIssue,
    StorageValidationIssueKind, StorageValidationReport,
};
pub use vector_rebuild::VectorRebuildReport;
pub use verification::{
    compare_numeric_values, extract_temporal_query_range, format_scaled_value,
    normalized_numeric_equal, parse_currency_code, parse_magnitude_suffix, parse_temporal_date,
    parse_unit_code, ConflictRecord, ContradictionRelationOptions, Magnitude, NumericComparison,
    NumericValue, TemporalDate, TemporalFactIndex, TemporalFactKey, TemporalFactRecord,
    TemporalQueryRange, TemporalStaleReason, TemporalValidity, VerificationEvidence,
    VerificationGuard, VerificationGuardCode, VerificationMatchKind, VerificationNumericConflict,
    VerificationReport, VerificationReportExportFormat, VerificationStatus,
};
