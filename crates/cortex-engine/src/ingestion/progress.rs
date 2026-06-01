use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use cortex_core::CellId;

use crate::database::Database;
use crate::error::{EngineError, EngineResult};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct IngestionJobId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IngestionJobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IngestionProgress {
    pub job_id: IngestionJobId,
    pub label: String,
    pub status: IngestionJobStatus,
    pub total_items: Option<u64>,
    pub completed_items: u64,
    pub failed_items: u64,
    #[serde(with = "cell_id_opt_serde")]
    pub last_cell_id: Option<CellId>,
    pub message: Option<String>,
    #[serde(default)]
    pub retry_count: u32,
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
}

const fn default_max_retries() -> u32 {
    3
}

mod cell_id_opt_serde {
    use cortex_core::CellId;
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &Option<CellId>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(cell_id) => serializer.serialize_some(&cell_id.0),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<CellId>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt: Option<u64> = Option::deserialize(deserializer)?;
        Ok(opt.map(CellId))
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct IngestionProgressTracker {
    next_id: u64,
    jobs: BTreeMap<IngestionJobId, IngestionProgress>,
}

impl IngestionProgressTracker {
    pub fn start(
        &mut self,
        label: impl Into<String>,
        total_items: Option<u64>,
    ) -> EngineResult<IngestionJobId> {
        self.next_id = self.next_id.checked_add(1).ok_or_else(progress_overflow)?;
        let job_id = IngestionJobId(self.next_id);
        self.jobs.insert(
            job_id,
            IngestionProgress {
                job_id,
                label: label.into(),
                status: IngestionJobStatus::Running,
                total_items,
                completed_items: 0,
                failed_items: 0,
                last_cell_id: None,
                message: None,
                retry_count: 0,
                max_retries: 3,
            },
        );
        Ok(job_id)
    }

    pub fn record_cell(&mut self, job_id: IngestionJobId, cell_id: CellId) -> EngineResult<()> {
        let progress = self.job_mut(job_id)?;
        progress.completed_items = progress
            .completed_items
            .checked_add(1)
            .ok_or_else(progress_overflow)?;
        progress.last_cell_id = Some(cell_id);
        Ok(())
    }

    pub fn finish(&mut self, job_id: IngestionJobId) -> EngineResult<()> {
        self.job_mut(job_id)?.status = IngestionJobStatus::Completed;
        Ok(())
    }

    pub fn fail(&mut self, job_id: IngestionJobId, message: impl Into<String>) -> EngineResult<()> {
        let progress = self.job_mut(job_id)?;
        progress.status = IngestionJobStatus::Failed;
        progress.failed_items = progress
            .failed_items
            .checked_add(1)
            .ok_or_else(progress_overflow)?;
        progress.message = Some(message.into());
        Ok(())
    }

    pub fn cancel(&mut self, job_id: IngestionJobId) -> EngineResult<()> {
        let progress = self.job_mut(job_id)?;
        match progress.status {
            IngestionJobStatus::Queued | IngestionJobStatus::Running => {
                progress.status = IngestionJobStatus::Cancelled;
                Ok(())
            }
            _ => Err(EngineError::InvalidOperation),
        }
    }

    pub fn retry(&mut self, job_id: IngestionJobId) -> EngineResult<()> {
        let progress = self.job_mut(job_id)?;
        if progress.status != IngestionJobStatus::Failed {
            return Err(EngineError::InvalidOperation);
        }
        if progress.retry_count >= progress.max_retries {
            return Err(EngineError::StorageInvariant(
                "max retries exceeded".to_owned(),
            ));
        }
        progress.retry_count += 1;
        progress.status = IngestionJobStatus::Queued;
        progress.message = None;
        Ok(())
    }

    pub fn get(&self, job_id: IngestionJobId) -> Option<&IngestionProgress> {
        self.jobs.get(&job_id)
    }

    pub fn list(&self) -> Vec<IngestionProgress> {
        self.jobs.values().cloned().collect()
    }

    /// Seeds `next_id` from the largest existing job id on disk so that
    /// new job ids never collide with persisted jobs.
    pub fn seed_next_id_from_disk(&mut self, db: &Database) {
        if let Ok(jobs) = db.list_ingestion_jobs() {
            if let Some(max) = jobs.iter().map(|j| j.job_id.0).max() {
                self.next_id = self.next_id.max(max);
            }
        }
    }

    fn job_mut(&mut self, job_id: IngestionJobId) -> EngineResult<&mut IngestionProgress> {
        self.jobs
            .get_mut(&job_id)
            .ok_or(EngineError::InvalidOperation)
    }
}

fn progress_overflow() -> EngineError {
    EngineError::StorageInvariant("ingestion progress counter overflow".to_owned())
}
