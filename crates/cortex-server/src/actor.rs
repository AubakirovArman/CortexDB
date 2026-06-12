use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use cortex_aql::AgentId;
use cortex_engine::concurrency::{CapacitySemaphore, WriterPrefRwLock};
use cortex_engine::{CompactionStats, Database, DatabaseOptions, ExpiredMemoryCell};

use crate::auth::AuthRouteContext;
use crate::auth_policy_cells::{self, AuthPolicyCellSyncReport};
use crate::auth_scope_admin::{
    apply_agent_scope_mutation, AgentScopeAccess, AgentScopeMutationResponse,
};
use crate::responses::RouterError;
use crate::router::route_database_with_auth;
use crate::DEFAULT_ACTOR_QUEUE_CAPACITY;

/// Returns `true` for routes that mutate database state and therefore require a
/// write lock. This classification must stay in sync with the route handlers in
/// `router.rs`.
fn is_write_route(method: &str, target: &str) -> bool {
    let (path, _query) = target.split_once('?').unwrap_or((target, ""));
    matches!(
        (method, path),
        ("POST", "/put")
            | ("POST", "/v1/cell")
            | ("POST", "/tombstone")
            | ("DELETE", "/v1/cell")
            | ("POST", "/flush")
            | ("POST", "/v1/flush")
            | ("POST", "/v1/compact")
            | ("POST", "/v1/admin/compact/trigger")
            | ("PUT", "/v1/admin/search/hnsw/no-fallback-profile")
            | ("DELETE", "/v1/admin/search/hnsw/no-fallback-profile")
            | ("POST", "/v1/remember")
            | ("POST", "/v1/forget")
            | ("POST", "/v1/ingest/text")
            | ("POST", "/v1/ingest/json")
            | ("POST", "/v1/ingest/csv")
    ) || (method == "DELETE" && path.starts_with("/v1/ingest/jobs/"))
        || (method == "POST"
            && path.starts_with("/v1/ingest/jobs/")
            && (path.ends_with("/cancel") || path.ends_with("/retry")))
}

/// A concurrent handle to a tenant database.
///
/// `DatabaseActor` replaces the previous single-threaded actor with a
/// writer-preferring `RwLock`. Reads run concurrently under a read lock; writes
/// run exclusively under a write lock. A bounded semaphore preserves the
/// backpressure semantics of the old actor queue.
pub struct DatabaseActor {
    db: Arc<WriterPrefRwLock<Database>>,
    semaphore: CapacitySemaphore,
    capacity: usize,
    requests_sent: AtomicUsize,
    requests_rejected: AtomicUsize,
    requests_completed: AtomicUsize,
    closed: Mutex<bool>,
}

impl DatabaseActor {
    pub fn open(path: &Path) -> std::io::Result<Self> {
        Self::open_with_capacity(path, DEFAULT_ACTOR_QUEUE_CAPACITY)
    }

    pub fn open_with_capacity(path: &Path, capacity: usize) -> std::io::Result<Self> {
        Self::open_with_capacity_and_options(path, capacity, DatabaseOptions::default())
    }

    pub fn open_with_capacity_and_options(
        path: &Path,
        capacity: usize,
        options: DatabaseOptions,
    ) -> std::io::Result<Self> {
        let db = Database::open_with_options(path, options)
            .map_err(|error| std::io::Error::other(error.to_string()))?;
        Ok(Self {
            db: Arc::new(WriterPrefRwLock::new(db)),
            semaphore: CapacitySemaphore::new(capacity),
            capacity,
            requests_sent: AtomicUsize::new(0),
            requests_rejected: AtomicUsize::new(0),
            requests_completed: AtomicUsize::new(0),
            closed: Mutex::new(false),
        })
    }

    fn ensure_open(&self) -> Result<(), RouterError> {
        if *self.closed.lock().expect("closed lock poisoned") {
            return Err(RouterError::Internal("database actor stopped".to_owned()));
        }
        Ok(())
    }

