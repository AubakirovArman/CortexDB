//! Embedded database facade.
//!
//! `Database` is the stable entrypoint for opening a local CortexDB store,
//! writing cells, reading cells, checkpointing, compacting, and validating
//! storage.
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
mod candidates;
mod open;
mod payload_cache;
mod payload_read;
mod read;
mod rebuild;
mod stores;
#[cfg(test)]
mod tests;
mod types;
mod write;
pub(crate) use crate::retrieval_rank::{
    diversify_retrieved_cells, expand_parent_context, rank_retrieved_cells_with_window,
    suppress_duplicate_content,
};
pub use types::{
    CapturedAccessDecision, CapturedAccessDenial, CapturedAccessDenialSet, CheckpointStats,
    Database, PinnedReadTxn, RetrievedCell,
};
pub use {candidates::CandidateResolver, payload_cache::PayloadCacheStats};
