use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::time::Duration;

use cortex_engine::{
    ClusterConfig, ClusterNode, ElectionState, NodeId, ReplicationPeerServer, ReplicationPeerState,
    Term,
};

use crate::cluster::{ClusterIngressMonitor, ContextIngressDecision};
use crate::ServerOptions;

const TEST_PEER_REQUESTS: usize = 32;

#[test]
fn load_policy_rejects_second_route_until_first_permit_drops() {
    let node_one_raft = start_status_peer(NodeId(1), NodeId(2));
    let node_two_raft = bind_loopback_addr();
    let node_one_http = bind_loopback_addr();
    let node_two_http = start_health_server(TEST_PEER_REQUESTS);
    let options = cluster_options(
        NodeId(1),
        node_one_raft,
        node_one_http,
        node_two_raft,
        node_two_http,
    );
    let monitor = ClusterIngressMonitor::with_max_in_flight(configured_cluster(&options), 1);
    wait_for_cached_leader(&monitor);

    let first = crate::cluster::context_ingress_decision_with_monitor(
        &options,
        Some(&monitor),
        "POST",
        "/v1/context",
    );
    assert!(matches!(first, Some(ContextIngressDecision::Forward(_))));

    let second = crate::cluster::context_ingress_decision_with_monitor(
        &options,
        Some(&monitor),
        "POST",
        "/v1/context",
    );
    assert!(
        matches!(
            second,
            Some(ContextIngressDecision::Unavailable(ref message))
                if message.contains("over ingress load limit")
        ),
        "second route should fail closed while the first permit is live: {second:?}"
    );

    drop(first);
    let after_release = crate::cluster::context_ingress_decision_with_monitor(
        &options,
        Some(&monitor),
        "POST",
        "/v1/context",
    );
    assert!(matches!(
        after_release,
        Some(ContextIngressDecision::Forward(_))
    ));
}

#[test]
fn load_policy_uses_operator_configured_limit_from_options() {
    let node_one_raft = start_status_peer(NodeId(1), NodeId(2));
    let node_two_raft = bind_loopback_addr();
    let node_one_http = bind_loopback_addr();
    let node_two_http = start_health_server(TEST_PEER_REQUESTS);
    let mut options = cluster_options(
        NodeId(1),
        node_one_raft,
        node_one_http,
        node_two_raft,
        node_two_http,
    );
    options.cluster_ingress_max_in_flight_per_node = 1;
    let monitor = ClusterIngressMonitor::from_options(&options).unwrap();
    wait_for_cached_leader(&monitor);

    let first = crate::cluster::context_ingress_decision_with_monitor(
        &options,
        Some(&monitor),
        "POST",
        "/v1/context",
    );
    assert!(matches!(first, Some(ContextIngressDecision::Forward(_))));

    let second = crate::cluster::context_ingress_decision_with_monitor(
        &options,
        Some(&monitor),
        "POST",
        "/v1/context",
    );
    assert!(
        matches!(
            second,
            Some(ContextIngressDecision::Unavailable(ref message))
                if message.contains("over ingress load limit")
        ),
        "operator configured limit should reject the second live route: {second:?}"
    );
}

#[test]
fn load_policy_metrics_report_cached_leader_limit_and_in_flight() {
    let node_one_raft = start_status_peer(NodeId(1), NodeId(2));
    let node_two_raft = bind_loopback_addr();
    let node_one_http = bind_loopback_addr();
    let node_two_http = start_health_server(TEST_PEER_REQUESTS);
    let options = cluster_options(
        NodeId(1),
        node_one_raft,
        node_one_http,
        node_two_raft,
        node_two_http,
    );
    let monitor = ClusterIngressMonitor::with_max_in_flight(configured_cluster(&options), 2);
    wait_for_cached_leader(&monitor);

    let first = crate::cluster::context_ingress_decision_with_monitor(
        &options,
        Some(&monitor),
        "POST",
        "/v1/context",
    );
    assert!(matches!(first, Some(ContextIngressDecision::Forward(_))));

    let metrics = monitor.load_metrics();
    assert_eq!(metrics.cached_leader_id, Some(NodeId(2)));
    assert_eq!(metrics.max_in_flight_per_node, 2);
    assert_eq!(metrics.in_flight_for_cached_leader, 1);
    assert_eq!(metrics.available_permits_for_cached_leader, 1);

    drop(first);
    let after_release = monitor.load_metrics();
    assert_eq!(after_release.in_flight_for_cached_leader, 0);
    assert_eq!(after_release.available_permits_for_cached_leader, 2);
}

fn cluster_options(
    local_node: NodeId,
    node_one_raft: SocketAddr,
    node_one_http: SocketAddr,
    node_two_raft: SocketAddr,
    node_two_http: SocketAddr,
) -> ServerOptions {
    ServerOptions {
        cluster_config: Some(ClusterConfig {
            local_node,
            nodes: vec![
                ClusterNode {
                    id: NodeId(1),
                    address: node_one_raft.to_string(),
                    ingress_address: Some(node_one_http.to_string()),
                },
                ClusterNode {
                    id: NodeId(2),
                    address: node_two_raft.to_string(),
                    ingress_address: Some(node_two_http.to_string()),
                },
            ],
            replication_factor: 2,
        }),
        ..ServerOptions::default()
    }
}

fn configured_cluster(options: &ServerOptions) -> ClusterConfig {
    options.cluster_config.clone().unwrap()
}

fn start_status_peer(local_node: NodeId, leader: NodeId) -> SocketAddr {
    let voters = BTreeSet::from([NodeId(1), NodeId(2)]);
    let mut election = ElectionState::new(local_node, voters);
    assert!(election.accept_leader(Term(2), leader));
    let server = ReplicationPeerServer::bind(
        "127.0.0.1:0",
        ReplicationPeerState {
            election,
            log: Vec::new(),
            snapshot: Vec::new(),
        },
        None,
    )
    .unwrap();
    let addr = server.local_addr().unwrap();
    std::thread::spawn(move || server.serve_n(TEST_PEER_REQUESTS).unwrap());
    addr
}

fn wait_for_cached_leader(monitor: &ClusterIngressMonitor) {
    for _ in 0..40 {
        monitor.refresh_once();
        if monitor.cached_leader_node().is_some() {
            return;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    panic!("monitor did not cache leader");
}

fn start_health_server(requests: usize) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        for _ in 0..requests {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0_u8; 512];
            let _ = stream.read(&mut buffer);
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}")
                .unwrap();
        }
    });
    addr
}

fn bind_loopback_addr() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);
    addr
}
