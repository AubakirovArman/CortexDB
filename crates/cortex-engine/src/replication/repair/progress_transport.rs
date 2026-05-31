use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use crate::distributed::NodeId;
use crate::error::{EngineError, EngineResult};
use crate::replication::{
    AppendEntriesRequest, AppendEntriesResponse, LogIndex, ReplicationSnapshotTransport,
    ReplicationTransport, SnapshotChunk, Term, VoteRequest, VoteResponse,
};

use super::{ReplicationFollowerProgress, ReplicationFollowerProgressStore};

#[derive(Clone, Debug)]
pub struct ReplicationProgressRecordingTransport<T> {
    inner: T,
    store: Arc<RwLock<ReplicationFollowerProgressStore>>,
    snapshot_bytes: BTreeMap<SnapshotProgressKey, u64>,
}

impl<T> ReplicationProgressRecordingTransport<T> {
    pub fn new(inner: T, store: Arc<RwLock<ReplicationFollowerProgressStore>>) -> Self {
        Self {
            inner,
            store,
            snapshot_bytes: BTreeMap::new(),
        }
    }

    pub fn inner(&self) -> &T {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    pub fn into_inner(self) -> T {
        self.inner
    }

    pub fn store(&self) -> &Arc<RwLock<ReplicationFollowerProgressStore>> {
        &self.store
    }
}

impl<T: ReplicationTransport> ReplicationTransport for ReplicationProgressRecordingTransport<T> {
    fn request_vote(&mut self, target: NodeId, request: VoteRequest) -> EngineResult<VoteResponse> {
        self.inner.request_vote(target, request)
    }

    fn append_entries(
        &mut self,
        target: NodeId,
        request: AppendEntriesRequest,
    ) -> EngineResult<AppendEntriesResponse> {
        let leader_commit = request.leader_commit;
        let response = self.inner.append_entries(target, request)?;
        if response.success {
            record_progress(
                &self.store,
                ReplicationFollowerProgress::new(
                    response.follower,
                    min_index(leader_commit, response.match_index),
                    response.match_index,
                ),
            )?;
        }
        Ok(response)
    }
}

impl<T: ReplicationSnapshotTransport> ReplicationSnapshotTransport
    for ReplicationProgressRecordingTransport<T>
{
    fn send_snapshot_chunk(&mut self, target: NodeId, chunk: &SnapshotChunk) -> EngineResult<u64> {
        let ack_bytes = self.inner.send_snapshot_chunk(target, chunk)?;
        let key = SnapshotProgressKey::new(target, chunk);
        if chunk.chunk_index == 0 {
            self.snapshot_bytes.insert(key, 0);
        }
        let previous = self.snapshot_bytes.get(&key).copied().unwrap_or_default();
        let expected = previous
            .checked_add(
                u64::try_from(chunk.payload.len()).map_err(|_| EngineError::InvalidOperation)?,
            )
            .ok_or(EngineError::InvalidOperation)?;
        if ack_bytes != expected {
            self.snapshot_bytes.remove(&key);
            return Ok(ack_bytes);
        }
        if chunk.last {
            self.snapshot_bytes.remove(&key);
            record_progress(
                &self.store,
                ReplicationFollowerProgress::new(target, chunk.leader_commit, chunk.leader_commit),
            )?;
        } else {
            self.snapshot_bytes.insert(key, expected);
        }
        Ok(ack_bytes)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct SnapshotProgressKey {
    target: NodeId,
    term: Term,
    leader_id: NodeId,
    leader_commit: LogIndex,
}

impl SnapshotProgressKey {
    fn new(target: NodeId, chunk: &SnapshotChunk) -> Self {
        Self {
            target,
            term: chunk.term,
            leader_id: chunk.leader_id,
            leader_commit: chunk.leader_commit,
        }
    }
}

fn record_progress(
    store: &Arc<RwLock<ReplicationFollowerProgressStore>>,
    progress: ReplicationFollowerProgress,
) -> EngineResult<()> {
    store
        .write()
        .map_err(|_| EngineError::InvalidOperation)?
        .record(progress)
}

fn min_index(left: LogIndex, right: LogIndex) -> LogIndex {
    if left <= right {
        left
    } else {
        right
    }
}
