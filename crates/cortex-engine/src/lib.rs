pub mod agent_views;
pub mod bundle;
pub mod checkpoint;
mod cleanup;
pub mod context;
pub mod database;
pub mod distributed;
pub mod error;
pub mod feedback;
pub mod ingestion;
mod lock;
pub mod memory;
pub mod operation;
pub mod query;
pub mod repair;
pub mod replay;
pub mod replication;
pub mod search;
pub mod validation;
pub mod verification;

pub use bundle::{RetiredSegmentGc, SegmentBundle};
pub use context::{
    estimate_tokens, ContextPack, ContextPackAnomaly, ContextPackCell, ContextPackOptions,
};
pub use database::{
    CandidateResolver, CheckpointStats, Database, DatabaseOptions, RecoveryMode, RetrievedCell,
    StaleLockPolicy,
};
pub use distributed::*;
pub use error::{EngineError, EngineResult};
pub use ingestion::{
    extract_pdf_text, CsvIngestOptions, IngestedCell, IngestionJobId, IngestionJobStatus,
    IngestionProgress, IngestionProgressTracker, JsonIngestOptions, PdfExtractionStats,
    PdfIngestOptions, TextIngestOptions,
};
pub use operation::*;
pub use query::{scope_id, CandidateId, CellMetadata, EngineAqlIndex};
pub use repair::RepairReport;
pub use replay::{
    replay_wal, replay_wal_best_effort, replay_wal_best_effort_into, replay_wal_into,
    ReplayMetrics, ReplayResult,
};
pub use replication::*;
pub use search::*;
pub use validation::{StorageStats, StorageValidation, StorageValidationReport};
