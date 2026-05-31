use crate::distributed::NodeId;
use crate::error::{EngineError, EngineResult};

use super::ReplicationSnapshotRepairRequest;
use crate::replication::{
    encode_snapshot_segment, LogIndex, SnapshotChunk, SnapshotSegment, TcpReplicationTransport,
    Term,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReplicationSnapshotSendPolicy {
    pub max_chunk_bytes: usize,
}

impl Default for ReplicationSnapshotSendPolicy {
    fn default() -> Self {
        Self {
            max_chunk_bytes: 64 * 1024,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReplicationSnapshotSendResult {
    pub target: NodeId,
    pub checkpoint: LogIndex,
    pub chunks_sent: u64,
    pub bytes_sent: u64,
}

pub trait ReplicationSnapshotTransport {
    fn send_snapshot_chunk(&mut self, target: NodeId, chunk: &SnapshotChunk) -> EngineResult<u64>;
}

impl ReplicationSnapshotTransport for TcpReplicationTransport {
    fn send_snapshot_chunk(&mut self, target: NodeId, chunk: &SnapshotChunk) -> EngineResult<u64> {
        TcpReplicationTransport::send_snapshot_chunk(self, target, chunk)
    }
}

pub fn send_replication_snapshot_request<T: ReplicationSnapshotTransport>(
    transport: &mut T,
    term: Term,
    leader_id: NodeId,
    request: &ReplicationSnapshotRepairRequest,
    snapshot: &SnapshotSegment,
    policy: ReplicationSnapshotSendPolicy,
) -> EngineResult<ReplicationSnapshotSendResult> {
    if policy.max_chunk_bytes == 0 || snapshot.checkpoint_seq.0 != request.checkpoint.0 {
        return Err(EngineError::InvalidOperation);
    }

    let encoded = encode_snapshot_segment(snapshot)?;
    let mut bytes_sent = 0usize;
    let mut chunks_sent = 0u64;
    let total_chunks = encoded.chunks(policy.max_chunk_bytes).count();
    for (index, payload) in encoded.chunks(policy.max_chunk_bytes).enumerate() {
        bytes_sent = bytes_sent
            .checked_add(payload.len())
            .ok_or(EngineError::InvalidOperation)?;
        let chunk = SnapshotChunk {
            term,
            leader_id,
            leader_commit: request.checkpoint,
            chunk_index: index as u64,
            last: index + 1 == total_chunks,
            payload: payload.to_vec(),
        };
        let ack_bytes = transport.send_snapshot_chunk(request.target, &chunk)?;
        if ack_bytes != checked_u64(bytes_sent)? {
            return Err(EngineError::InvalidOperation);
        }
        chunks_sent += 1;
    }

    Ok(ReplicationSnapshotSendResult {
        target: request.target,
        checkpoint: request.checkpoint,
        chunks_sent,
        bytes_sent: checked_u64(bytes_sent)?,
    })
}

fn checked_u64(value: usize) -> EngineResult<u64> {
    u64::try_from(value).map_err(|_| EngineError::InvalidOperation)
}
