use std::collections::BTreeSet;

use cortex_engine::{
    AppendEntriesRequest, ElectionState, InMemoryReplicationTransport, LogIndex, NodeId,
    ReplicatedEntry, ReplicationTransport, Term,
};

#[test]
fn append_entries_rejects_missing_previous_log_entry() {
    let mut transport = follower_transport();

    let response = transport
        .append_entries(
            NodeId(2),
            AppendEntriesRequest {
                term: Term(1),
                leader_id: NodeId(1),
                prev_log_index: LogIndex(1),
                prev_log_term: Term(1),
                entries: vec![entry(1, 2, b"orphan")],
                leader_commit: LogIndex(0),
            },
        )
        .unwrap();

    assert!(!response.success);
    assert_eq!(response.match_index, LogIndex(0));
    assert_eq!(transport.peer_log(NodeId(2)).unwrap(), &[]);
}

#[test]
fn append_entries_truncates_conflicting_suffix() {
    let mut transport = follower_transport();
    append(&mut transport, LogIndex(0), Term(0), entry(1, 1, b"base"));
    append(&mut transport, LogIndex(1), Term(1), entry(2, 2, b"stale"));

    let response = transport
        .append_entries(
            NodeId(2),
            AppendEntriesRequest {
                term: Term(3),
                leader_id: NodeId(1),
                prev_log_index: LogIndex(1),
                prev_log_term: Term(1),
                entries: vec![entry(2, 3, b"replacement")],
                leader_commit: LogIndex(2),
            },
        )
        .unwrap();

    assert!(response.success);
    assert_eq!(transport.peer_commit(NodeId(2)), Some(LogIndex(2)));
    let log = transport.peer_log(NodeId(2)).unwrap();
    assert_eq!(log.len(), 2);
    assert_eq!(log[1], entry(2, 3, b"replacement"));
}

#[test]
fn append_entries_clamps_commit_to_last_replicated_index() {
    let mut transport = follower_transport();

    let response = transport
        .append_entries(
            NodeId(2),
            AppendEntriesRequest {
                term: Term(1),
                leader_id: NodeId(1),
                prev_log_index: LogIndex(0),
                prev_log_term: Term(0),
                entries: vec![entry(1, 1, b"base")],
                leader_commit: LogIndex(99),
            },
        )
        .unwrap();

    assert!(response.success);
    assert_eq!(response.match_index, LogIndex(1));
    assert_eq!(transport.peer_commit(NodeId(2)), Some(LogIndex(1)));
}

#[test]
fn append_entries_rejects_non_contiguous_entry_indexes() {
    let mut transport = follower_transport();
    append(&mut transport, LogIndex(0), Term(0), entry(1, 1, b"base"));

    let response = transport
        .append_entries(
            NodeId(2),
            AppendEntriesRequest {
                term: Term(1),
                leader_id: NodeId(1),
                prev_log_index: LogIndex(1),
                prev_log_term: Term(1),
                entries: vec![entry(3, 1, b"gap")],
                leader_commit: LogIndex(3),
            },
        )
        .unwrap();

    assert!(!response.success);
    assert_eq!(response.match_index, LogIndex(1));
    assert_eq!(
        transport.peer_log(NodeId(2)).unwrap(),
        &[entry(1, 1, b"base")]
    );
    assert_eq!(transport.peer_commit(NodeId(2)), Some(LogIndex(0)));
}

#[test]
fn append_entries_rejects_out_of_order_entry_indexes() {
    let mut transport = follower_transport();

    let response = transport
        .append_entries(
            NodeId(2),
            AppendEntriesRequest {
                term: Term(1),
                leader_id: NodeId(1),
                prev_log_index: LogIndex(0),
                prev_log_term: Term(0),
                entries: vec![entry(2, 1, b"two"), entry(1, 1, b"one")],
                leader_commit: LogIndex(2),
            },
        )
        .unwrap();

    assert!(!response.success);
    assert_eq!(response.match_index, LogIndex(0));
    assert_eq!(transport.peer_log(NodeId(2)).unwrap(), &[]);
}

fn follower_transport() -> InMemoryReplicationTransport {
    let voters = BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)]);
    let mut state = ElectionState::new(NodeId(2), voters);
    state.current_term = Term(1);
    let mut transport = InMemoryReplicationTransport::default();
    transport.register_peer(state);
    transport
}

fn append(
    transport: &mut InMemoryReplicationTransport,
    prev_index: LogIndex,
    prev_term: Term,
    entry: ReplicatedEntry,
) {
    let response = transport
        .append_entries(
            NodeId(2),
            AppendEntriesRequest {
                term: entry.term,
                leader_id: NodeId(1),
                prev_log_index: prev_index,
                prev_log_term: prev_term,
                entries: vec![entry],
                leader_commit: LogIndex(0),
            },
        )
        .unwrap();
    assert!(response.success);
}

fn entry(index: u64, term: u64, payload: &[u8]) -> ReplicatedEntry {
    ReplicatedEntry {
        term: Term(term),
        index: LogIndex(index),
        payload: payload.to_vec(),
    }
}
