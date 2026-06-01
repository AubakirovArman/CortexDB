use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use cortex_core::CellId;

use crate::database::Database;
use crate::error::{EngineError, EngineResult};

use super::{
    CsvIngestOptions, IngestedCell, IngestionJobId, IngestionJobStatus, IngestionProgress,
    IngestionProgressTracker,
};

static INGESTION_JOB_TEMP_COUNTER: AtomicU64 = AtomicU64::new(1);

impl Database {
    pub fn save_ingestion_job(&self, progress: &IngestionProgress) -> EngineResult<()> {
        let dir = self.root_path.join("ingestion_jobs");
        fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}.json", progress.job_id.0));
        let content = serde_json::to_string_pretty(progress)
            .map_err(|error| EngineError::StorageInvariant(error.to_string()))?;
        write_atomic(&path, content.as_bytes())?;
        Ok(())
    }

    pub fn load_ingestion_job(&self, id: u64) -> EngineResult<Option<IngestionProgress>> {
        let path = self
            .root_path
            .join("ingestion_jobs")
            .join(format!("{id}.json"));
        if !path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(path)?;
        let progress: IngestionProgress = serde_json::from_str(&content)
            .map_err(|error| EngineError::StorageInvariant(error.to_string()))?;
        Ok(Some(progress))
    }

    pub fn list_ingestion_jobs(&self) -> EngineResult<Vec<IngestionProgress>> {
        let dir = self.root_path.join("ingestion_jobs");
        if !dir.exists() {
            return Ok(Vec::new());
        }
        let mut jobs = Vec::new();
        for entry in fs::read_dir(dir)?.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "json") {
                let content = fs::read_to_string(path)?;
                if let Ok(progress) = serde_json::from_str::<IngestionProgress>(&content) {
                    jobs.push(progress);
                }
            }
        }
        jobs.sort_by_key(|job| job.job_id.0);
        Ok(jobs)
    }

    pub fn ingest_csv_with_progress(
        &mut self,
        first_cell_id: CellId,
        csv: &str,
        options: CsvIngestOptions,
        tracker: &mut IngestionProgressTracker,
        label: impl Into<String>,
    ) -> EngineResult<(IngestionJobId, Vec<IngestedCell>)> {
        let total = csv.lines().count().saturating_sub(1) as u64;
        let job_id = tracker.start(label, Some(total))?;
        persist_tracker_job(self, tracker, job_id)?;
        match self.ingest_csv(first_cell_id, csv, options) {
            Ok(cells) => {
                for cell in &cells {
                    tracker.record_cell(job_id, cell.cell_id)?;
                }
                tracker.finish(job_id)?;
                persist_tracker_job(self, tracker, job_id)?;
                Ok((job_id, cells))
            }
            Err(error) => {
                let _ = tracker.fail(job_id, error.to_string());
                persist_tracker_job(self, tracker, job_id)?;
                Err(error)
            }
        }
    }

    /// Requeue jobs that were persisted as running before a process restart.
    ///
    /// CortexDB's current ingestion path is synchronous. On crash/restart there
    /// is no safe in-flight worker to continue, so the durable resume point is
    /// the job metadata: `running` jobs become `queued` and can be retried by an
    /// operator or client without being lost or stuck forever.
    pub fn resume_interrupted_ingestion_jobs(&self) -> EngineResult<Vec<IngestionProgress>> {
        let mut resumed = Vec::new();
        for mut progress in self.list_ingestion_jobs()? {
            if progress.status == IngestionJobStatus::Running {
                progress.status = IngestionJobStatus::Queued;
                progress.message =
                    Some("resumed after database restart; retry the ingestion request".to_owned());
                self.save_ingestion_job(&progress)?;
                resumed.push(progress);
            }
        }
        Ok(resumed)
    }

    pub fn cancel_ingestion_job(&self, id: u64) -> EngineResult<IngestionProgress> {
        let mut progress = self
            .load_ingestion_job(id)?
            .ok_or(EngineError::InvalidOperation)?;
        match progress.status {
            IngestionJobStatus::Queued | IngestionJobStatus::Running => {
                progress.status = IngestionJobStatus::Cancelled;
                self.save_ingestion_job(&progress)?;
                Ok(progress)
            }
            _ => Err(EngineError::InvalidOperation),
        }
    }

    pub fn retry_ingestion_job(&self, id: u64) -> EngineResult<IngestionProgress> {
        let mut progress = self
            .load_ingestion_job(id)?
            .ok_or(EngineError::InvalidOperation)?;
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
        self.save_ingestion_job(&progress)?;
        Ok(progress)
    }

    pub fn delete_ingestion_job(&self, id: u64) -> EngineResult<bool> {
        let path = self
            .root_path
            .join("ingestion_jobs")
            .join(format!("{id}.json"));
        if !path.exists() {
            return Ok(false);
        }
        fs::remove_file(path)?;
        Ok(true)
    }
}

fn persist_tracker_job(
    db: &Database,
    tracker: &IngestionProgressTracker,
    job_id: IngestionJobId,
) -> EngineResult<()> {
    let progress = tracker.get(job_id).ok_or(EngineError::InvalidOperation)?;
    db.save_ingestion_job(progress)
}

fn write_atomic(path: &Path, bytes: &[u8]) -> EngineResult<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    let tmp = temp_path(path);
    {
        let mut file = File::create(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        OpenOptions::new().read(true).open(parent)?.sync_all()?;
    }
    Ok(())
}

fn temp_path(path: &Path) -> PathBuf {
    let counter = INGESTION_JOB_TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("ingestion-job");
    path.with_file_name(format!(
        "{file_name}.tmp.{}.{}",
        std::process::id(),
        counter
    ))
}
