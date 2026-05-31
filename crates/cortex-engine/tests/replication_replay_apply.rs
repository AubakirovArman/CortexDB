use std::collections::BTreeSet;

use cortex_engine::{
    ConsensusReplayApplyResult, ConsensusState, EngineError, LogIndex, NodeId, ReplicatedEntry,
    Term,
};

#[test]
fn replay_apply_is_idempotent_after_recovery() {
    let entries = vec![entry(1, 1, b"first"), entry(2, 2, b"second")];
    let mut consensus = ConsensusState::recover(
        NodeId(1),
        voters(),
        entries.clone(),
        entries.last().unwrap().index,
    );

    for recovered in entries {
        assert_eq!(
            consensus.apply_replayed_entry(recovered).unwrap(),
            ConsensusReplayApplyResult::AlreadyPresent
        );
    }

    let next = consensus.append_local(b"third".to_vec());
    assert_eq!(next.index, LogIndex(3));
    assert_eq!(next.term, Term(2));
}

#[test]
fn replay_apply_appends_contiguous_tail_and_updates_term() {
    let mut consensus = ConsensusState::new(NodeId(1), voters());

    assert_eq!(
        consensus
            .apply_replayed_entry(entry(1, 1, b"first"))
            .unwrap(),
        ConsensusReplayApplyResult::Appended
    );
    assert_eq!(
        consensus
            .apply_replayed_entry(entry(3, 2, b"second"))
            .unwrap(),
        ConsensusReplayApplyResult::Appended
    );

    assert_eq!(consensus.current_term, Term(3));
    assert_eq!(consensus.last_log_index(), LogIndex(2));
    assert_eq!(consensus.last_log_term(), Term(3));
}

#[test]
fn replay_apply_rejects_index_gap() {
    let mut consensus = ConsensusState::new(NodeId(1), voters());

    let result = consensus.apply_replayed_entry(entry(1, 2, b"gap"));

    assert!(matches!(result, Err(EngineError::InvalidOperation)));
    assert!(consensus.entries().is_empty());
}

#[test]
fn replay_apply_rejects_conflicting_duplicate() {
    let mut consensus = ConsensusState::new(NodeId(1), voters());
    consensus
        .apply_replayed_entry(entry(1, 1, b"first"))
        .unwrap();

    let result = consensus.apply_replayed_entry(entry(1, 1, b"different"));

    assert!(matches!(result, Err(EngineError::InvalidOperation)));
    assert_eq!(consensus.entries(), &[entry(1, 1, b"first")]);
}

#[test]
fn replay_apply_rejects_term_regression() {
    let mut consensus = ConsensusState::new(NodeId(1), voters());
    consensus
        .apply_replayed_entry(entry(2, 1, b"first"))
        .unwrap();

    let result = consensus.apply_replayed_entry(entry(1, 2, b"regression"));

    assert!(matches!(result, Err(EngineError::InvalidOperation)));
    assert_eq!(consensus.last_log_term(), Term(2));
}

#[test]
fn replay_apply_rejects_zero_term() {
    let mut consensus = ConsensusState::new(NodeId(1), voters());

    let result = consensus.apply_replayed_entry(entry(0, 1, b"zero"));

    assert!(matches!(result, Err(EngineError::InvalidOperation)));
}

fn entry(term: u64, index: u64, payload: &[u8]) -> ReplicatedEntry {
    ReplicatedEntry {
        term: Term(term),
        index: LogIndex(index),
        payload: payload.to_vec(),
    }
}

fn voters() -> BTreeSet<NodeId> {
    BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)])
}
