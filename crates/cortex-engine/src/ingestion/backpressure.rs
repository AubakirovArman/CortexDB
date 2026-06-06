use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::database::Database;
use crate::error::{EngineError, EngineResult};
use crate::ingestion::{IngestionJobId, IngestionJobStatus};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IngestionBackpressurePolicy {
    pub max_queued_jobs: usize,
    pub max_running_jobs: usize,
    pub max_input_bytes: usize,
    pub max_total_items: u64,
    pub max_requests_per_window: u32,
    pub rate_window_seconds: u64,
}

impl Default for IngestionBackpressurePolicy {
    fn default() -> Self {
        Self {
            max_queued_jobs: 10_000,
            max_running_jobs: 1_024,
            max_input_bytes: 128 * 1024 * 1024,
            max_total_items: 1_000_000,
            max_requests_per_window: 10_000,
            rate_window_seconds: 60,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IngestionBackpressureRequest {
    pub input_bytes: usize,
    pub total_items: Option<u64>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct IngestionRateState {
    pub window_start_unix_seconds: u64,
    pub requests_in_window: u32,
}

impl Database {
    pub fn ingestion_backpressure_policy(&self) -> IngestionBackpressurePolicy {
        self.ingestion_backpressure_policy
    }

    pub fn check_ingestion_backpressure(
        &self,
        request: IngestionBackpressureRequest,
    ) -> EngineResult<()> {
        self.check_ingestion_size(request)?;
        self.check_ingestion_jobs()?;
        self.check_ingestion_rate()
    }

    pub fn ensure_ingestion_job_not_cancelled(&self, job_id: IngestionJobId) -> EngineResult<()> {
        let Some(progress) = self.load_ingestion_job(job_id.0)? else {
            return Ok(());
        };
        if progress.status == IngestionJobStatus::Cancelled {
            return Err(EngineError::IngestionCancelled(job_id.0));
        }
        Ok(())
    }

    fn check_ingestion_size(&self, request: IngestionBackpressureRequest) -> EngineResult<()> {
        let policy = self.ingestion_backpressure_policy;
        if request.input_bytes > policy.max_input_bytes {
            return Err(EngineError::IngestionInputTooLarge {
                limit_bytes: policy.max_input_bytes,
                actual_bytes: request.input_bytes,
            });
        }
        if let Some(items) = request.total_items {
            if items > policy.max_total_items {
                return Err(EngineError::IngestionItemLimitExceeded {
                    limit_items: policy.max_total_items,
                    actual_items: items,
                });
            }
        }
        Ok(())
    }

    fn check_ingestion_jobs(&self) -> EngineResult<()> {
        let policy = self.ingestion_backpressure_policy;
        let jobs = self.list_ingestion_jobs()?;
        let queued = jobs
            .iter()
            .filter(|job| job.status == IngestionJobStatus::Queued)
            .count();
        let running = jobs
            .iter()
            .filter(|job| job.status == IngestionJobStatus::Running)
            .count();
        if queued >= policy.max_queued_jobs {
            return Err(EngineError::IngestionBackpressure {
                reason: format!(
                    "ingestion queued job limit exceeded: queued={queued}, limit={}",
                    policy.max_queued_jobs
                ),
            });
        }
        if running >= policy.max_running_jobs {
            return Err(EngineError::IngestionBackpressure {
                reason: format!(
                    "ingestion running job limit exceeded: running={running}, limit={}",
                    policy.max_running_jobs
                ),
            });
        }
        Ok(())
    }

    fn check_ingestion_rate(&self) -> EngineResult<()> {
        let policy = self.ingestion_backpressure_policy;
        if policy.max_requests_per_window == u32::MAX {
            return Ok(());
        }
        let now = unix_now();
        let window_seconds = policy.rate_window_seconds.max(1);
        let mut state = self.ingestion_rate_state.lock().map_err(|_| {
            EngineError::StorageInvariant("ingestion rate lock poisoned".to_owned())
        })?;
        if state.window_start_unix_seconds == 0
            || now
                >= state
                    .window_start_unix_seconds
                    .saturating_add(window_seconds)
        {
            state.window_start_unix_seconds = now;
            state.requests_in_window = 0;
        }
        if state.requests_in_window >= policy.max_requests_per_window {
            return Err(EngineError::IngestionRateLimited {
                limit: policy.max_requests_per_window,
                window_seconds,
            });
        }
        state.requests_in_window = state.requests_in_window.saturating_add(1);
        Ok(())
    }
}

pub(crate) fn default_ingestion_rate_state() -> Mutex<IngestionRateState> {
    Mutex::new(IngestionRateState::default())
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}
