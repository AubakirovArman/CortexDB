use super::common::prelude::*;

#[test]
fn cluster_config_places_keys_on_replicas() {
    let cluster = ClusterConfig::single_node();
    let placement = cluster.placement_for_key(42).unwrap();
    assert_eq!(placement.primary.0, 1);
    assert!(cluster.owns_key(42));
}

#[test]
fn consensus_state_commits_only_after_majority_and_recovers_log() {
    let voters = BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)]);
    let mut consensus = ConsensusState::new(NodeId(1), voters.clone());
    let entry = consensus.append_local(b"put cell".to_vec());
    let minority = consensus.record_acks(entry.index, BTreeSet::from([NodeId(1)]));
    assert!(!minority.committed);

    let majority = consensus.record_acks(entry.index, BTreeSet::from([NodeId(1), NodeId(2)]));
    assert!(majority.committed);
    assert_eq!(consensus.committed_entries(), vec![entry.clone()]);

    let recovered = ConsensusState::recover(
        NodeId(1),
        voters,
        vec![ReplicatedEntry {
            term: Term(2),
            index: LogIndex(1),
            payload: b"put cell".to_vec(),
        }],
        LogIndex(1),
    );
    assert_eq!(recovered.current_term, Term(2));
    assert_eq!(recovered.committed_entries().len(), 1);
}