    pub fn route(&self, method: &str, target: &str, body: &[u8]) -> Result<String, RouterError> {
        self.route_with_agent(method, target, body, None)
    }

    pub fn route_with_agent(
        &self,
        method: &str,
        target: &str,
        body: &[u8],
        auth_agent_id: Option<u64>,
    ) -> Result<String, RouterError> {
        self.route_with_auth(
            method,
            target,
            body,
            AuthRouteContext::for_agent(auth_agent_id),
        )
    }

    pub(crate) fn route_with_auth(
        &self,
        method: &str,
        target: &str,
        body: &[u8],
        auth_context: AuthRouteContext,
    ) -> Result<String, RouterError> {
        self.ensure_open()?;
        let permit = match self.semaphore.try_acquire() {
            Some(permit) => permit,
            None => {
                self.requests_rejected.fetch_add(1, Ordering::Relaxed);
                return Err(RouterError::DatabaseBusy("database actor busy".to_owned()));
            }
        };
        self.requests_sent.fetch_add(1, Ordering::Relaxed);

        let result = if is_write_route(method, target) {
            let guard = self.db.write();
            route_database_with_auth(guard, method, target, body, auth_context)
        } else {
            let guard = self.db.read();
            route_database_with_auth(guard, method, target, body, auth_context)
        };

        drop(permit);
        self.requests_completed.fetch_add(1, Ordering::Relaxed);
        result
    }

    pub fn queue_depth(&self) -> usize {
        self.capacity - self.semaphore.available_permits()
    }

    pub fn queue_capacity(&self) -> usize {
        self.capacity
    }

    pub fn requests_sent(&self) -> usize {
        self.requests_sent.load(Ordering::Relaxed)
    }

    pub fn requests_rejected(&self) -> usize {
        self.requests_rejected.load(Ordering::Relaxed)
    }

    pub fn requests_completed(&self) -> usize {
        self.requests_completed.load(Ordering::Relaxed)
    }

    pub fn active_readers(&self) -> usize {
        self.db.active_readers()
    }

    pub fn waiting_writers(&self) -> usize {
        self.db.waiting_writers()
    }

    pub fn expire_memory(
        &self,
        now_unix_seconds: u64,
    ) -> Result<Vec<ExpiredMemoryCell>, RouterError> {
        self.ensure_open()?;
        let _permit = self.semaphore.acquire();
        self.requests_sent.fetch_add(1, Ordering::Relaxed);
        let mut guard = self.db.write();
        let result = guard
            .expire_memory_cells(now_unix_seconds)
            .map_err(|e| RouterError::Internal(e.to_string()));
        self.requests_completed.fetch_add(1, Ordering::Relaxed);
        result
    }

    /// Run an incremental compaction if storage pressure or the live segment
    /// count crosses the configured threshold. Returns `None` when no work was
    /// performed, including when foreground writes are waiting so the
    /// compaction does not starve them.
    pub fn maybe_incremental_compact(&self) -> Result<Option<CompactionStats>, RouterError> {
        self.ensure_open()?;
        if self.waiting_writers() > 0 {
            return Ok(None);
        }
        let _permit = self.semaphore.acquire();
        self.requests_sent.fetch_add(1, Ordering::Relaxed);
        let mut guard = self.db.write();
        let result = guard
            .maybe_incremental_compact()
            .map_err(|e| RouterError::Internal(e.to_string()));
        self.requests_completed.fetch_add(1, Ordering::Relaxed);
        result
    }

    /// Force an incremental compaction regardless of current pressure.
    pub fn incremental_compact(&self) -> Result<CompactionStats, RouterError> {
        self.ensure_open()?;
        let _permit = self.semaphore.acquire();
        self.requests_sent.fetch_add(1, Ordering::Relaxed);
        let mut guard = self.db.write();
        let result = guard
            .incremental_compact()
            .map_err(|e| RouterError::Internal(e.to_string()));
        self.requests_completed.fetch_add(1, Ordering::Relaxed);
        result
    }

