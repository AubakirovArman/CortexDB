use std::collections::BTreeSet;

use cortex_engine::{ElectionRole, ElectionState, NodeId, Term};

#[test]
fn follower_rejects_conflicting_same_term_leader() {
    let mut follower = ElectionState::new(NodeId(2), voters());
    assert!(follower.accept_leader(Term(3), NodeId(1)));

    assert!(!follower.accept_leader(Term(3), NodeId(3)));
    assert_eq!(follower.leader, Some(NodeId(1)));
    assert_eq!(follower.current_term, Term(3));
    assert_eq!(follower.role, ElectionRole::Follower);
}

#[test]
fn higher_term_leader_replaces_previous_term_leader() {
    let mut follower = ElectionState::new(NodeId(2), voters());
    assert!(follower.accept_leader(Term(3), NodeId(1)));

    assert!(follower.accept_leader(Term(4), NodeId(3)));
    assert_eq!(follower.leader, Some(NodeId(3)));
    assert_eq!(follower.current_term, Term(4));
    assert_eq!(follower.role, ElectionRole::Follower);
}

fn voters() -> BTreeSet<NodeId> {
    BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)])
}
