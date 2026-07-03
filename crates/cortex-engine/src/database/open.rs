use std::collections::BTreeMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use cortex_core::memtable::ReadTxn;
use cortex_core::CommitSeq;
use cortex_storage::wal::WalWriter;

use super::payload_cache::SegmentPayloadCache;
use super::stores::DerivedStores;
use super::Database;
use crate::checkpoint::{load_checkpoint, manifest_path, segments_path};
use crate::cleanup::{cleanup_orphans, remove_lock_file};
use crate::database_files::find_wal_files;
use crate::database_files::truncate_wal_tail;
use crate::error::{EngineError, EngineResult};
use crate::lock::DatabaseLock;
use crate::options::{
    AgentTransactionOptions, DatabaseOptions, EngineFeature, EngineFeatureFlags,
    LearnedRankingOptions, PayloadResidency, RecoveryMode, SemanticCompressionOptions,
    StaleLockPolicy, TieredStorageOptions,
};
use crate::query::cache::AqlQueryCache;
use crate::query::AqlDeltaIndex;
use crate::replay::{replay_wal_best_effort_into, replay_wal_into};
use crate::search::TextAnalyzerConfig;

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

    pub fn payload_residency(&self) -> PayloadResidency {
        self.payload_residency
    }

    pub fn tiered_storage_options(&self) -> TieredStorageOptions {
        self.tiered_storage
    }

    pub fn agent_transaction_options(&self) -> AgentTransactionOptions {
        self.agent_transactions
    }

    pub fn learned_ranking_options(&self) -> LearnedRankingOptions {
        self.learned_ranking
    }

    pub fn semantic_compression_options(&self) -> SemanticCompressionOptions {
        self.semantic_compression
    }

    /// A3.3: the current ANN serving epoch for the receipt's determinism input.
    /// `None` when guarded sampling is off (default) or unarmed — keeping the
    /// signed determinism surface byte-identical to pre-A3.3.
    pub(crate) fn current_ann_serving_epoch(&self) -> Option<u64> {
        if !self.ann_guarded_sampling.enabled {
            return None;
        }
        self.guarded_recall
            .lock()
            .ok()?
            .as_ref()
            .map(|state| state.serving_epoch())
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
        let checkpoint = load_checkpoint(&root_path, options.payload_residency)?;
        ensure_text_analyzer_profile(&checkpoint.manifest, options.text_analyzer)?;
        ensure_embedding_profile(&checkpoint.manifest, options.embedding_profile.as_ref())?;
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
        let stores = DerivedStores::from_memtable_for_residency(
            &current_memtable,
            ReadTxn::at(current_seq),
            options.payload_residency,
        );
        let mut database = Self {
            root_path,
            wal_path,
            manifest_path,
            segments_path,
            manifest: checkpoint.manifest,
            memtable: current_memtable,
            writer,
            current_seq,
            durability_mode: options.durability_mode,
            wal_archive_enabled: options.wal_archive_enabled,
            wal_archive_max_files: options.wal_archive_max_files,
            payload_residency: options.payload_residency,
            payload_cache: Mutex::new(SegmentPayloadCache::new(options.payload_cache_bytes)),
            tiered_storage: options.tiered_storage,
            agent_transactions: options.agent_transactions,
            learned_ranking: options.learned_ranking,
            semantic_compression: options.semantic_compression,
            ann_guarded_sampling: options.ann_guarded_sampling,
            // A3.3: arm the per-database guarded-recall state only when opted in;
            // `None` keeps the ANN read path byte-identical to pre-A3.3.
            guarded_recall: std::sync::Mutex::new(
                options
                    .ann_guarded_sampling
                    .enabled
                    .then(|| crate::search::GuardedRecallState::new(0)),
            ),
            hnsw_build_config: options.hnsw_build_config.normalized(),
            embedding_profile: options.embedding_profile.clone(),
            retrieval_diversify_lambda_q16: options.retrieval_diversify_lambda_q16,
            retrieval_recency_window_seconds: options.retrieval_recency_window_seconds,
            retrieval_two_stage_rerank_weight_q16: options.retrieval_two_stage_rerank_weight_q16,
            retrieval_suppress_superseded: options.retrieval_suppress_superseded,
            feature_flags: options.feature_flags,
            ingestion_backpressure_policy: options.ingestion_backpressure,
            ingestion_rate_state: crate::ingestion::default_ingestion_rate_state(),
            aql_query_cache: Mutex::new(AqlQueryCache::new(options.aql_query_cache_max_entries)),
            aql_delta_index,
            corpus_synonym_store: stores.corpus_synonym_store,
            feedback_index: stores.feedback_index,
            graph_index_store: stores.graph_index_store,
            live_search_store: stores.live_search_store,
            search_context_store: stores.search_context_store,
            session_index: stores.session_index,
            memory_lifecycle_store: stores.memory_lifecycle_store,
            fact_claim_store: stores.fact_claim_store,
            conflict_index_store: stores.conflict_index_store,
            temporal_fact_store: stores.temporal_fact_store,
            temporal_validity_store: stores.temporal_validity_store,
            tool_index: stores.tool_index,
            persisted_index_cache: Mutex::new(None),
            active_read_pins: Arc::new(Mutex::new(BTreeMap::new())),
            compaction_policy: options.compaction_policy,
            text_analyzer_config: options.text_analyzer,
            _lock: lock,
            closed: false,
        };
        if options.rebuild_lazy_payload_indexes_on_open {
            database.rebuild_lazy_derived_stores_for_residency(options.payload_residency);
        }
        database.resume_interrupted_ingestion_jobs()?;
        Ok(database)
    }
}

