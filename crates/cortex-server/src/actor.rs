use std::path::Path;
use std::sync::{mpsc, Mutex, RwLock};
use std::thread::JoinHandle;

use cortex_engine::Database;

use crate::router::route_shared;

enum ActorCommand {
    Route {
        method: String,
        target: String,
        body: Vec<u8>,
        reply: mpsc::Sender<Result<String, String>>,
    },
    Shutdown,
}

pub struct DatabaseActor {
    tx: mpsc::Sender<ActorCommand>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

impl DatabaseActor {
    pub fn open(path: &Path) -> std::io::Result<Self> {
        let db = Database::open(path).map_err(|error| std::io::Error::other(error.to_string()))?;
        let (tx, rx) = mpsc::channel::<ActorCommand>();
        let worker = std::thread::spawn(move || {
            let db = RwLock::new(db);
            while let Ok(command) = rx.recv() {
                match command {
                    ActorCommand::Route {
                        method,
                        target,
                        body,
                        reply,
                    } => {
                        let result = route_shared(&db, &method, &target, &body);
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

    pub fn route(&self, method: &str, target: &str, body: &[u8]) -> Result<String, String> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx
            .send(ActorCommand::Route {
                method: method.to_owned(),
                target: target.to_owned(),
                body: body.to_vec(),
                reply: reply_tx,
            })
            .map_err(|_| "database actor stopped".to_owned())?;
        reply_rx
            .recv()
            .map_err(|_| "database actor stopped".to_owned())?
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
