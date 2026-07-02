use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::time::Duration;

use cortex_engine::{
    ClusterConfig, ClusterNode, ElectionState, EngineFeatureFlags, NodeId, ReplicationPeerServer,
    ReplicationPeerState, Term,
};

use crate::{handle_http_with_options, ReceiptSigningKey, ServerOptions};

const RECEIPT_SEED: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";

#[test]
fn context_route_discovers_raft_leader_then_forwards_to_ingress_address() {
    let node_one_dir = tempfile::tempdir().unwrap();
    let node_two_dir = tempfile::tempdir().unwrap();
    let node_one_http = bind_loopback_addr();
    let node_two_http = bind_loopback_addr();
    let node_one_raft = start_status_peer(NodeId(1), NodeId(2));
    let node_two_raft = bind_loopback_addr();
    let node_one_options = cluster_options(
        NodeId(1),
        node_one_raft,
        node_one_http,
        node_two_raft,
        node_two_http,
    );
    let node_two_options = cluster_options(
        NodeId(2),
        node_one_raft,
        node_one_http,
        node_two_raft,
        node_two_http,
    );

    let seed = concat!(
        "POST /v1/cell?cell_id=3 HTTP/1.1\r\n\r\n",
        "scope=project:investments\nstatus=ready\nsource=discovered-leader\n",
        "source_url=https://example.test/discovered-leader\n",
        "automatic leader discovery budget approval"
    );
    let seeded = handle_http_with_options(node_two_dir.path(), seed, &node_two_options);
    assert!(seeded.contains(r#""seq":1"#), "seed failed: {seeded}");

    let node_two_root = node_two_dir.path().to_owned();
    std::thread::spawn(move || {
        let _ =
            crate::serve_with_options(&node_two_root, &node_two_http.to_string(), node_two_options);
    });
    let health = request_full(
        node_two_http,
        "GET /v1/health HTTP/1.1\r\nConnection: close\r\n\r\n",
    );
    assert!(health.contains("200 OK"), "leader health failed: {health}");

    let node_one_root = node_one_dir.path().to_owned();
    std::thread::spawn(move || {
        let _ =
            crate::serve_with_options(&node_one_root, &node_one_http.to_string(), node_one_options);
    });
    let body = context_query();
    let request = format!(
        "POST /v1/context?scope=project:investments HTTP/1.1\r\n\
         Host: node-one\r\n\
         Content-Type: text/plain\r\n\
         Connection: close\r\n\
         Content-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    let response = request_full(node_one_http, &request);
    assert!(
        response.contains("200 OK"),
        "discovery forward failed: {response}"
    );
    let value = response_body_json(&response);
    assert_eq!(value["cells"][0]["cell_id"], 3);
    assert_eq!(value["cells"][0]["citation"], "discovered-leader");
    assert_eq!(
        value["accountability_receipt"]["header"]["key_id"],
        "raft-ingress-discovery-test"
    );
}

#[test]
fn separate_ingress_config_fails_closed_when_raft_leader_discovery_is_unavailable() {
    let dir = tempfile::tempdir().unwrap();
    let node_one_http = bind_loopback_addr();
    let node_two_http = bind_loopback_addr();
    let options = cluster_options(
        NodeId(1),
        bind_loopback_addr(),
        node_one_http,
        bind_loopback_addr(),
        node_two_http,
    );
    let response = handle_http_with_options(
        dir.path(),
        concat!(
            "POST /v1/context?scope=project:investments HTTP/1.1\r\n\r\n",
            "RETRIEVE CONTEXT FOR TASK \"budget\" IN BRAIN investment_projects ",
            "WHERE status = \"ready\" LIMIT 10 CANDIDATES;"
        ),
        &options,
    );
    assert!(response.contains("503 Service Unavailable"), "{response}");
    assert!(
        response.contains("automatic Raft ingress leader discovery did not find a known leader"),
        "{response}"
    );
}

fn cluster_options(
    local_node: NodeId,
    node_one_raft: SocketAddr,
    node_one_http: SocketAddr,
    node_two_raft: SocketAddr,
    node_two_http: SocketAddr,
) -> ServerOptions {
    let mut options = ServerOptions {
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
        receipt_signing_key: Some(
            ReceiptSigningKey::from_seed_hex("raft-ingress-discovery-test", RECEIPT_SEED).unwrap(),
        ),
        db_instance_id: Some(format!("dbi_{:064x}", local_node.0)),
        ..ServerOptions::default()
    };
    options.engine_database_options.feature_flags =
        EngineFeatureFlags::production_safe().with_experimental_replication(true);
    options
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
    std::thread::spawn(move || server.serve_n(4).unwrap());
    addr
}

fn context_query() -> &'static str {
    r#"RETRIEVE CONTEXT FOR TASK "automatic leader discovery budget approval" IN BRAIN investment_projects
WHERE status = "ready" LIMIT 10 CANDIDATES;"#
}

fn bind_loopback_addr() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);
    addr
}

fn request_full(addr: SocketAddr, request: &str) -> String {
    let mut stream = connect_with_retry(addr);
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    stream.write_all(request.as_bytes()).unwrap();
    let mut response = Vec::new();
    let mut buffer = [0_u8; 4096];
    loop {
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(bytes) => response.extend_from_slice(&buffer[..bytes]),
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                break;
            }
            Err(error) => panic!("failed to read response: {error}"),
        }
    }
    String::from_utf8_lossy(&response).into_owned()
}

fn response_body_json(response: &str) -> serde_json::Value {
    let (_, body) = response.split_once("\r\n\r\n").unwrap();
    serde_json::from_str(body).unwrap()
}

fn connect_with_retry(addr: SocketAddr) -> TcpStream {
    let mut last_error = None;
    for _ in 0..40 {
        match TcpStream::connect(addr) {
            Ok(stream) => return stream,
            Err(error) => {
                last_error = Some(error);
                std::thread::sleep(Duration::from_millis(25));
            }
        }
    }
    panic!("failed to connect to test server: {last_error:?}");
}
