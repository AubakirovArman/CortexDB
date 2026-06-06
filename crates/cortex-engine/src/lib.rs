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
pub mod checkpoint;
mod cleanup;
pub mod compatibility;
mod config;
pub mod context;
pub mod database;
mod database_files;
pub mod distributed;
pub mod error;
pub mod feedback;
pub mod graph;
pub mod ingestion;
pub mod legal;
mod lock;
pub mod memory;
mod memory_accounting;
pub use memory::{ExpiredMemoryCell, MemoryDecayScore};
pub mod operation;
mod options;
pub mod query;
pub mod repair;
pub mod replay;
pub mod replication;
pub mod search;
pub mod source_trust;
pub mod tool_registry;
pub mod typed_body;
pub mod validation;
pub mod vector_rebuild;
pub mod verification;

pub use backup::{
    BackupDrillReport, BackupReport, BackupRetentionPlan, BackupRetentionReport,
    EncryptedBackupReport, EncryptedRestoreReport, LocalFilesystemOffsiteAdapter,
    OffsiteBackupAdapter, OffsiteBackupStageReport, OffsiteBackupTransferReport,
    RestoreDryRunReport, RestoreReport,
};
pub use bundle::{RetiredSegmentGc, SegmentBundle};
pub use compatibility::{
    compatibility_summary, ApiCompatibility, CompatibilitySummary, MigrationCompatibility,
    SdkCompatibility, StorageFormatCompatibility,
};
pub use config::{EngineConfig, EngineConfigError};
pub use context::{
    estimate_tokens, estimate_tokens_for_profile, ContextExplain, ContextPack, ContextPackAnomaly,
    ContextPackAnomalyCode, ContextPackCell, ContextPackExportFormat, ContextPackOptions,
    ContextScoreComponent, ContextTokenProfile, DEFAULT_CITATION_OVERHEAD_TOKENS,
};
pub use database::{CandidateResolver, CheckpointStats, Database, RetrievedCell};
pub use distributed::*;
pub use error::{EngineError, EngineErrorCategory, EngineErrorCode, EngineResult};
pub use graph::{GraphEdge, GraphEntity, GraphSourceRef, KnowledgeGraphIndex, ToolCell};
pub use ingestion::{
    extract_pdf_text, split_text_chunks, stable_chunk_id, validate_external_ocr_request,
    CsvIngestOptions, DigitalPdfTextExtractor, DisabledExternalOcrAdapter, EntityIngestOptions,
    ExternalOcrAdapter, ExternalOcrOutput, ExternalOcrPageImage, ExternalOcrPageText,
    ExternalOcrRequest, IngestedCell, IngestionJobId, IngestionJobStatus, IngestionProgress,
    IngestionProgressTracker, IngestionSkippedItem, IngestionSourceRefReport,
    IngestionValidationIssue, IngestionValidationReport, JsonIngestOptions,
    NativeDigitalPdfTextExtractor, PdfExtractionStats, PdfIngestOptions, PdfTextExtractionBoundary,
    RelationIngestOptions, RememberedCell, TextChunk, TextChunkPolicy, TextIngestOptions,
};
pub use legal::{
    evaluate_legal_report_contract, evaluate_legal_verification_boundary, LegalOutputBoundary,
    LegalRefusalReason, LegalReportContract, LegalReportContractIssue, LegalReportRetention,
    LegalVerificationPolicy, LegalVerificationRequest, LegalVerificationReview,
};
pub use operation::{
    decode_cell_core, decode_cell_id, decoded_operation_from_wal_record, encode_cell_core,
    encode_cell_id, metadata_from_decoded_wal_record, operation_from_decoded_wal_record,
    wal_record_from_operation, wal_record_from_operation_with_metadata,
    wal_record_from_operation_with_seq, DbOperation, DbOperationKind, DecodedCellCore,
    DecodedDbOperation, OperationDecoder, OperationEncoder,
};
pub use options::{
    DatabaseOptions, EngineFeature, EngineFeatureFlags, RecoveryMode, StaleLockPolicy,
};
pub use query::{
    scope_id, AqlCandidateCounts, AqlExplainFilter, AqlExplainReport, AqlQueryCacheStats,
    CandidateId, CellMetadata, EngineAqlIndex,
};
pub use repair::RepairReport;
pub use replay::{
    replay_wal, replay_wal_best_effort, replay_wal_best_effort_into, replay_wal_into,
    ReplayMetrics, ReplayResult,
};
pub use replication::*;
pub use search::*;
pub use source_trust::{SourceTrust, SourceTrustCategory, DEFAULT_SOURCE_TRUST_Q16};
pub use tool_registry::{RegisteredTool, ToolDescriptor, ToolPermission};
pub use typed_body::{EntityBody, FactBody, RelationBody};
pub use validation::{StorageStats, StorageValidation, StorageValidationReport};
pub use vector_rebuild::VectorRebuildReport;
pub use verification::{
    format_scaled_value, ContradictionRelationOptions, Magnitude, NumericValue,
    VerificationEvidence, VerificationGuard, VerificationGuardCode, VerificationNumericConflict,
    VerificationReport, VerificationReportExportFormat, VerificationStatus,
};
