use std::path::Path;
use std::sync::Arc;

#[cfg(test)]
use cortex_aql::eval_bitmap_program;
use cortex_aql::BoundRetrievePlan;
use cortex_core::memtable::{CellVersion, ReadTxn};
use cortex_core::{CellDescriptor, CellId, CommitSeq};
use cortex_storage::manifest::StorageManifest;

use super::payload_cache::PayloadCacheStats;
use super::{CandidateResolver, Database, PinnedReadTxn, RetrievedCell};
use crate::error::EngineResult;
use crate::exec::{execute_retrieve, RetrieveExecutionReport};
#[cfg(test)]
use crate::feedback::current_unix_seconds;
#[cfg(test)]
use crate::retrieval_quality::cell_version_meets_quality_thresholds;
use crate::retrieval_rank::{rank_retrieved_cells, two_stage_rerank};
#[cfg(test)]
use crate::retrieval_rank::{expand_parent_context, suppress_duplicate_content};

impl Database {
    pub fn read_txn(&self) -> ReadTxn {
        ReadTxn {
            read_seq: self.current_seq,
        }
    }

    /// Pin a read transaction so that GC does not remove versions visible to
    /// this snapshot until the returned handle is dropped.
    pub fn pin_read_txn(&self) -> PinnedReadTxn {
        let read_txn = self.read_txn();
        let mut pins = self
            .active_read_pins
            .lock()
            .expect("read pin lock poisoned");
        *pins.entry(read_txn.read_seq).or_insert(0) += 1;
        PinnedReadTxn {
            read_txn,
            registry: Arc::clone(&self.active_read_pins),
        }
    }

    /// The oldest snapshot that must be preserved. GC may remove versions
    /// older than this sequence.
    pub fn gc_horizon(&self) -> CommitSeq {
        let pins = self
            .active_read_pins
            .lock()
            .expect("read pin lock poisoned");
        pins.keys().next().copied().unwrap_or(self.current_seq)
    }

    pub fn get_cell(&self, txn: ReadTxn, cell_id: CellId) -> Option<Vec<u8>> {
        self.memtable
            .read(txn, cell_id)
            .and_then(|version| self.payload_for_version(version).ok())
    }

    /// Read the latest visible payload for a cell.
    ///
    /// # Example
    ///
    /// ```
    /// # use cortex_engine::Database;
    /// # use cortex_core::CellId;
    /// # let dir = tempfile::tempdir().unwrap();
    /// # let mut db = Database::open(dir.path()).unwrap();
    /// db.put_cell(CellId(1), b"hello world".to_vec()).unwrap();
    /// let payload = db.get_latest_cell(CellId(1)).unwrap();
    /// assert_eq!(payload, b"hello world");
    /// ```
    pub fn get_latest_cell(&self, cell_id: CellId) -> Option<Vec<u8>> {
        self.get_cell(self.read_txn(), cell_id)
    }

    pub fn get_latest_cell_with_descriptor(
        &self,
        cell_id: CellId,
    ) -> Option<(Vec<u8>, CellDescriptor)> {
        self.memtable
            .read(self.read_txn(), cell_id)
            .and_then(|version| {
                self.payload_for_version(version)
                    .ok()
                    .map(|payload| (payload, version.descriptor.clone()))
            })
    }

    pub fn get_latest_cell_descriptor(&self, cell_id: CellId) -> Option<CellDescriptor> {
        self.memtable
            .read(self.read_txn(), cell_id)
            .map(|version| version.descriptor.clone())
    }

    pub fn payload_cache_stats(&self) -> PayloadCacheStats {
        self.payload_cache
            .lock()
            .expect("payload cache lock poisoned")
            .stats()
    }

    pub fn retrieve_cells<P: CandidateResolver>(
        &self,
        plan: &BoundRetrievePlan,
        provider: &P,
    ) -> EngineResult<Vec<RetrievedCell>> {
        self.retrieve_cells_with_execution_trace(plan, provider)
            .map(|report| report.cells)
    }

    pub(crate) fn retrieve_cells_with_execution_trace<P: CandidateResolver>(
        &self,
        plan: &BoundRetrievePlan,
        provider: &P,
    ) -> EngineResult<RetrieveExecutionReport> {
        execute_retrieve(self, plan, provider)
    }