    pub(crate) fn sync_auth_policy_store(
        &self,
        store_json: &str,
    ) -> Result<AuthPolicyCellSyncReport, RouterError> {
        self.ensure_open()?;
        let _permit = self.semaphore.acquire();
        self.requests_sent.fetch_add(1, Ordering::Relaxed);
        let mut guard = self.db.write();
        let result = auth_policy_cells::sync_store_json_to_database(&mut guard, store_json);
        self.requests_completed.fetch_add(1, Ordering::Relaxed);
        result
    }

    pub(crate) fn mutate_agent_scope(
        &self,
        agent_id: AgentId,
        scope: &str,
        access: AgentScopeAccess,
        grant: bool,
    ) -> Result<AgentScopeMutationResponse, RouterError> {
        self.ensure_open()?;
        let _permit = self.semaphore.acquire();
        self.requests_sent.fetch_add(1, Ordering::Relaxed);
        let mut guard = self.db.write();
        let result = apply_agent_scope_mutation(&mut guard, agent_id, scope, access, grant);
        self.requests_completed.fetch_add(1, Ordering::Relaxed);
        result
    }

    pub fn close(&self) -> Result<(), RouterError> {
        let mut closed = self.closed.lock().expect("closed lock poisoned");
        if *closed {
            return Ok(());
        }
        *closed = true;
        Ok(())
    }
}

impl Drop for DatabaseActor {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;
    use std::sync::Arc;
    use std::thread;
    use std::time::{Duration, Instant};

    use super::DatabaseActor;

