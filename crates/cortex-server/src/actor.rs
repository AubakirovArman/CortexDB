use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::JoinHandle;

use cortex_engine::Database;

use crate::responses::RouterError;
use crate::router::route_database_with_agent;
use crate::DEFAULT_ACTOR_QUEUE_CAPACITY;

enum ActorCommand {
    Route {
        method: String,
        target: String,
        body: Vec<u8>,
        auth_agent_id: Option<u64>,
        reply: mpsc::Sender<Result<String, RouterError>>,
    },
    ExpireMemory {
        now_unix_seconds: u64,
        reply: mpsc::Sender<Result<Vec<cortex_engine::memory::ExpiredMemoryCell>, RouterError>>,
    },
    #[cfg(test)]
    TestBlock {
        started: mpsc::Sender<()>,
        release: mpsc::Receiver<()>,
    },
    #[cfg(test)]
    TestNoop,
    Shutdown,
}

pub struct DatabaseActor {
    tx: Mutex<Option<mpsc::SyncSender<ActorCommand>>>,
    worker: Mutex<Option<JoinHandle<()>>>,
    queued: Arc<AtomicUsize>,
    capacity: usize,
    requests_sent: AtomicUsize,
    requests_rejected: AtomicUsize,
    requests_completed: AtomicUsize,
}

impl DatabaseActor {
    pub fn open(path: &Path) -> std::io::Result<Self> {
        Self::open_with_capacity(path, DEFAULT_ACTOR_QUEUE_CAPACITY)
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
                        auth_agent_id,
                        reply,
                    } => {
                        let result = route_database_with_agent(
                            &mut db,
                            &method,
                            &target,
                            &body,
                            auth_agent_id,
                        );
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
                    #[cfg(test)]
                    ActorCommand::TestBlock { started, release } => {
                        let _ = started.send(());
                        let _ = release.recv();
                    }
                    #[cfg(test)]
                    ActorCommand::TestNoop => {}
                    ActorCommand::Shutdown => break,
                }
            }
        });

        Ok(Self {
            tx: Mutex::new(Some(tx)),
            worker: Mutex::new(Some(worker)),
            queued,
            capacity,
            requests_sent: AtomicUsize::new(0),
            requests_rejected: AtomicUsize::new(0),
            requests_completed: AtomicUsize::new(0),
        })
    }

    fn enqueue(&self, command: ActorCommand) -> Result<(), RouterError> {
        self.queued.fetch_add(1, Ordering::Relaxed);
        let result = {
            let tx = self
                .tx
                .lock()
                .map_err(|e| RouterError::Internal(e.to_string()))?;
            let Some(tx) = tx.as_ref() else {
                self.queued.fetch_sub(1, Ordering::Relaxed);
                return Err(RouterError::Internal("database actor stopped".to_owned()));
            };
            tx.try_send(command)
        };

        match result {
            Ok(()) => {
                self.requests_sent.fetch_add(1, Ordering::Relaxed);
                Ok(())
            }
            Err(mpsc::TrySendError::Full(_)) => {
                self.queued.fetch_sub(1, Ordering::Relaxed);
                self.requests_rejected.fetch_add(1, Ordering::Relaxed);
                Err(RouterError::DatabaseBusy("database actor busy".to_owned()))
            }
            Err(mpsc::TrySendError::Disconnected(_)) => {
                self.queued.fetch_sub(1, Ordering::Relaxed);
                Err(RouterError::Internal("database actor stopped".to_owned()))
            }
        }
    }

    pub fn close(&self) -> Result<(), RouterError> {
        let tx = self
            .tx
            .lock()
            .map_err(|error| RouterError::Internal(error.to_string()))?
            .take();
        if tx.is_none() {
            return Ok(());
        }

        // Best-effort shutdown command. If the queue is full, we still stop via
        // sender drop + queue drain, which is sufficient because worker loop exits
        // when channel disconnects and buffered commands are consumed.
        if let Some(tx) = tx {
            if tx.try_send(ActorCommand::Shutdown).is_err() {
                // keep fallback behavior and let channel-disconnect drive exit.
            }
            drop(tx);
        }

        let mut worker = self
            .worker
            .lock()
            .map_err(|error| RouterError::Internal(error.to_string()))?;
        if let Some(handle) = worker.take() {
            handle
                .join()
                .map_err(|_| RouterError::Internal("database actor worker panicked".to_owned()))?;
        }
        self.queued.store(0, Ordering::Relaxed);
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
        let (reply_tx, reply_rx) = mpsc::channel();
        self.enqueue(ActorCommand::Route {
            method: method.to_owned(),
            target: target.to_owned(),
            body: body.to_vec(),
            auth_agent_id,
            reply: reply_tx,
        })?;
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
        self.enqueue(ActorCommand::ExpireMemory {
            now_unix_seconds,
            reply: reply_tx,
        })?;
        let result = reply_rx
            .recv()
            .map_err(|_| RouterError::Internal("database actor stopped".to_owned()))?;
        self.requests_completed.fetch_add(1, Ordering::Relaxed);
        result
    }

    #[cfg(test)]
    fn enqueue_test_blocker(
        &self,
        started: mpsc::Sender<()>,
        release: mpsc::Receiver<()>,
    ) -> Result<(), RouterError> {
        self.enqueue(ActorCommand::TestBlock { started, release })
    }

    #[cfg(test)]
    fn enqueue_test_noop(&self) -> Result<(), RouterError> {
        self.enqueue(ActorCommand::TestNoop)
    }
}

impl Drop for DatabaseActor {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

#[cfg(test)]
mod tests {
    use super::DatabaseActor;
    use crate::responses::RouterError;
    use std::sync::mpsc;
    use std::time::Duration;

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
    fn drop_completes_after_full_queue_shutdown_fallback() {
        let dir = tempfile::tempdir().unwrap();
        let actor = DatabaseActor::open_with_capacity(dir.path(), 1).unwrap();
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();

        actor
            .enqueue_test_blocker(started_tx, release_rx)
            .expect("blocker should enqueue");
        started_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("worker should start blocker");
        actor
            .enqueue_test_noop()
            .expect("queue should accept one waiting command");

        let (done_tx, done_rx) = mpsc::channel();
        let dropper = std::thread::spawn(move || {
            drop(actor);
            let _ = done_tx.send(());
        });

        release_tx.send(()).unwrap();
        done_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("actor drop should not hang when shutdown cannot be enqueued");
        dropper.join().unwrap();
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
        assert!(matches!(
            actor.route("GET", "/v1/health", b""),
            Err(RouterError::Internal(_))
        ));
    }
}
