use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use cortex_engine::{ClusterConfig, ClusterNode, NodeId};

use crate::cluster::{ClusterIngressMonitor, ContextIngressDecision};
use crate::ServerOptions;

const TEST_PEER_REQUESTS: usize = 32;

#[test]
fn adaptive_ingress_refreshes_leader_when_cached_route_is_over_limit() {
    let reported_leader = Arc::new(AtomicU64::new(2));
    let node_one_raft = start_dynamic_status_peer(NodeId(1), Arc::clone(&reported_leader));
    let node_two_raft = bind_loopback_addr();
    let node_three_raft = bind_loopback_addr();
    let node_one_http = bind_loopback_addr();
    let node_two_http = start_health_server(TEST_PEER_REQUESTS);
    let node_three_http = bind_loopback_addr();
    let options = cluster_options(
        node_one_raft,
        node_one_http,
        node_two_raft,
        node_two_http,
        node_three_raft,
        node_three_http,
    );
    let monitor = ClusterIngressMonitor::with_max_in_flight(configured_cluster(&options), 1);
    assert_eq!(wait_for_cached_leader_id(&monitor), NodeId(2));

    let first = crate::cluster::context_ingress_decision_with_monitor(
        &options,
        Some(&monitor),
        "POST",
        "/v1/context",
    );
    assert!(
        matches!(first, Some(ContextIngressDecision::Forward(ref target)) if target.node_id == 2),
        "first route should use the initially cached leader: {first:?}"
    );

    reported_leader.store(3, Ordering::SeqCst);
    let second = crate::cluster::context_ingress_decision_with_monitor(
        &options,
        Some(&monitor),
        "POST",
        "/v1/context",
    );
    assert!(
        matches!(second, Some(ContextIngressDecision::Local)),
        "over-limit cached leader should trigger refresh and use the current local leader: {second:?}"
    );
    assert_eq!(monitor.cached_leader_node().unwrap().id, NodeId(3));
}

fn cluster_options(
    node_one_raft: SocketAddr,
    node_one_http: SocketAddr,
    node_two_raft: SocketAddr,
    node_two_http: SocketAddr,
    node_three_raft: SocketAddr,
    node_three_http: SocketAddr,
) -> ServerOptions {
    ServerOptions {
        cluster_config: Some(ClusterConfig {
            local_node: NodeId(3),
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
                ClusterNode {
                    id: NodeId(3),
                    address: node_three_raft.to_string(),
                    ingress_address: Some(node_three_http.to_string()),
                },
            ],
            replication_factor: 3,
        }),
        ..ServerOptions::default()
    }
}

fn configured_cluster(options: &ServerOptions) -> ClusterConfig {
    options.cluster_config.clone().unwrap()
}

fn wait_for_cached_leader_id(monitor: &ClusterIngressMonitor) -> NodeId {
    for _ in 0..40 {
        monitor.refresh_once();
        if let Some(node) = monitor.cached_leader_node() {
            return node.id;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    panic!("monitor did not cache leader");
}

fn start_dynamic_status_peer(local_node: NodeId, leader: Arc<AtomicU64>) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        for _ in 0..TEST_PEER_REQUESTS {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0_u8; 512];
            let _ = stream.read(&mut buffer);
            let response = format!(
                "STATUS_RESP 2 {} Follower {}\n",
                local_node.0,
                leader.load(Ordering::SeqCst)
            );
            stream.write_all(response.as_bytes()).unwrap();
        }
    });
    addr
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
