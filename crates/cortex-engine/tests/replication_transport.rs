use std::collections::BTreeSet;

use cortex_engine::{
    AppendEntriesRequest, ConsensusState, ElectionRole, ElectionState,
    InMemoryReplicationTransport, LogIndex, NodeId, ReplicationTransport, Term,
};

#[test]
fn election_state_wins_majority_vote() {
    let voters = voters();
    let mut candidate = ElectionState::new(NodeId(1), voters.clone());
    let mut transport = transport_with_followers(&voters, &[NodeId(2), NodeId(3)]);
    let request = candidate.start_election();

    let first = transport.request_vote(NodeId(2), request.clone()).unwrap();
    let second = transport.request_vote(NodeId(3), request).unwrap();

    let outcome = candidate.record_vote(first);
    assert!(outcome.elected);
    assert_eq!(outcome.role, ElectionRole::Leader);
    assert_eq!(outcome.leader, Some(NodeId(1)));
    assert_eq!(candidate.record_vote(second).role, ElectionRole::Leader);
}

#[test]
fn stale_candidate_log_is_rejected() {
    let voters = voters();
    let mut follower = ElectionState::new(NodeId(2), voters.clone());
    follower.set_last_log(LogIndex(3), Term(2));
    let mut candidate = ElectionState::new(NodeId(1), voters);
    candidate.set_last_log(LogIndex(2), Term(1));
    let request = candidate.start_election();

    let response = follower.handle_vote_request(&request);

    assert!(!response.vote_granted);
    assert_eq!(response.term, request.term);
}

#[test]
fn append_entries_rejects_stale_term_and_accepts_current_leader() {
    let voters = voters();
    let mut transport = transport_with_followers(&voters, &[NodeId(2)]);
    let mut leader = ConsensusState::new(NodeId(1), voters.clone());
    let entry = leader.append_local(b"put cell".to_vec());

    let stale = transport
        .append_entries(
            NodeId(2),
            AppendEntriesRequest {
                term: Term(0),
                leader_id: NodeId(1),
                entries: vec![entry.clone()],
                leader_commit: LogIndex(0),
            },
        )
        .unwrap();
    assert!(!stale.success);

    let accepted = transport
        .append_entries(
            NodeId(2),
            AppendEntriesRequest {
                term: Term(1),
                leader_id: NodeId(1),
                entries: vec![entry.clone()],
                leader_commit: LogIndex(1),
            },
        )
        .unwrap();
    assert!(accepted.success);
    assert_eq!(accepted.match_index, entry.index);
    assert_eq!(transport.peer_log(NodeId(2)).unwrap(), &[entry]);
}

#[test]
fn replicated_entry_acks_can_commit_consensus_state() {
    let voters = voters();
    let mut leader = ConsensusState::new(NodeId(1), voters.clone());
    let entry = leader.append_local(b"put cell".to_vec());
    let mut transport = transport_with_followers(&voters, &[NodeId(2), NodeId(3)]);
    let acks = transport
        .replicate_to(
            [NodeId(2), NodeId(3)],
            AppendEntriesRequest {
                term: leader.current_term,
                leader_id: NodeId(1),
                entries: vec![entry.clone()],
                leader_commit: LogIndex(0),
            },
        )
        .unwrap();

    let decision = leader.record_acks(entry.index, acks);

    assert!(decision.committed);
    assert_eq!(leader.committed_entries(), vec![entry]);
}

fn voters() -> BTreeSet<NodeId> {
    BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)])
}

fn transport_with_followers(
    voters: &BTreeSet<NodeId>,
    followers: &[NodeId],
) -> InMemoryReplicationTransport {
    let mut transport = InMemoryReplicationTransport::default();
    for follower in followers {
        let mut state = ElectionState::new(*follower, voters.clone());
        state.current_term = Term(1);
        transport.register_peer(state);
    }
    transport
}