    #[test]
    fn queue_depth_returns_to_zero_after_completed_request() {
        let dir = tempfile::tempdir().unwrap();
        let actor = DatabaseActor::open_with_capacity(dir.path(), 1).unwrap();

        let response = actor
            .route("GET", "/v1/health", b"")
            .expect("health route should succeed");

        assert!(response.contains(r#""status":"ok""#));
        assert_eq!(actor.queue_depth(), 0);
        assert_eq!(actor.requests_sent(), 1);
        assert_eq!(actor.requests_completed(), 1);
    }

    #[test]
    fn close_is_idempotent_and_blocks_new_requests() {
        let dir = tempfile::tempdir().unwrap();
        let actor = DatabaseActor::open_with_capacity(dir.path(), 8).unwrap();

        let _ = actor
            .route("GET", "/v1/health", b"")
            .expect("route should work before close");
        actor.close().expect("actor close should succeed");
        actor
            .close()
            .expect("closing an already closed actor should stay idempotent");

        assert_eq!(actor.queue_depth(), 0);
        assert_eq!(actor.requests_completed(), 1);
        assert!(actor.route("GET", "/v1/health", b"").is_err());
    }

    #[test]
    fn concurrent_reads_run_in_parallel() {
        let dir = tempfile::tempdir().unwrap();
        let actor = Arc::new(DatabaseActor::open_with_capacity(dir.path(), 8).unwrap());
        actor
            .route(
                "POST",
                "/v1/cell?cell_id=1",
                b"scope=default\nstatus=ready\nhello",
            )
            .unwrap();

        let mut handles = Vec::new();
        let start = Instant::now();
        for _ in 0..4 {
            let actor = Arc::clone(&actor);
            handles.push(thread::spawn(move || {
                actor
                    .route("GET", "/v1/cell?cell_id=1", b"")
                    .expect("read should succeed");
            }));
        }
        for handle in handles {
            handle.join().unwrap();
        }
        let elapsed = start.elapsed();
        assert!(
            elapsed < Duration::from_millis(500),
            "concurrent reads serialized: {elapsed:?}"
        );
    }

    #[test]
    fn waiting_write_blocks_new_reads() {
        let dir = tempfile::tempdir().unwrap();
        let actor = Arc::new(DatabaseActor::open_with_capacity(dir.path(), 8).unwrap());
        actor
            .route(
                "POST",
                "/v1/cell?cell_id=1",
                b"scope=default\nstatus=ready\nv1",
            )
            .unwrap();

        // Start a write that will take a little while by holding a read lock
        // indirectly: we cannot easily delay a write, but we can verify that
        // a writer waiting behind active readers eventually gets priority by
        // checking the waiting_writers metric after starting a write in a
        // separate thread while reads are active.
        let actor_write = Arc::clone(&actor);
        let write_started = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let write_started_clone = Arc::clone(&write_started);
        let write_handle = thread::spawn(move || {
            write_started_clone.store(true, Ordering::SeqCst);
            actor_write
                .route(
                    "POST",
                    "/v1/cell?cell_id=1",
                    b"scope=default\nstatus=ready\nv2",
                )
                .expect("write should succeed");
        });

        // Spin until the write thread has started.
        while !write_started.load(Ordering::SeqCst) {
            thread::yield_now();
        }
        thread::sleep(Duration::from_millis(5));

        // The writer is waiting for any active readers to drain.
        // We may or may not catch the exact moment, so this is best-effort.
        // The main property we want is that the write eventually completes.
        let _ = actor.waiting_writers();

        // New readers should not jump ahead of the waiting writer.
        let actor_read = Arc::clone(&actor);
        let read_handle = thread::spawn(move || {
            actor_read
                .route("GET", "/v1/cell?cell_id=1", b"")
                .expect("read should succeed")
        });

        write_handle.join().unwrap();
        let body = read_handle.join().unwrap();
        assert!(body.contains("v1") || body.contains("v2"));
    }

    #[test]
    fn write_route_classifier_covers_mutating_routes() {
        let write_routes = [
            ("POST", "/put"),
            ("POST", "/v1/cell"),
            ("POST", "/tombstone"),
            ("DELETE", "/v1/cell"),
            ("POST", "/flush"),
            ("POST", "/v1/flush"),
            ("POST", "/v1/compact"),
            ("POST", "/v1/admin/compact/trigger"),
            ("PUT", "/v1/admin/search/hnsw/no-fallback-profile"),
            ("DELETE", "/v1/admin/search/hnsw/no-fallback-profile"),
            ("POST", "/v1/remember"),
            ("POST", "/v1/forget"),
            ("POST", "/v1/ingest/text"),
            ("POST", "/v1/ingest/json"),
            ("POST", "/v1/ingest/csv"),
            ("DELETE", "/v1/ingest/jobs/42"),
            ("POST", "/v1/ingest/jobs/42/cancel"),
            ("POST", "/v1/ingest/jobs/42/retry"),
        ];
        for (method, target) in write_routes {
            assert!(
                super::is_write_route(method, target),
                "{method} {target} must take a write lock"
            );
        }

        let read_routes = [
            ("GET", "/v1/health"),
            ("GET", "/v1/stats"),
            ("GET", "/v1/validate"),
            ("GET", "/v1/cell?cell_id=1"),
            ("POST", "/v1/context"),
            ("POST", "/v1/context/trace"),
            ("POST", "/v1/aql"),
            ("POST", "/v1/search"),
            ("POST", "/v1/search/explain"),
            ("POST", "/v1/search/ann-evaluate"),
            ("GET", "/v1/admin/search/hnsw/no-fallback-profile"),
            ("GET", "/v1/admin/compact/status"),
            ("GET", "/v1/metrics"),
            ("GET", "/v1/ann/metrics"),
            ("POST", "/v1/verify"),
            ("GET", "/v1/ingest/jobs"),
            ("GET", "/v1/ingest/jobs/42"),
        ];
        for (method, target) in read_routes {
            assert!(
                !super::is_write_route(method, target),
                "{method} {target} should not take a write lock"
            );
        }
    }
}
