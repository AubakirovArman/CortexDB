use std::path::Path;
use std::sync::{mpsc, Mutex};
use std::thread::JoinHandle;

use cortex_engine::Database;

use crate::responses::RouterError;
use crate::router::route_database;

const ACTOR_QUEUE_CAPACITY: usize = 1024;

enum ActorCommand {
    Route {
        method: String,
        target: String,
        body: Vec<u8>,
        reply: mpsc::Sender<Result<String, RouterError>>,
    },
    Shutdown,
}

pub struct DatabaseActor {
    tx: mpsc::SyncSender<ActorCommand>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

impl DatabaseActor {
    pub fn open(path: &Path) -> std::io::Result<Self> {
        let db = Database::open(path).map_err(|error| std::io::Error::other(error.to_string()))?;
        let (tx, rx) = mpsc::sync_channel::<ActorCommand>(ACTOR_QUEUE_CAPACITY);
        let worker = std::thread::spawn(move || {
            let mut db = db;
            while let Ok(command) = rx.recv() {
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
                    ActorCommand::Shutdown => break,
                }
            }
        });

        Ok(Self {
            tx,
            worker: Mutex::new(Some(worker)),
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
            Ok(()) => {}
            Err(mpsc::TrySendError::Full(_)) => {
                return Err(RouterError::ServiceUnavailable);
            }
            Err(mpsc::TrySendError::Disconnected(_)) => {
                return Err(RouterError::Internal("database actor stopped".to_owned()));
            }
        }
        reply_rx
            .recv()
            .map_err(|_| RouterError::Internal("database actor stopped".to_owned()))?
    }
}

impl Drop for DatabaseActor {
    fn drop(&mut self) {
        let _ = self.tx.send(ActorCommand::Shutdown);
        if let Ok(mut worker) = self.worker.lock() {
            if let Some(handle) = worker.take() {
                let _ = handle.join();
            }
        }
    }
}
