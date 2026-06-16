use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

pub(super) struct RunLogger {
    started: Instant,
    log_file: Option<PathBuf>,
    status_file: Option<PathBuf>,
}

impl RunLogger {
    pub(super) fn new(
        started: Instant,
        log_file: Option<PathBuf>,
        status_file: Option<PathBuf>,
    ) -> Result<Self, String> {
        if let Some(path) = &log_file {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
            }
            fs::write(path, "")
                .map_err(|error| format!("failed to initialize {}: {error}", path.display()))?;
        }
        if let Some(path) = &status_file {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
            }
        }
        Ok(Self {
            started,
            log_file,
            status_file,
        })
    }

    pub(super) fn log(&self, message: &str) {
        let line = format!(
            "[enterprise-rag-retrieval +{:>6.1}s] {message}",
            self.started.elapsed().as_secs_f64()
        );
        eprintln!("{line}");
        if let Some(path) = &self.log_file {
            if let Ok(mut handle) = OpenOptions::new().create(true).append(true).open(path) {
                let _ = writeln!(handle, "{line}");
            }
        }
    }

    pub(super) fn status(
        &self,
        stage: &str,
        state: &str,
        message: &str,
        completed: Option<usize>,
        total: Option<usize>,
        extra: &[(&str, Value)],
    ) {
        let Some(path) = &self.status_file else {
            return;
        };
        let elapsed_seconds = self.started.elapsed().as_secs_f64();
        let mut payload = json!({
            "schema_version": "cortexdb.enterprise_rag_bench.retrieval_progress_status.v1",
            "stage": stage,
            "state": state,
            "message": message,
            "elapsed_seconds": (elapsed_seconds * 10.0).round() / 10.0,
            "updated_unix_ms": now_unix_ms(),
            "log_file": self.log_file.as_ref().map(|path| path.display().to_string()),
        });
        if let Some(completed) = completed {
            payload["completed"] = json!(completed);
        }
        if let Some(total) = total {
            payload["total"] = json!(total);
        }
        if let (Some(completed), Some(total)) = (completed, total) {
            let progress_pct = if total > 0 {
                completed as f64 / total as f64 * 100.0
            } else {
                100.0
            };
            payload["progress_pct"] = json!((progress_pct * 100.0).round() / 100.0);
        }
        if let Some(object) = payload.as_object_mut() {
            for (key, value) in extra {
                object.insert((*key).to_owned(), value.clone());
            }
        }
        if let Ok(bytes) = serde_json::to_vec_pretty(&payload) {
            let mut with_newline = bytes;
            with_newline.push(b'\n');
            let _ = fs::write(path, with_newline);
        }
    }
}

fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

pub(super) fn logger_progress_due(completed: usize, total: usize, every: usize) -> bool {
    every > 0 && (completed.is_multiple_of(every) || completed == total)
}
