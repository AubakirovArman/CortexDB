#![cfg(feature = "experimental-replication")]

use std::collections::{BTreeMap, BTreeSet};
use std::thread;

use cortex_core::{CellId, CommitSeq};
use cortex_engine::{
    encode_snapshot_segment, Database, DatabaseOptions, ElectionState, EngineFeatureFlags, NodeId,
    ReplicationPeerServer, ReplicationPeerState, SnapshotChunk, SnapshotSegment,
    TcpReplicationTransport, Term,
};
use cortex_storage::segment::SegmentCell;

#[test]
fn partial_peer_snapshot_does_not_replace_follower_after_restart() {
    let dir = tempfile::tempdir().unwrap();
    seed_stale_follower(dir.path());

    let snapshot = snapshot_with_payload(31, 99, b"scope=project\nstatus=ready\n\nnew snapshot");
    let encoded = encode_snapshot_segment(&snapshot).unwrap();
    let split_at = encoded.len() / 2;
    let state = serve_snapshot_install(dir.path(), 1);
    let transport = transport_for(&state.addr);

    let received = transport
        .send_snapshot_chunk(
            NodeId(2),
            &SnapshotChunk {
                term: Term(4),
                leader_id: NodeId(1),
                leader_commit: cortex_engine::LogIndex(31),
                chunk_index: 0,
                last: false,
                payload: encoded[..split_at].to_vec(),
            },
        )
        .unwrap();
    let peer_state = state.join.join().unwrap().unwrap();
    let follower = Database::open(dir.path()).unwrap();

    assert_eq!(received as usize, split_at);
    assert_eq!(peer_state.snapshot, encoded[..split_at]);
    assert!(follower.get_latest_cell(CellId(99)).is_none());
    assert!(follower.get_latest_cell(CellId(7)).is_some());
    assert_eq!(follower.manifest().checkpoint_seq, 0);
}

#[test]
fn follower_can_install_clean_snapshot_after_partial_snapshot_restart() {
    let dir = tempfile::tempdir().unwrap();
    seed_stale_follower(dir.path());

    let snapshot = snapshot_with_payload(32, 99, b"scope=project\nstatus=ready\n\nnew snapshot");
    let encoded = encode_snapshot_segment(&snapshot).unwrap();
    let split_at = encoded.len() / 2;
    let partial = serve_snapshot_install(dir.path(), 1);
    let partial_transport = transport_for(&partial.addr);
    partial_transport
        .send_snapshot_chunk(
            NodeId(2),
            &SnapshotChunk {
                term: Term(4),
                leader_id: NodeId(1),
                leader_commit: cortex_engine::LogIndex(32),
                chunk_index: 0,
                last: false,
                payload: encoded[..split_at].to_vec(),
            },
        )
        .unwrap();
    partial.join.join().unwrap().unwrap();

    let clean = serve_snapshot_install(dir.path(), 2);
    let clean_transport = transport_for(&clean.addr);
    clean_transport
        .send_snapshot_chunk(
            NodeId(2),
            &SnapshotChunk {
                term: Term(5),
                leader_id: NodeId(1),
                leader_commit: cortex_engine::LogIndex(32),
                chunk_index: 0,
                last: false,
                payload: encoded[..split_at].to_vec(),
            },
        )
        .unwrap();
    clean_transport
        .send_snapshot_chunk(
            NodeId(2),
            &SnapshotChunk {
                term: Term(5),
                leader_id: NodeId(1),
                leader_commit: cortex_engine::LogIndex(32),
                chunk_index: 1,
                last: true,
                payload: encoded[split_at..].to_vec(),
            },
        )
        .unwrap();
    clean.join.join().unwrap().unwrap();
    let follower = Database::open(dir.path()).unwrap();

    assert_eq!(
        follower.get_latest_cell(CellId(99)).unwrap(),
        snapshot.cells[0].payload
    );
    assert!(follower.get_latest_cell(CellId(7)).is_none());
    assert_eq!(follower.manifest().checkpoint_seq, 32);
}

