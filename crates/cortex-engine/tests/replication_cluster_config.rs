use cortex_engine::{
    ClusterConfig, ClusterNode, EngineError, NodeId, ReplicationFollowerProgressStore,
    ReplicationPathConfig,
};

#[test]
fn cluster_config_places_replication_paths_under_local_node_scope() {
    let cluster = ClusterConfig {
        local_node: NodeId(2),
        nodes: vec![
            ClusterNode {
                id: NodeId(1),
                address: "127.0.0.1:9101".to_owned(),
            },
            ClusterNode {
                id: NodeId(2),
                address: "127.0.0.1:9102".to_owned(),
            },
        ],
        replication_factor: 2,
    };

    let paths = cluster.replication_paths("/var/lib/cortexdb").unwrap();

    assert_eq!(paths.local_node(), NodeId(2));
    assert!(paths
        .consensus_log_path()
        .ends_with("replication/node-2.consensus.aclog"));
    assert!(paths
        .repair_progress_store_path()
        .ends_with("replication/node-2.repair-progress"));
    assert!(paths
        .snapshot_inbox_dir()
        .ends_with("replication/node-2.snapshots"));
}

#[test]
fn progress_store_default_path_matches_cluster_path_placement() {
    let paths = ReplicationPathConfig::new("/tmp/cortexdb", NodeId(7)).unwrap();

    assert_eq!(
        ReplicationFollowerProgressStore::default_path("/tmp/cortexdb", NodeId(7)),
        paths.repair_progress_store_path()
    );
}

#[test]
fn cluster_config_rejects_missing_local_node() {
    let cluster = ClusterConfig {
        local_node: NodeId(3),
        nodes: vec![ClusterNode {
            id: NodeId(1),
            address: "127.0.0.1:9101".to_owned(),
        }],
        replication_factor: 1,
    };

    assert!(matches!(
        cluster.replication_paths("/tmp/cortexdb"),
        Err(EngineError::InvalidOperation)
    ));
}

#[test]
fn cluster_config_rejects_duplicate_or_invalid_nodes() {
    let duplicate = ClusterConfig {
        local_node: NodeId(1),
        nodes: vec![
            ClusterNode {
                id: NodeId(1),
                address: "127.0.0.1:9101".to_owned(),
            },
            ClusterNode {
                id: NodeId(1),
                address: "127.0.0.1:9102".to_owned(),
            },
        ],
        replication_factor: 2,
    };
    assert!(duplicate.validate_replication_config().is_err());

    let zero = ReplicationPathConfig::new("/tmp/cortexdb", NodeId(0));
    assert!(matches!(zero, Err(EngineError::InvalidOperation)));
}

#[test]
fn single_node_cluster_has_replication_paths() {
    let cluster = ClusterConfig::single_node();
    let paths = cluster.replication_paths("/tmp/cortexdb").unwrap();

    assert!(paths
        .consensus_log_path()
        .ends_with("replication/node-1.consensus.aclog"));
}