/// Fails closed at open when the store was built with an embedding profile that
/// differs from the caller-configured one. Permissive by design: an empty store,
/// a store with no recorded profile (legacy), or an open with no configured
/// profile all pass — a mismatch only fires when both sides are present.
fn ensure_embedding_profile(
    manifest: &cortex_storage::manifest::StorageManifest,
    requested: Option<&crate::embedding_pipeline::EmbeddingProfile>,
) -> EngineResult<()> {
    if manifest.live_segments.is_empty() {
        return Ok(());
    }
    // Internal consistency (checked regardless of `requested`): a recorded
    // embedding profile must describe the same vector space as the recorded
    // vector profile. Catches a manifest left inconsistent by any path — e.g.
    // an out-of-band segment install or a future stamping bug.
    if let (Some(embedding), Some(vector)) =
        (manifest.embedding_profile.as_ref(), manifest.vector_profile)
    {
        if embedding.dimension != vector.dimension || embedding.metric != vector.metric {
            return Err(EngineError::StorageInvariant(format!(
                "recorded embedding profile {embedding:?} disagrees with the vector profile {vector:?}; the store's embedding provenance is inconsistent — rebuild or compact the store"
            )));
        }
    }
    let Some(existing) = manifest.embedding_profile.as_ref() else {
        return Ok(());
    };
    let Some(requested) = requested else {
        return Ok(());
    };
    let requested_profile = requested.to_manifest_profile();
    if *existing != requested_profile {
        return Err(EngineError::StorageInvariant(format!(
            "configured embedding profile {requested_profile:?} does not match this store's profile {existing:?}; open with the embedding model, dimension, and metric this store was built with, or rebuild/compact the store"
        )));
    }
    Ok(())
}

fn ensure_text_analyzer_profile(
    manifest: &cortex_storage::manifest::StorageManifest,
    requested: TextAnalyzerConfig,
) -> EngineResult<()> {
    if manifest.live_segments.is_empty() {
        return Ok(());
    }
    let existing = match manifest.text_analyzer_profile {
        Some(profile) => TextAnalyzerConfig::from_manifest_profile(profile).ok_or_else(|| {
            EngineError::StorageInvariant(format!(
                "manifest text analyzer profile {:?} is not supported by this engine",
                profile
            ))
        })?,
        None => TextAnalyzerConfig::default(),
    };
    if existing != requested {
        return Err(EngineError::StorageInvariant(format!(
            "requested text analyzer {:?} does not match existing manifest profile {:?}; rebuild or compact with one analyzer profile",
            requested, existing
        )));
    }
    Ok(())
}
