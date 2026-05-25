use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use crate::error::{EngineError, EngineResult};

use super::election::ElectionState;
use super::tcp::handle_authenticated_replication_frame;
use super::ReplicatedEntry;

#[derive(Clone, Debug)]
pub struct ReplicationPeerState {
    pub election: ElectionState,
    pub log: Vec<ReplicatedEntry>,
    pub snapshot: Vec<u8>,
}

#[derive(Debug)]
pub struct ReplicationPeerServer {
    listener: TcpListener,
    state: Arc<Mutex<ReplicationPeerState>>,
    token: Option<String>,
}

impl ReplicationPeerServer {
    pub fn bind(
        addr: &str,
        state: ReplicationPeerState,
        token: Option<String>,
    ) -> EngineResult<Self> {
        Ok(Self {
            listener: TcpListener::bind(addr)?,
            state: Arc::new(Mutex::new(state)),
            token,
        })
    }

    pub fn local_addr(&self) -> EngineResult<std::net::SocketAddr> {
        Ok(self.listener.local_addr()?)
    }

    pub fn serve_n(&self, requests: usize) -> EngineResult<()> {
        for _ in 0..requests {
            let (mut stream, _) = self.listener.accept()?;
            let mut frame = String::new();
            stream.read_to_string(&mut frame)?;
            let mut state = self
                .state
                .lock()
                .map_err(|_| EngineError::StorageInvariant("replication peer poisoned".into()))?;
            let ReplicationPeerState {
                election,
                log,
                snapshot,
            } = &mut *state;
            let response = handle_authenticated_replication_frame(
                election,
                log,
                snapshot,
                self.token.as_deref(),
                &frame,
            )?;
            stream.write_all(response.as_bytes())?;
        }
        Ok(())
    }

    pub fn state(&self) -> EngineResult<ReplicationPeerState> {
        self.state
            .lock()
            .map(|value| value.clone())
            .map_err(|_| EngineError::StorageInvariant("replication peer poisoned".into()))
    }
}
