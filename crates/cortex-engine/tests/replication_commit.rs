use std::collections::{BTreeMap, BTreeSet};

use cortex_engine::{ConsensusState, LogIndex, NodeId, Term};

#[test]
fn majority_acks_do_not_commit_unknown_index() {
    let mut leader = ConsensusState::new(NodeId(1), voters());

    let decision = leader.record_acks(LogIndex(99), BTreeSet::from([NodeId(1), NodeId(2)]));

    assert!(!decision.committed);
    assert_eq!(leader.commit_index, LogIndex(0));
}

#[test]
fn majority_acks_do_not_directly_commit_prior_term_entry() {
    let mut leader = ConsensusState::new(NodeId(1), voters());
    let old = leader.append_local(b"old-term".to_vec());
    leader.current_term = Term(2);

    let decision = leader.record_acks(old.index, BTreeSet::from([NodeId(1), NodeId(2)]));

    assert!(!decision.committed);
    assert_eq!(leader.commit_index, LogIndex(0));
}

#[test]
fn match_indexes_commit_highest_current_term_entry() {
    let mut leader = ConsensusState::new(NodeId(1), voters());
    leader.append_local(b"first".to_vec());
    let second = leader.append_local(b"second".to_vec());

    let decision = leader.record_match_indexes(BTreeMap::from([
        (NodeId(2), second.index),
        (NodeId(3), LogIndex(1)),
    ]));

    assert!(decision.committed);
    assert_eq!(decision.index, second.index);
    assert_eq!(decision.acknowledgements, 2);
    assert_eq!(leader.commit_index, second.index);
}

#[test]
fn current_term_commit_indirectly_commits_prior_term_prefix() {
    let mut leader = ConsensusState::new(NodeId(1), voters());
    let old = leader.append_local(b"old-term".to_vec());
    leader.current_term = Term(2);
    let current = leader.append_local(b"current-term".to_vec());

    let decision = leader.record_match_indexes(BTreeMap::from([(NodeId(2), current.index)]));

    assert!(decision.committed);
    assert_eq!(leader.commit_index, current.index);
    assert_eq!(leader.committed_entries(), vec![old, current]);
}

#[test]
fn match_indexes_ignore_non_voter_progress() {
    let mut leader = ConsensusState::new(NodeId(1), voters());
    let entry = leader.append_local(b"entry".to_vec());

    let decision = leader.record_match_indexes(BTreeMap::from([(NodeId(99), entry.index)]));

    assert!(!decision.committed);
    assert_eq!(leader.commit_index, LogIndex(0));
}

fn voters() -> BTreeSet<NodeId> {
    BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)])
}
