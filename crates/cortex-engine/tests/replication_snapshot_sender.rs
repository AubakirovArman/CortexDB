use std::collections::{BTreeMap, BTreeSet};
use std::thread;

use cortex_core::{CellId, CommitSeq};
use cortex_engine::{
    assemble_snapshot_chunks, decode_snapshot_segment, send_replication_snapshot_request, Database,
    ElectionState, EngineResult, LogIndex, NodeId, ReplicationPeerServer, ReplicationPeerState,
    ReplicationSnapshotRepairRequest, ReplicationSnapshotSendPolicy, ReplicationSnapshotTransport,
    SnapshotChunk, SnapshotSegment, TcpReplicationTransport, Term,
};
use cortex_storage::segment::SegmentCell;

#[test]
fn snapshot_sender_chunks_snapshot_and_requires_cumulative_acks() {
    let snapshot = snapshot_with_payload(42, 99, b"scope=project\nstatus=ready\n\nlarge payload");
    let request = ReplicationSnapshotRepairRequest {
        target: NodeId(2),
        checkpoint: LogIndex(42),
    };
    let mut transport = RecordingSnapshotTransport::default();

    let result = send_replication_snapshot_request(
        &mut transport,
        Term(7),
        NodeId(1),
        &request,
        &snapshot,
        ReplicationSnapshotSendPolicy {
            max_chunk_bytes: 17,
        },
    )
    .unwrap();

    assert!(result.chunks_sent > 1);
    assert_eq!(result.target, NodeId(2));
    assert_eq!(result.checkpoint, LogIndex(42));
    assert!(transport.chunks.last().unwrap().last);
    let reassembled = assemble_snapshot_chunks(&transport.chunks).unwrap();
    let decoded = decode_snapshot_segment(&reassembled).unwrap();
    assert_eq!(decoded, snapshot);
}

#[test]
fn snapshot_sender_rejects_checkpoint_mismatch_and_bad_ack() {
    let snapshot = snapshot_with_payload(42, 99, b"scope=project\nstatus=ready\n\npayload");
    let mut transport = RecordingSnapshotTransport::default();
    let mismatch = send_replication_snapshot_request(
        &mut transport,
        Term(7),
        NodeId(1),
        &ReplicationSnapshotRepairRequest {
            target: NodeId(2),
            checkpoint: LogIndex(41),
        },
        &snapshot,
        ReplicationSnapshotSendPolicy::default(),
    );
    assert!(mismatch.is_err());
    assert!(transport.chunks.is_empty());

    transport.bad_ack = true;
    let bad_ack = send_replication_snapshot_request(
        &mut transport,
        Term(7),
        NodeId(1),
        &ReplicationSnapshotRepairRequest {
            target: NodeId(2),
            checkpoint: LogIndex(42),
        },
        &snapshot,
        ReplicationSnapshotSendPolicy {
            max_chunk_bytes: 16,
        },
    );
    assert!(bad_ack.is_err());
}

#[test]
fn snapshot_sender_installs_snapshot_over_tcp_peer() {
    let dir = tempfile::tempdir().unwrap();
    let snapshot = snapshot_with_payload(43, 99, b"scope=project\nstatus=ready\n\nnew snapshot");
    let request = ReplicationSnapshotRepairRequest {
        target: NodeId(2),
        checkpoint: LogIndex(43),
    };
    let server = ReplicationPeerServer::bind(
        "127.0.0.1:0",
        ReplicationPeerState {
            election: follower_state(),
            log: Vec::new(),
            snapshot: Vec::new(),
        },
        Some("secret".to_owned()),
    )
    .unwrap();
    let addr = server.local_addr().unwrap().to_string();
    let path = dir.path().to_owned();
    let join = thread::spawn(move || {
        let mut follower = Database::open(&path)?;
        let result = server.serve_n_with_snapshot_install(1, &mut follower);
        let close_result = follower.close();
        result?;
        close_result
    });
    let mut transport =
        TcpReplicationTransport::with_token(BTreeMap::from([(NodeId(2), addr)]), "secret".into());

    let result = send_replication_snapshot_request(
        &mut transport,
        Term(7),
        NodeId(1),
        &request,
        &snapshot,
        ReplicationSnapshotSendPolicy::default(),
    )
    .unwrap();
    join.join().unwrap().unwrap();
    let follower = Database::open(dir.path()).unwrap();

    assert_eq!(result.chunks_sent, 1);
    assert_eq!(
        follower.get_latest_cell(CellId(99)).unwrap(),
        snapshot.cells[0].payload
    );
    assert_eq!(follower.manifest().checkpoint_seq, 43);
}

#[derive(Default)]
struct RecordingSnapshotTransport {
    chunks: Vec<SnapshotChunk>,
    received: u64,
    bad_ack: bool,
}

impl ReplicationSnapshotTransport for RecordingSnapshotTransport {
    fn send_snapshot_chunk(&mut self, target: NodeId, chunk: &SnapshotChunk) -> EngineResult<u64> {
        assert_eq!(target, NodeId(2));
        self.received += chunk.payload.len() as u64;
        self.chunks.push(chunk.clone());
        Ok(if self.bad_ack {
            self.received.saturating_sub(1)
        } else {
            self.received
        })
    }
}

fn snapshot_with_payload(seq: u64, cell_id: u64, payload: &[u8]) -> SnapshotSegment {
    SnapshotSegment {
        checkpoint_seq: CommitSeq(seq),
        cells: vec![SegmentCell {
            candidate_id: 1,
            cell_id,
            created_seq: seq,
            deleted_seq: None,
            payload: payload.to_vec(),
        }],
    }
}

fn follower_state() -> ElectionState {
    let mut state =
        ElectionState::new(NodeId(2), BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)]));
    state.current_term = Term(1);
    state
}
