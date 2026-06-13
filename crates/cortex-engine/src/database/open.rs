use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use cortex_core::memtable::ReadTxn;
use cortex_core::CommitSeq;
use cortex_storage::wal::WalWriter;

use super::Database;
use crate::checkpoint::{load_checkpoint, manifest_path, segments_path};
use crate::cleanup::{cleanup_orphans, remove_lock_file};
use crate::database_files::find_wal_files;
use crate::database_files::truncate_wal_tail;
use crate::error::{EngineError, EngineResult};
use crate::feedback::FeedbackIndex;
use crate::graph::GraphIndexStore;
use crate::lock::DatabaseLock;
use crate::options::{
    DatabaseOptions, EngineFeature, EngineFeatureFlags, RecoveryMode, StaleLockPolicy,
};
use crate::query::cache::AqlQueryCache;
use crate::query::AqlDeltaIndex;
use crate::replay::{replay_wal_best_effort_into, replay_wal_into};
use crate::search::{CorpusSynonymStore, LiveSearchStore, SearchContextStore};
use crate::session::SessionIndex;
use crate::tool_registry::ToolIndex;
use crate::verification::TemporalFactStore;

impl Database {
    /// Open a database at the given path with default options.
    ///
    /// # Example
    ///
    /// ```
    /// # use std::path::Path;
    /// # let dir = tempfile::tempdir().unwrap();
    /// # let path = dir.path();
    /// use cortex_engine::Database;
    /// let db = Database::open(path).unwrap();
    /// ```
    pub fn open(path: impl AsRef<Path>) -> EngineResult<Self> {
        Self::open_with_options(path, DatabaseOptions::default())
    }

    pub fn break_stale_lock(path: impl AsRef<Path>) -> EngineResult<()> {
        remove_lock_file(path.as_ref())
    }

    pub fn feature_flags(&self) -> EngineFeatureFlags {
        self.feature_flags
    }

    pub(crate) fn require_feature(&self, feature: EngineFeature) -> EngineResult<()> {
        if self.feature_flags.is_enabled(feature) {
            Ok(())
        } else {
            Err(EngineError::FeatureDisabled(feature.as_str()))
        }
    }

    pub fn open_with_options(
        path: impl AsRef<Path>,
        options: DatabaseOptions,
    ) -> EngineResult<Self> {
        let root_path = path.as_ref().to_owned();
        std::fs::create_dir_all(&root_path)?;
        let lock = match DatabaseLock::acquire(&root_path) {
            Ok(lock) => lock,
            Err(EngineError::DatabaseAlreadyOpen(_))
                if options.stale_lock_policy == StaleLockPolicy::Break =>
            {
                Self::break_stale_lock(&root_path)?;
                DatabaseLock::acquire(&root_path)?
            }
            Err(error) => return Err(error),
        };
        cleanup_orphans(&root_path)?;
        let wal_path = root_path.join("db.aclog");
        let manifest_path = manifest_path(&root_path);
        let segments_path = segments_path(&root_path);
        let checkpoint = load_checkpoint(&root_path)?;
        let wal_files = find_wal_files(&wal_path);
        let mut current_memtable = checkpoint.memtable;
        let mut current_seq = CommitSeq(checkpoint.manifest.checkpoint_seq);
        let mut last_safe_offset = 0;

        for file in &wal_files {
            let replay = match options.recovery_mode {
                RecoveryMode::Strict => replay_wal_into(file, current_memtable, current_seq)?,
                RecoveryMode::BestEffort => {
                    replay_wal_best_effort_into(file, current_memtable, current_seq)?
                }
            };
            current_memtable = replay.memtable;
            current_seq = replay.last_seq;
            if file == &wal_path {
                last_safe_offset = replay.safe_truncate_offset;
            }
        }

        truncate_wal_tail(&wal_path, last_safe_offset)?;
        let writer = WalWriter::start(&wal_path, options.durability_mode)?;
        let aql_delta_index = AqlDeltaIndex::from_memtable_after(
            &current_memtable,
            ReadTxn::at(current_seq),
            CommitSeq(checkpoint.manifest.checkpoint_seq),
        );
        let feedback_index =
            FeedbackIndex::from_memtable(&current_memtable, ReadTxn::at(current_seq));
        let graph_index_store =
            GraphIndexStore::from_memtable(&current_memtable, ReadTxn::at(current_seq));
        let live_search_store =
            LiveSearchStore::from_memtable(&current_memtable, ReadTxn::at(current_seq));
        let search_context_store =
            SearchContextStore::from_memtable(&current_memtable, ReadTxn::at(current_seq));
        let corpus_synonym_store =
            CorpusSynonymStore::from_memtable(&current_memtable, ReadTxn::at(current_seq));
        let session_index =
            SessionIndex::from_memtable(&current_memtable, ReadTxn::at(current_seq));
        let temporal_fact_store =
            TemporalFactStore::from_memtable(&current_memtable, ReadTxn::at(current_seq));
        let tool_index = ToolIndex::from_memtable(&current_memtable, ReadTxn::at(current_seq));
        let database = Self {
            root_path,
            wal_path,
            manifest_path,
            segments_path,
            manifest: checkpoint.manifest,
            memtable: current_memtable,
            writer,
            current_seq,
            durability_mode: options.durability_mode,
            hnsw_build_config: options.hnsw_build_config.normalized(),
            feature_flags: options.feature_flags,
            ingestion_backpressure_policy: options.ingestion_backpressure,
            ingestion_rate_state: crate::ingestion::default_ingestion_rate_state(),
            aql_query_cache: Mutex::new(AqlQueryCache::default()),
            aql_delta_index,
            corpus_synonym_store,
            feedback_index,
            graph_index_store,
            live_search_store,
            search_context_store,
            session_index,
            temporal_fact_store,
            tool_index,
            persisted_index_cache: Mutex::new(None),
            active_read_pins: Arc::new(Mutex::new(BTreeMap::new())),
            compaction_policy: options.compaction_policy,
            _lock: lock,
            closed: false,
        };
        database.resume_interrupted_ingestion_jobs()?;
        Ok(database)
    }
}
