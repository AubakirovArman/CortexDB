use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::{Arc, Mutex};
use std::thread;

use axum::body::{to_bytes, Body};
use axum::extract::State;
use axum::http::{Method, Request, StatusCode};
use cortex_core::CellId;
use cortex_engine::{
    encode_snapshot_segment, Database, DatabaseOptions, ElectionState, EngineFeatureFlags,
    LogIndex, NodeId, ReplicationPeerServer, ReplicationPeerState, SnapshotChunk, SnapshotSegment,
    TcpReplicationTransport, Term,
};
use serde_json::Value;

use crate::config::ReceiptSigningKey;
use crate::state::AppState;
use crate::ServerOptions;

const RECEIPT_SEED: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
const DB_INSTANCE_ID: &str = "dbi_bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

#[tokio::test]
async fn http_raft_arbitrary_node_context_receipts_use_replicated_snapshot() {
    let leader_dir = tempfile::tempdir().unwrap();
    let follower_two_dir = tempfile::tempdir().unwrap();
    let follower_three_dir = tempfile::tempdir().unwrap();
    let stale_dir = tempfile::tempdir().unwrap();

    let snapshot = leader_snapshot(leader_dir.path());
    install_snapshot_over_peer(NodeId(2), follower_two_dir.path(), snapshot.clone());
    install_snapshot_over_peer(NodeId(3), follower_three_dir.path(), snapshot);

    let leader = context_response(leader_dir.path()).await;
    let follower_two = context_response(follower_two_dir.path()).await;
    let follower_three = context_response(follower_three_dir.path()).await;
    let stale = context_response(stale_dir.path()).await;

    for response in [&leader, &follower_two, &follower_three] {
        assert_eq!(cell_ids(response), vec![1, 2]);
        assert_node_receipt(response);
    }
    assert_eq!(cell_ids(&stale), Vec::<u64>::new());
    assert_node_receipt(&stale);

    assert_eq!(
        stable_receipt_commitments(&leader),
        stable_receipt_commitments(&follower_two)
    );
    assert_eq!(
        stable_receipt_commitments(&leader),
        stable_receipt_commitments(&follower_three)
    );
}

async fn context_response(root: &Path) -> Value {
    let request = Request::builder()
        .method(Method::POST)
        .uri("/v1/context?scope=project:investments")
        .body(Body::from(context_query()))
        .unwrap();
    let response = crate::handler::axum_handler(State(app_state(root)), request).await;
    let status = response.status();
    let body = to_bytes(response.into_body(), 2 * 1024 * 1024)
        .await
        .unwrap();
    assert_eq!(
        status,
        StatusCode::OK,
        "unexpected HTTP status: {}",
        String::from_utf8_lossy(&body)
    );
    serde_json::from_slice(&body).unwrap()
}

fn app_state(root: &Path) -> AppState {
    AppState {
        root: root.to_owned(),
        dbs: Arc::new(Mutex::new(BTreeMap::new())),
        options: Arc::new(server_options()),
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

fn server_options() -> ServerOptions {
    ServerOptions {
        receipt_signing_key: Some(
            ReceiptSigningKey::from_seed_hex("http-raft-routing-test", RECEIPT_SEED).unwrap(),
        ),
        db_instance_id: Some(DB_INSTANCE_ID.to_owned()),
        engine_database_options: replication_options(),
        ..ServerOptions::default()
    }
}

fn leader_snapshot(path: &Path) -> SnapshotSegment {
    let mut db = open_replication_db(path);
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nsource=leader-a\nalpha budget approved".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\nsource=leader-b\nalpha approval evidence"
            .to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(99),
        b"scope=private:board\nstatus=ready\nsource=private\nforbidden alpha budget".to_vec(),
    )
    .unwrap();
    let snapshot = db.replication_snapshot_segment().unwrap();
    db.close().unwrap();
    snapshot
}

fn install_snapshot_over_peer(node_id: NodeId, follower_path: &Path, snapshot: SnapshotSegment) {
    let encoded = encode_snapshot_segment(&snapshot).unwrap();
    let split_at = encoded.len() / 2;
    let server = ReplicationPeerServer::bind(
        "127.0.0.1:0",
        ReplicationPeerState {
            election: follower_state(node_id),
            log: Vec::new(),
            snapshot: Vec::new(),
        },
        Some("secret".to_owned()),
    )
    .unwrap();
    let addr = server.local_addr().unwrap().to_string();
    let follower_path = follower_path.to_owned();
    let handle = thread::spawn(move || {
        let mut follower = open_replication_db(&follower_path);
        server
            .serve_n_with_snapshot_install(2, &mut follower)
            .unwrap();
        follower.close().unwrap();
    });
    let transport =
        TcpReplicationTransport::with_token(BTreeMap::from([(node_id, addr)]), "secret".into());
    for (chunk_index, (payload, last)) in [
        (encoded[..split_at].to_vec(), false),
        (encoded[split_at..].to_vec(), true),
    ]
    .into_iter()
    .enumerate()
    {
        transport
            .send_snapshot_chunk(
                node_id,
                &SnapshotChunk {
                    term: Term(2),
                    leader_id: NodeId(1),
                    leader_commit: LogIndex(snapshot.checkpoint_seq.0),
                    chunk_index: chunk_index as u64,
                    last,
                    payload,
                },
            )
            .unwrap();
    }
    handle.join().unwrap();
}

fn follower_state(node_id: NodeId) -> ElectionState {
    let mut state = ElectionState::new(node_id, BTreeSet::from([NodeId(1), NodeId(2), NodeId(3)]));
    state.current_term = Term(1);
    state
}

fn open_replication_db(path: &Path) -> Database {
    Database::open_with_options(path, replication_options()).unwrap()
}

fn replication_options() -> DatabaseOptions {
    DatabaseOptions {
        feature_flags: EngineFeatureFlags::production_safe().with_experimental_replication(true),
        ..DatabaseOptions::default()
    }
}

fn context_query() -> &'static str {
    r#"RETRIEVE CONTEXT FOR TASK "alpha budget approval" IN BRAIN investment_projects
WHERE status = "ready" LIMIT 10 CANDIDATES;"#
}

fn cell_ids(value: &Value) -> Vec<u64> {
    value["cells"]
        .as_array()
        .unwrap()
        .iter()
        .map(|cell| cell["cell_id"].as_u64().unwrap())
        .collect()
}

fn assert_node_receipt(value: &Value) {
    let receipt = &value["accountability_receipt"];
    assert_eq!(receipt["schema_version"], "accountability_receipt.v1");
    assert_eq!(
        receipt["header"]["schema_version"],
        "accountability_receipt.v1"
    );
    assert_eq!(receipt["header"]["key_id"], "http-raft-routing-test");
    assert_eq!(receipt["header"]["db_instance_id"], DB_INSTANCE_ID);
    assert_eq!(
        receipt["header"]["signature"]["public_key_hex"],
        ReceiptSigningKey::from_seed_hex("http-raft-routing-test", RECEIPT_SEED)
            .unwrap()
            .public_key_hex()
    );
    assert!(receipt["leaves"]["access"]
        .as_array()
        .unwrap()
        .iter()
        .all(|leaf| leaf["decision"] == "allowed"));
}

fn stable_receipt_commitments(value: &Value) -> Vec<Value> {
    let header = &value["accountability_receipt"]["header"];
    [
        "access_root",
        "provenance_root",
        "cell_set_root",
        "budget_commitment",
        "conflict_commitment",
        "pack_root",
        "determinism_hash",
        "audit_chain_head",
    ]
    .into_iter()
    .map(|key| header[key].clone())
    .collect()
}
