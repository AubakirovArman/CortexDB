use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use crate::database::Database;
use crate::error::{EngineError, EngineResult};

use super::election::ElectionState;
use super::snapshot::{
    assemble_snapshot_chunks, decode_snapshot_chunk, decode_snapshot_segment, SnapshotChunk,
};
use super::tcp::{authenticated_payload, handle_authenticated_replication_frame};
use super::{ReplicatedEntry, Term};

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

    pub fn serve_n_with_snapshot_install(
        &self,
        requests: usize,
        database: &mut Database,
    ) -> EngineResult<()> {
        let mut snapshot_chunks = Vec::<SnapshotChunk>::new();
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
            let response = handle_peer_frame_with_snapshot_install(
                election,
                log,
                snapshot,
                &mut snapshot_chunks,
                database,
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

fn handle_peer_frame_with_snapshot_install(
    state: &mut ElectionState,
    log: &mut Vec<ReplicatedEntry>,
    snapshot: &mut Vec<u8>,
    snapshot_chunks: &mut Vec<SnapshotChunk>,
    database: &mut Database,
    expected_token: Option<&str>,
    frame: &str,
) -> EngineResult<String> {
    let frame = authenticated_payload(expected_token, frame)?;
    let parts = frame.split_whitespace().collect::<Vec<_>>();
    let ["SNAPSHOT", rest @ ..] = parts.as_slice() else {
        return handle_authenticated_replication_frame(state, log, snapshot, None, frame);
    };

    let chunk = decode_snapshot_chunk(rest)?;
    if !state.accept_leader(chunk.term, chunk.leader_id) {
        return Ok(snapshot_response(state.current_term, false, 0));
    }
    if chunk.chunk_index == 0 {
        snapshot.clear();
        snapshot_chunks.clear();
    } else if snapshot.is_empty() || snapshot_chunks.is_empty() {
        return Ok(snapshot_response(state.current_term, false, 0));
    }

    snapshot.extend_from_slice(&chunk.payload);
    snapshot_chunks.push(chunk.clone());
    let received = snapshot.len();
    if chunk.last {
        let encoded = assemble_snapshot_chunks(snapshot_chunks)?;
        let decoded = decode_snapshot_segment(&encoded)?;
        database.install_snapshot_segment(decoded)?;
        snapshot_chunks.clear();
    }
    Ok(snapshot_response(state.current_term, true, received))
}

fn snapshot_response(term: Term, success: bool, bytes: usize) -> String {
    format!("SNAPSHOT_RESP {} {} {}\n", term.0, success as u8, bytes)
}