#[test]
fn stale_second_snapshot_chunk_does_not_replace_follower() {
    let dir = tempfile::tempdir().unwrap();
    seed_stale_follower(dir.path());

    let snapshot = snapshot_with_payload(33, 99, b"scope=project\nstatus=ready\n\nnew snapshot");
    let encoded = encode_snapshot_segment(&snapshot).unwrap();
    let split_at = encoded.len() / 2;
    let server = serve_snapshot_install(dir.path(), 2);
    let transport = transport_for(&server.addr);
    transport
        .send_snapshot_chunk(
            NodeId(2),
            &SnapshotChunk {
                term: Term(5),
                leader_id: NodeId(1),
                leader_commit: cortex_engine::LogIndex(33),
                chunk_index: 0,
                last: false,
                payload: encoded[..split_at].to_vec(),
            },
        )
        .unwrap();
    let stale = transport.send_snapshot_chunk(
        NodeId(2),
        &SnapshotChunk {
            term: Term(4),
            leader_id: NodeId(1),
            leader_commit: cortex_engine::LogIndex(33),
            chunk_index: 1,
            last: true,
            payload: encoded[split_at..].to_vec(),
        },
    );
    server.join.join().unwrap().unwrap();
    let follower = Database::open(dir.path()).unwrap();

    assert!(stale.is_err());
    assert!(follower.get_latest_cell(CellId(99)).is_none());
    assert!(follower.get_latest_cell(CellId(7)).is_some());
    assert_eq!(follower.manifest().checkpoint_seq, 0);
}

#[test]
fn corrupt_final_snapshot_chunk_does_not_replace_follower() {
    let dir = tempfile::tempdir().unwrap();
    seed_stale_follower(dir.path());

    let snapshot = snapshot_with_payload(34, 99, b"scope=project\nstatus=ready\n\nnew snapshot");
    let mut encoded = encode_snapshot_segment(&snapshot).unwrap();
    let split_at = encoded.len() / 2;
    let last = encoded.last_mut().unwrap();
    *last ^= 0xff;
    let server = serve_snapshot_install(dir.path(), 2);
    let transport = transport_for(&server.addr);
    transport
        .send_snapshot_chunk(
            NodeId(2),
            &SnapshotChunk {
                term: Term(6),
                leader_id: NodeId(1),
                leader_commit: cortex_engine::LogIndex(34),
                chunk_index: 0,
                last: false,
                payload: encoded[..split_at].to_vec(),
            },
        )
        .unwrap();
    let corrupt = transport.send_snapshot_chunk(
        NodeId(2),
        &SnapshotChunk {
            term: Term(6),
            leader_id: NodeId(1),
            leader_commit: cortex_engine::LogIndex(34),
            chunk_index: 1,
            last: true,
            payload: encoded[split_at..].to_vec(),
        },
    );
    assert!(server.join.join().unwrap().is_err());
    let follower = Database::open(dir.path()).unwrap();

    assert!(corrupt.is_err());
    assert!(follower.get_latest_cell(CellId(99)).is_none());
    assert!(follower.get_latest_cell(CellId(7)).is_some());
    assert_eq!(follower.manifest().checkpoint_seq, 0);
}

struct SnapshotServer {
    addr: String,
    join: thread::JoinHandle<cortex_engine::EngineResult<ReplicationPeerState>>,
}

fn serve_snapshot_install(path: &std::path::Path, requests: usize) -> SnapshotServer {
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
    let path = path.to_owned();
    let join = thread::spawn(move || {
        let mut follower = open_replication_database(&path)?;
        let result = server.serve_n_with_snapshot_install(requests, &mut follower);
        let close_result = follower.close();
        result?;
        close_result?;
        server.state()
    });
    SnapshotServer { addr, join }
}

fn transport_for(addr: &str) -> TcpReplicationTransport {
    TcpReplicationTransport::with_token(
        BTreeMap::from([(NodeId(2), addr.to_owned())]),
        "secret".into(),
    )
}

fn seed_stale_follower(path: &std::path::Path) {
    let mut follower = Database::open(path).unwrap();
    follower
        .put_cell(
            CellId(7),
            b"scope=old\nstatus=stale\n\nstale follower state".to_vec(),
        )
        .unwrap();
    follower.close().unwrap();
}

fn open_replication_database(path: &std::path::Path) -> cortex_engine::EngineResult<Database> {
    Database::open_with_options(
        path,
        DatabaseOptions {
            feature_flags: EngineFeatureFlags::production_safe()
                .with_experimental_replication(true),
            ..DatabaseOptions::default()
        },
    )
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
        }
        .into()],
    }
}

fn follower_state() -> ElectionState {
    let mut state =
        ElectionState::new(NodeId(2), BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)]));
    state.current_term = Term(1);
    state
}
