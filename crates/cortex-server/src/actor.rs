use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::JoinHandle;

use cortex_engine::Database;

use crate::responses::RouterError;
use crate::router::route_database;

enum ActorCommand {
    Route {
        method: String,
        target: String,
        body: Vec<u8>,
        reply: mpsc::Sender<Result<String, RouterError>>,
    },
    ExpireMemory {
        now_unix_seconds: u64,
        reply: mpsc::Sender<Result<Vec<cortex_engine::memory::ExpiredMemoryCell>, RouterError>>,
    },
    Shutdown,
}

pub struct DatabaseActor {
    tx: mpsc::SyncSender<ActorCommand>,
    worker: Mutex<Option<JoinHandle<()>>>,
    queued: AtomicUsize,
    capacity: usize,
    requests_sent: AtomicUsize,
    requests_rejected: AtomicUsize,
    requests_completed: AtomicUsize,
}

impl DatabaseActor {
    pub fn open(path: &Path) -> std::io::Result<Self> {
        Self::open_with_capacity(path, 1024)
    }

    pub fn open_with_capacity(path: &Path, capacity: usize) -> std::io::Result<Self> {
        let db = Database::open(path).map_err(|error| std::io::Error::other(error.to_string()))?;
        let (tx, rx) = mpsc::sync_channel::<ActorCommand>(capacity);
        let queued = Arc::new(AtomicUsize::new(0));
        let queued_worker = queued.clone();
        let worker = std::thread::spawn(move || {
            let mut db = db;
            while let Ok(command) = rx.recv() {
                queued_worker.fetch_sub(1, Ordering::Relaxed);
                match command {
                    ActorCommand::Route {
                        method,
                        target,
                        body,
                        reply,
                    } => {
                        let result = route_database(&mut db, &method, &target, &body);
                        let _ = reply.send(result);
                    }
                    ActorCommand::ExpireMemory {
                        now_unix_seconds,
                        reply,
                    } => {
                        let result = db
                            .expire_memory_cells(now_unix_seconds)
                            .map_err(|e| RouterError::Internal(e.to_string()));
                        let _ = reply.send(result);
                    }
                    ActorCommand::Shutdown => break,
                }
            }
        });

        Ok(Self {
            tx,
            worker: Mutex::new(Some(worker)),
            queued: AtomicUsize::new(0),
            capacity,
            requests_sent: AtomicUsize::new(0),
            requests_rejected: AtomicUsize::new(0),
            requests_completed: AtomicUsize::new(0),
        })
    }

    pub fn route(&self, method: &str, target: &str, body: &[u8]) -> Result<String, RouterError> {
        let (reply_tx, reply_rx) = mpsc::channel();
        match self.tx.try_send(ActorCommand::Route {
            method: method.to_owned(),
            target: target.to_owned(),
            body: body.to_vec(),
            reply: reply_tx,
        }) {
            Ok(()) => {
                self.queued.fetch_add(1, Ordering::Relaxed);
                self.requests_sent.fetch_add(1, Ordering::Relaxed);
            }
            Err(mpsc::TrySendError::Full(_)) => {
                self.requests_rejected.fetch_add(1, Ordering::Relaxed);
                return Err(RouterError::DatabaseBusy("database actor busy".to_owned()));
            }
            Err(mpsc::TrySendError::Disconnected(_)) => {
                return Err(RouterError::Internal("database actor stopped".to_owned()));
            }
        }
        let result = reply_rx
            .recv()
            .map_err(|_| RouterError::Internal("database actor stopped".to_owned()))?;
        self.requests_completed.fetch_add(1, Ordering::Relaxed);
        result
    }

    pub fn queue_depth(&self) -> usize {
        self.queued.load(Ordering::Relaxed)
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

    pub fn expire_memory(
        &self,
        now_unix_seconds: u64,
    ) -> Result<Vec<cortex_engine::memory::ExpiredMemoryCell>, RouterError> {
        let (reply_tx, reply_rx) = mpsc::channel();
        match self.tx.try_send(ActorCommand::ExpireMemory {
            now_unix_seconds,
            reply: reply_tx,
        }) {
            Ok(()) => {
                self.queued.fetch_add(1, Ordering::Relaxed);
                self.requests_sent.fetch_add(1, Ordering::Relaxed);
            }
            Err(mpsc::TrySendError::Full(_)) => {
                self.requests_rejected.fetch_add(1, Ordering::Relaxed);
                return Err(RouterError::DatabaseBusy("database actor busy".to_owned()));
            }
            Err(mpsc::TrySendError::Disconnected(_)) => {
                return Err(RouterError::Internal("database actor stopped".to_owned()));
            }
        }
        let result = reply_rx
            .recv()
            .map_err(|_| RouterError::Internal("database actor stopped".to_owned()))?;
        self.requests_completed.fetch_add(1, Ordering::Relaxed);
        result
    }
}

impl Drop for DatabaseActor {
    fn drop(&mut self) {
        // Use try_send so shutdown never blocks even if the queue is full.
        let _ = self.tx.try_send(ActorCommand::Shutdown);
        if let Ok(mut worker) = self.worker.lock() {
            if let Some(handle) = worker.take() {
                let _ = handle.join();
            }
        }
    }
}
