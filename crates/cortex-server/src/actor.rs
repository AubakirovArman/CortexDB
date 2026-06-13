mod agent_views;
mod operations;
mod routing;

#[cfg(test)]
mod tests;

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use cortex_engine::concurrency::{CapacitySemaphore, WriterPrefRwLock};
use cortex_engine::{Database, DatabaseOptions};

use crate::responses::RouterError;
use crate::DEFAULT_ACTOR_QUEUE_CAPACITY;

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
