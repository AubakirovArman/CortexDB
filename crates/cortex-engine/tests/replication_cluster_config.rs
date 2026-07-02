use cortex_engine::{
    ClusterConfig, ClusterNode, EngineError, NodeId, ReplicationFollowerProgressStore,
    ReplicationPathConfig,
};

#[test]
fn cluster_config_places_replication_paths_under_local_node_scope() {
    let cluster = two_node_cluster();

    let paths = cluster.replication_paths("/var/lib/cortexdb").unwrap();

    assert_eq!(paths.local_node(), NodeId(2));
    assert!(paths
        .cluster_config_path()
        .ends_with("replication/node-2.cluster.conf"));
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
fn cluster_config_store_load_roundtrips_operator_topology() {
    let dir = tempfile::tempdir().unwrap();
    let cluster = two_node_cluster();
    let path = cluster
        .replication_paths(dir.path())
        .unwrap()
        .cluster_config_path();

    cluster.store(&path).unwrap();
    let loaded = ClusterConfig::load(&path).unwrap();

    assert_eq!(loaded, cluster);
    assert!(path.exists());
}

#[test]
fn cluster_config_roundtrips_optional_ingress_addresses() {
    let dir = tempfile::tempdir().unwrap();
    let mut cluster = two_node_cluster();
    cluster.nodes[0].ingress_address = Some("127.0.0.1:8101".to_owned());
    let path = cluster
        .replication_paths(dir.path())
        .unwrap()
        .cluster_config_path();

    cluster.store(&path).unwrap();
    let loaded = ClusterConfig::load(&path).unwrap();

    assert_eq!(loaded, cluster);
    assert_eq!(loaded.nodes[0].address, "127.0.0.1:9101");
    assert_eq!(loaded.nodes[0].ingress_address(), "127.0.0.1:8101");
    assert_eq!(loaded.nodes[1].ingress_address(), "127.0.0.1:9102");
}

#[test]
fn cluster_config_load_rejects_bad_magic_and_invalid_topology() {
    let dir = tempfile::tempdir().unwrap();
    let bad_magic = dir.path().join("bad-magic.conf");
    std::fs::write(&bad_magic, "not-a-cluster-config\n").unwrap();
    assert!(ClusterConfig::load(&bad_magic).is_err());

    let invalid = dir.path().join("invalid.conf");
    std::fs::write(
        &invalid,
        "CORTEXDB_CLUSTER_CONFIG_V1\nlocal_node 2\nreplication_factor 1\nnode 1 127.0.0.1:1\n",
    )
    .unwrap();
    assert!(ClusterConfig::load(&invalid).is_err());
}

#[test]
fn cluster_config_store_rejects_invalid_topology_without_writing_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cluster.conf");
    let cluster = ClusterConfig {
        local_node: NodeId(1),
        nodes: vec![ClusterNode {
            id: NodeId(1),
            address: String::new(),
            ingress_address: None,
        }],
        replication_factor: 1,
    };

    assert!(cluster.store(&path).is_err());
    assert!(!path.exists());
}

#[test]
fn cluster_config_rejects_missing_local_node() {
    let cluster = ClusterConfig {
        local_node: NodeId(3),
        nodes: vec![ClusterNode {
            id: NodeId(1),
            address: "127.0.0.1:9101".to_owned(),
            ingress_address: None,
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
                ingress_address: None,
            },
            ClusterNode {
                id: NodeId(1),
                address: "127.0.0.1:9102".to_owned(),
                ingress_address: None,
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

fn two_node_cluster() -> ClusterConfig {
    ClusterConfig {
        local_node: NodeId(2),
        nodes: vec![
            ClusterNode {
                id: NodeId(1),
                address: "127.0.0.1:9101".to_owned(),
                ingress_address: None,
            },
            ClusterNode {
                id: NodeId(2),
                address: "127.0.0.1:9102".to_owned(),
                ingress_address: None,
            },
        ],
        replication_factor: 2,
    }
}
