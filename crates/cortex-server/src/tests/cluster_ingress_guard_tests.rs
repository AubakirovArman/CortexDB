use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::body::{to_bytes, Body};
use axum::extract::State;
use axum::http::{Method, Request, StatusCode};
use cortex_engine::{ClusterConfig, ClusterNode, EngineFeatureFlags, NodeId};

use crate::state::AppState;
use crate::{handle_http_with_options, ReceiptSigningKey, ServerOptions};

const RECEIPT_SEED: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";

#[test]
fn cluster_status_uses_configured_multi_node_topology() {
    let dir = tempfile::tempdir().unwrap();
    let options = cluster_options(NodeId(2));

    let response = handle_http_with_options(
        dir.path(),
        "GET /v1/cluster/status HTTP/1.1\r\n\r\n",
        &options,
    );

    assert!(response.contains("200 OK"));
    assert!(response.contains(r#""local_node":2"#));
    assert!(response.contains(r#""replication_factor":3"#));
    assert!(response.contains(r#""distributed_enabled":true"#));
    assert!(response.contains(r#""address":"127.0.0.1:9103""#));
}

#[tokio::test]
async fn non_primary_context_route_fails_closed_when_primary_is_unavailable() {
    let dir = tempfile::tempdir().unwrap();
    let state = app_state(dir.path(), cluster_options(NodeId(2)));
    let request = Request::builder()
        .method(Method::POST)
        .uri("/v1/context?scope=project:investments")
        .body(Body::from(context_query()))
        .unwrap();

    let response = crate::handler::axum_handler(State(state), request).await;
    let status = response.status();
    let body = to_bytes(response.into_body(), 2 * 1024 * 1024)
        .await
        .unwrap();
    let body = String::from_utf8(body.to_vec()).unwrap();

    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(body.contains(r#""code":"service_unavailable""#));
    assert!(body.contains("live Raft ingress forwarding to node 1"));
}

#[test]
fn non_primary_context_route_forwards_to_live_primary() {
    let primary_dir = tempfile::tempdir().unwrap();
    let follower_dir = tempfile::tempdir().unwrap();
    let primary_addr = bind_loopback_addr();
    let follower_addr = bind_loopback_addr();
    let primary_options = cluster_options_with_addresses(NodeId(1), primary_addr, follower_addr);
    let follower_options = cluster_options_with_addresses(NodeId(2), primary_addr, follower_addr);

    let seed = concat!(
        "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\n",
        "scope=project:investments\nstatus=ready\nsource=primary-live\n",
        "source_url=https://example.test/primary-live\nalpha budget approval"
    );
    let seeded = handle_http_with_options(primary_dir.path(), seed, &primary_options);
    assert!(seeded.contains(r#""seq":1"#), "seed failed: {seeded}");

    let primary_root = primary_dir.path().to_owned();
    std::thread::spawn(move || {
        let _ =
            crate::serve_with_options(&primary_root, &primary_addr.to_string(), primary_options);
    });
    let health = request_full(
        primary_addr,
        "GET /v1/health HTTP/1.1\r\nConnection: close\r\n\r\n",
    );
    assert!(health.contains("200 OK"), "primary health failed: {health}");
    let primary_context = request_full(primary_addr, &context_http_request("primary"));
    assert_primary_context_response(&primary_context);

    let follower_root = follower_dir.path().to_owned();
    std::thread::spawn(move || {
        let _ =
            crate::serve_with_options(&follower_root, &follower_addr.to_string(), follower_options);
    });
    let follower_health = request_full(
        follower_addr,
        "GET /v1/health HTTP/1.1\r\nConnection: close\r\n\r\n",
    );
    assert!(
        follower_health.contains("200 OK"),
        "follower health failed: {follower_health}"
    );
    let response = request_full(follower_addr, &context_http_request("follower"));
    assert_primary_context_response(&response);
}

fn cluster_options(local_node: NodeId) -> ServerOptions {
    cluster_options_with_nodes(
        local_node,
        [1, 2, 3]
            .into_iter()
            .map(|id| ClusterNode {
                id: NodeId(id),
                address: format!("127.0.0.1:910{id}"),
                ingress_address: None,
            })
            .collect(),
        3,
    )
}

fn cluster_options_with_addresses(
    local_node: NodeId,
    primary_addr: SocketAddr,
    follower_addr: SocketAddr,
) -> ServerOptions {
    cluster_options_with_nodes(
        local_node,
        vec![
            ClusterNode {
                id: NodeId(1),
                address: primary_addr.to_string(),
                ingress_address: None,
            },
            ClusterNode {
                id: NodeId(2),
                address: follower_addr.to_string(),
                ingress_address: None,
            },
        ],
        2,
    )
}

fn cluster_options_with_nodes(
    local_node: NodeId,
    nodes: Vec<ClusterNode>,
    replication_factor: usize,
) -> ServerOptions {
    let mut options = ServerOptions {
        cluster_config: Some(ClusterConfig {
            local_node,
            nodes,
            replication_factor,
        }),
        receipt_signing_key: Some(
            ReceiptSigningKey::from_seed_hex("raft-ingress-forwarding-test", RECEIPT_SEED).unwrap(),
        ),
        db_instance_id: Some(format!("dbi_{:064x}", local_node.0)),
        ..ServerOptions::default()
    };
    options.engine_database_options.feature_flags =
        EngineFeatureFlags::production_safe().with_experimental_replication(true);
    options
}

fn app_state(root: &std::path::Path, options: ServerOptions) -> AppState {
    AppState {
        root: root.to_owned(),
        dbs: Arc::new(Mutex::new(BTreeMap::new())),
        options: Arc::new(options),
        cluster_ingress_monitor: None,
        audit_sink: None,
        request_count: Arc::new(AtomicU64::new(0)),
        request_rejected: Arc::new(AtomicU64::new(0)),
        request_timeout: Arc::new(AtomicU64::new(0)),
        request_duration_ms_total: Arc::new(AtomicU64::new(0)),
        request_id_client_provided: Arc::new(AtomicU64::new(0)),
        request_id_generated: Arc::new(AtomicU64::new(0)),
        ann_search_requests: Arc::new(AtomicU64::new(0)),
        ann_fallbacks: Arc::new(AtomicU64::new(0)),
        ann_no_fallback_requests: Arc::new(AtomicU64::new(0)),
        ann_no_fallback_allowed: Arc::new(AtomicU64::new(0)),
        ann_no_fallback_blocked: Arc::new(AtomicU64::new(0)),
        ann_search_latency_ms: crate::metrics::LatencyHistogram::new(),
        actor_queue_wait_latency_ms: crate::metrics::LatencyHistogram::new(),
        validation_failures: Arc::new(AtomicU64::new(0)),
        principal_quota_requests_allowed: Arc::new(AtomicU64::new(0)),
        principal_quota_requests_rejected: Arc::new(AtomicU64::new(0)),
        principal_quota_body_bytes_allowed: Arc::new(AtomicU64::new(0)),
        principal_quota_body_bytes_rejected: Arc::new(AtomicU64::new(0)),
        principal_quota_queue_acquired: Arc::new(AtomicU64::new(0)),
        principal_quota_queue_rejected: Arc::new(AtomicU64::new(0)),
        compactions_triggered: Arc::new(AtomicU64::new(0)),
        compactions_completed: Arc::new(AtomicU64::new(0)),
        compaction_duration_ms_total: Arc::new(AtomicU64::new(0)),
        compaction_cells_compacted: Arc::new(AtomicU64::new(0)),
        compaction_input_bytes: Arc::new(AtomicU64::new(0)),
        compaction_paused: Arc::new(AtomicBool::new(false)),
        rate_limit: None,
        principal_rate_limits: Default::default(),
        tenant_queue_limits: Default::default(),
    }
}

fn context_query() -> &'static str {
    r#"RETRIEVE CONTEXT FOR TASK "alpha budget approval" IN BRAIN investment_projects
WHERE status = "ready" LIMIT 10 CANDIDATES;"#
}

fn context_http_request(host: &str) -> String {
    let body = context_query();
    format!(
        "POST /v1/context?scope=project:investments HTTP/1.1\r\n\
         Host: {host}\r\n\
         Content-Type: text/plain\r\n\
         Connection: close\r\n\
         Content-Length: {}\r\n\r\n{}",
        body.len(),
        body
    )
}

fn assert_primary_context_response(response: &str) {
    assert!(
        response.contains("200 OK"),
        "context request failed: {response}"
    );
    let value = response_body_json(response);
    assert_eq!(
        value["cells"][0]["cell_id"], 1,
        "context response missing primary cell: {response}"
    );
    assert_eq!(
        value["cells"][0]["citation"], "primary-live",
        "context response missing primary citation: {response}"
    );
    assert_eq!(
        value["accountability_receipt"]["header"]["key_id"], "raft-ingress-forwarding-test",
        "context response missing forwarded receipt: {response}"
    );
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
