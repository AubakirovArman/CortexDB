#![cfg(feature = "experimental-replication")]

use std::collections::BTreeSet;

use cortex_engine::{
    membership_entry, open_replication_node_runtime, ClusterConfig, ClusterNode, EngineError,
    LogIndex, NodeId, ReplicationFollowerProgress, ReplicationFollowerProgressStore,
    ReplicationLog, Term,
};

#[test]
fn runtime_opens_default_topology_paths_and_reconciles_progress() {
    let dir = tempfile::tempdir().unwrap();
    let cluster = cluster(NodeId(2), &[1, 2, 3]);
    let paths = cluster.replication_paths(dir.path()).unwrap();
    cluster.store(paths.cluster_config_path()).unwrap();
    let mut store =
        ReplicationFollowerProgressStore::open(paths.repair_progress_store_path()).unwrap();
    store
        .record_many([
            ReplicationFollowerProgress::new(NodeId(3), LogIndex(4), LogIndex(5)),
            ReplicationFollowerProgress::new(NodeId(9), LogIndex(1), LogIndex(1)),
        ])
        .unwrap();

    let runtime = open_replication_node_runtime(dir.path(), NodeId(2), LogIndex(0)).unwrap();

    assert_eq!(runtime.cluster, cluster);
    assert_eq!(runtime.paths.local_node(), NodeId(2));
    assert_eq!(
        runtime.consensus.voters,
        BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)])
    );
    assert_eq!(
        runtime.progress_store.progress().get(&NodeId(1)).copied(),
        Some(ReplicationFollowerProgress::new(
            NodeId(1),
            LogIndex(0),
            LogIndex(0)
        ))
    );
    assert_eq!(
        runtime.progress_store.progress().get(&NodeId(3)).copied(),
        Some(ReplicationFollowerProgress::new(
            NodeId(3),
            LogIndex(4),
            LogIndex(5)
        ))
    );
    assert!(!runtime.progress_store.progress().contains_key(&NodeId(2)));
    assert!(!runtime.progress_store.progress().contains_key(&NodeId(9)));
    assert!(runtime.paths.snapshot_inbox_dir().exists());
    runtime.close().unwrap();
}

#[test]
fn runtime_recovers_committed_membership_before_progress_reconcile() {
    let dir = tempfile::tempdir().unwrap();
    let cluster = cluster(NodeId(1), &[1, 2, 3, 4]);
    let paths = cluster.replication_paths(dir.path()).unwrap();
    cluster.store(paths.cluster_config_path()).unwrap();
    let log = ReplicationLog::open(paths.consensus_log_path()).unwrap();
    let committed =
        membership_entry(Term(1), LogIndex(1), BTreeSet::from([NodeId(1), NodeId(4)])).unwrap();
    log.append(&committed).unwrap();
    log.close().unwrap();
    let mut store =
        ReplicationFollowerProgressStore::open(paths.repair_progress_store_path()).unwrap();
    store
        .record_many([
            ReplicationFollowerProgress::new(NodeId(2), LogIndex(1), LogIndex(1)),
            ReplicationFollowerProgress::new(NodeId(4), LogIndex(1), LogIndex(1)),
        ])
        .unwrap();

    let runtime = open_replication_node_runtime(dir.path(), NodeId(1), LogIndex(1)).unwrap();

    assert_eq!(
        runtime.consensus.voters,
        BTreeSet::from([NodeId(1), NodeId(4)])
    );
    assert_eq!(runtime.consensus.commit_index, LogIndex(1));
    assert_eq!(runtime.progress_store.progress().len(), 1);
    assert!(runtime.progress_store.progress().contains_key(&NodeId(4)));
    assert!(!runtime.progress_store.progress().contains_key(&NodeId(2)));
    runtime.close().unwrap();
}

#[test]
fn runtime_rejects_commit_index_beyond_recovered_log() {
    let dir = tempfile::tempdir().unwrap();
    let cluster = cluster(NodeId(1), &[1, 2, 3]);
    let paths = cluster.replication_paths(dir.path()).unwrap();
    cluster.store(paths.cluster_config_path()).unwrap();

    let result = open_replication_node_runtime(dir.path(), NodeId(1), LogIndex(1));

    assert!(matches!(result, Err(EngineError::InvalidOperation)));
}

fn cluster(local_node: NodeId, nodes: &[u64]) -> ClusterConfig {
    ClusterConfig {
        local_node,
        nodes: nodes
            .iter()
            .map(|id| ClusterNode {
                id: NodeId(*id),
                address: format!("127.0.0.1:91{id:02}"),
                ingress_address: None,
            })
            .collect(),
        replication_factor: nodes.len(),
    }
}