    #[cfg(test)]
    pub(crate) fn retrieve_cells_direct<P: CandidateResolver>(
        &self,
        plan: &BoundRetrievePlan,
        provider: &P,
    ) -> EngineResult<Vec<RetrievedCell>> {
        let candidates = eval_bitmap_program(&plan.bitmap_program, provider)?;
        let txn = self.read_txn();
        let now = current_unix_seconds();
        let cells = candidates
            .into_iter()
            .filter_map(|candidate| {
                provider
                    .cell_id_for_candidate(candidate)
                    .map(|cell_id| (candidate, cell_id))
            })
            .filter(|(_, cell_id)| self.memory_lifecycle_store.is_active_at(*cell_id, now))
            .filter_map(|(candidate, cell_id)| {
                self.memtable
                    .read(txn, cell_id)
                    .filter(|version| {
                        cell_version_meets_quality_thresholds(version, &plan.quality_thresholds)
                    })
                    .and_then(|version| {
                        let mut cell = self.retrieved_cell_from_version(version).ok()?;
                        cell.captured_access_decision = provider
                            .captured_access_decision_for_candidate(
                                candidate,
                                cell.cell_id,
                                &cell.descriptor,
                            );
                        Some(cell)
                    })
            })
            .collect::<Vec<_>>();
        let ranked =
            suppress_duplicate_content(rank_retrieved_cells(cells, &plan.task, &plan.weights));
        Ok(expand_parent_context(ranked)
            .into_iter()
            .take(plan.context_policy.candidate_limit as usize)
            .collect())
    }

    pub fn rerank_retrieved_cells_for_task(
        &self,
        cells: Vec<RetrievedCell>,
        task: &str,
        weights: &cortex_aql::RetrievalWeights,
    ) -> Vec<RetrievedCell> {
        rank_retrieved_cells(cells, task, weights)
    }

    /// A7.2: reranks an already-materialized candidate pool by an exact dense
    /// pass against the query vector embedded in `task` (a `query_vector=` /
    /// `vector=` line), blended with the pool's incoming order by `weight_q16`
    /// (Q16; `65535` is pure dense, `0`/no query vector is a no-op). This is the
    /// exact `two_stage_rerank` the retrieve pipeline's `RerankOp` runs when
    /// `retrieval_two_stage_rerank_weight_q16` is set — exposed so a caller that
    /// already holds a lexical candidate pool (e.g. a benchmark harness) can
    /// apply the engine's own two-stage rerank directly.
    pub fn two_stage_rerank_for_task(
        &self,
        cells: Vec<RetrievedCell>,
        task: &str,
        weight_q16: u64,
    ) -> Vec<RetrievedCell> {
        two_stage_rerank(cells, task, weight_q16)
    }

    pub fn current_seq(&self) -> CommitSeq {
        self.current_seq
    }

    pub fn root_path(&self) -> &Path {
        &self.root_path
    }

    pub fn enterprise_system_check(&self) -> EngineResult<bool> {
        let report = self.validate_storage_report();
        if !report.manifest_ok || !report.wal_ok {
            return Ok(false);
        }
        Ok(true)
    }

    pub fn wal_path(&self) -> &Path {
        &self.wal_path
    }

    pub fn manifest(&self) -> &StorageManifest {
        &self.manifest
    }

    pub fn hnsw_no_fallback_rollout_policy(
        &self,
    ) -> Option<crate::search::HnswNoFallbackRolloutPolicy> {
        self.manifest
            .hnsw_no_fallback_profile
            .map(crate::search::HnswNoFallbackRolloutPolicy::from_manifest)
    }

    pub fn set_hnsw_no_fallback_rollout_policy(
        &mut self,
        policy: crate::search::HnswNoFallbackRolloutPolicy,
    ) -> EngineResult<()> {
        let mut manifest = self.manifest.clone();
        manifest.generation += 1;
        manifest.hnsw_no_fallback_profile = Some(policy.to_manifest());
        manifest.store(&self.manifest_path)?;
        self.manifest = manifest;
        Ok(())
    }

    pub fn clear_hnsw_no_fallback_rollout_policy(&mut self) -> EngineResult<()> {
        let mut manifest = self.manifest.clone();
        manifest.generation += 1;
        manifest.hnsw_no_fallback_profile = None;
        manifest.store(&self.manifest_path)?;
        self.manifest = manifest;
        Ok(())
    }

    /// Gracefully shut down the database, flushing WAL and releasing the lock.
    ///
    /// # Example
    ///
    /// ```
    /// # use cortex_engine::Database;
    /// # let dir = tempfile::tempdir().unwrap();
    /// # let db = Database::open(dir.path()).unwrap();
    /// db.close().unwrap();
    /// ```
    pub fn close(mut self) -> EngineResult<()> {
        self.writer.shutdown()?;
        self.closed = true;
        Ok(())
    }

    pub(crate) fn snapshot_versions(&self) -> Vec<CellVersion> {
        self.memtable.visible_cells(self.read_txn())
    }

    pub(crate) fn retrieved_cell_from_version(
        &self,
        version: &CellVersion,
    ) -> EngineResult<RetrievedCell> {
        Ok(RetrievedCell {
            cell_id: version.cell_id,
            payload: self.payload_for_version(version)?,
            descriptor: version.descriptor.clone(),
            captured_access_decision: None,
        })
    }
}
