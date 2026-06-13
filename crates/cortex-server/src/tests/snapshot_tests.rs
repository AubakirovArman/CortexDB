//! Snapshot/golden tests for stable API response shapes.
//! These tests guard against accidental JSON schema drift.

use crate::handle_http;

#[test]
fn snapshot_health_response() {
    let dir = tempfile::tempdir().unwrap();
    let response = handle_http(dir.path(), "GET /v1/health HTTP/1.1\r\n\r\n");
    assert!(response.contains(r#""status":"ok""#));
    assert!(response.contains(r#""version":"v1""#));
    assert!(response.contains(r#""server_version":"#));
}

#[test]
fn snapshot_compatibility_response_shape() {
    let dir = tempfile::tempdir().unwrap();
    let response = handle_http(dir.path(), "GET /v1/compatibility HTTP/1.1\r\n\r\n");
    assert!(response.contains(r#""schema_version":"cortexdb.compatibility.v1""#));
    assert!(response.contains(r#""version":"v1""#));
    assert!(response.contains(r#""contract":"sdk-contract.v1""#));
    assert!(response.contains(r#""current_magic":"ACLOGv0""#));
    assert!(response.contains(r#""current_magic":"ACS3""#));
    assert!(response.contains(r#""legacy_magics":["ACS1","ACS2"]"#));
    assert!(response.contains(r#""gate":"make migration-compatibility-check""#));
    assert!(response.contains(r#""schema_version":"cortexdb.migration_registry.v1""#));
    assert!(response.contains(r#""migration_gate":"make migration-compatibility-check""#));
    assert!(response.contains(r#""kind":"segment""#));
    assert!(response.contains(r#""required_gate":"make storage-format-freeze-check""#));
    assert!(response.contains(r#""from":"v0.1.0-core-alpha.5""#));
    assert!(response.contains(r#""to":"v0.2.0-beta.2""#));
}

#[test]
fn snapshot_stats_response_shape() {
    let dir = tempfile::tempdir().unwrap();
    handle_http(dir.path(), "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\nhello");
    let response = handle_http(dir.path(), "GET /v1/stats HTTP/1.1\r\n\r\n");
    // Required fields
    assert!(response.contains(r#""current_seq":"#));
    assert!(response.contains(r#""checkpoint_seq":"#));
    assert!(response.contains(r#""live_segments":"#));
    assert!(response.contains(r#""retired_segments":"#));
    assert!(response.contains(r#""memtable_cells":"#));
    assert!(response.contains(r#""memtable_versions":"#));
    assert!(response.contains(r#""wal_size_bytes":"#));
    assert!(response.contains(r#""wal_writer_records":"#));
    assert!(response.contains(r#""wal_writer_bytes":"#));
    assert!(response.contains(r#""wal_writer_fsyncs":"#));
    assert!(response.contains(r#""wal_writer_batches":"#));
}

#[test]
fn snapshot_validation_response_shape() {
    let dir = tempfile::tempdir().unwrap();
    let response = handle_http(dir.path(), "GET /v1/validate HTTP/1.1\r\n\r\n");
    assert!(response.contains(r#""ok":true"#));
    assert!(response.contains(r#""manifest_ok":"#));
    assert!(response.contains(r#""wal_ok":"#));
    assert!(response.contains(r#""errors":[]"#));
}

#[test]
fn snapshot_cell_lookup_response_shape() {
    let dir = tempfile::tempdir().unwrap();
    handle_http(dir.path(), "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\nhello");
    let response = handle_http(dir.path(), "GET /v1/cell?cell_id=1 HTTP/1.1\r\n\r\n");
    assert!(response.contains(r#""cell":{"#));
    assert!(response.contains(r#""cell_id":1"#));
    assert!(response.contains(r#""payload":"hello""#));
}

#[test]
fn snapshot_cell_miss_response_shape() {
    let dir = tempfile::tempdir().unwrap();
    let response = handle_http(dir.path(), "GET /v1/cell?cell_id=99 HTTP/1.1\r\n\r\n");
    assert!(response.contains(r#""cell":null"#));
}

#[test]
fn snapshot_metrics_includes_actor_and_request_fields() {
    let dir = tempfile::tempdir().unwrap();
    // Make a request first so request_count > 0
    handle_http(dir.path(), "GET /v1/health HTTP/1.1\r\n\r\n");
    let response = handle_http(dir.path(), "GET /v1/metrics HTTP/1.1\r\n\r\n");
    assert!(response.contains(r#""actor_queue_depth":"#));
    assert!(response.contains(r#""actor_queue_capacity":"#));
    assert!(response.contains(r#""request_count":"#));
    assert!(response.contains(r#""request_duration_ms_total":"#));
    assert!(response.contains(r#""request_id_client_provided":"#));
    assert!(response.contains(r#""request_id_generated":"#));
    assert!(response.contains(r#""ann_search_requests":"#));
    assert!(response.contains(r#""ann_fallbacks":"#));
    assert!(response.contains(r#""ann_no_fallback_requests":"#));
    assert!(response.contains(r#""ann_no_fallback_allowed":"#));
    assert!(response.contains(r#""ann_no_fallback_blocked":"#));
    assert!(response.contains(r#""ann_search_latency_ms":"#));
    assert!(response.contains(r#""validation_failures":"#));
    assert!(response.contains(r#""backup_latest_age_seconds":"#));
}

#[test]
fn metrics_prometheus_output_contains_contract_series() {
    let dir = tempfile::tempdir().unwrap();
    let local_addr = spawn_metrics_server(dir.path().join("db"));
    let response = tcp_request(
        local_addr,
        "GET /v1/metrics?format=prometheus HTTP/1.1\r\n\r\n",
    );
    assert!(
        response.contains("200 OK"),
        "metrics request failed: {response}"
    );
    for metric in [
        "cortexdb_current_seq",
        "cortexdb_checkpoint_seq",
        "cortexdb_wal_size_bytes",
        "cortexdb_actor_queue_depth",
        "cortexdb_request_count",
        "cortexdb_request_id_client_provided",
        "cortexdb_request_id_generated",
        "cortexdb_request_id_source_total",
        "cortexdb_ann_search_requests",
        "cortexdb_ann_search_latency_ms_bucket",
        "cortexdb_validation_failures",
        "cortexdb_backup_latest_age_seconds",
        "cortexdb_principal_quota_requests_allowed",
        "cortexdb_principal_quota_queue_rejected",
    ] {
        assert!(response.contains(metric), "missing metric {metric}");
    }
}

fn spawn_metrics_server(root_path: std::path::PathBuf) -> std::net::SocketAddr {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let local_addr = listener.local_addr().unwrap();
    drop(listener);

    std::thread::spawn(move || {
        let _ = crate::serve_with_options(
            &root_path,
            &local_addr.to_string(),
            crate::ServerOptions::default(),
        );
    });
    std::thread::sleep(std::time::Duration::from_millis(100));
    local_addr
}

fn tcp_request(addr: std::net::SocketAddr, request: &str) -> String {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::time::Duration;

    let mut last_err = None;
    for _ in 0..20 {
        match TcpStream::connect(addr) {
            Ok(mut stream) => {
                if let Err(err) = stream.write_all(request.as_bytes()) {
                    last_err = Some(err);
                    std::thread::sleep(Duration::from_millis(50));
                    continue;
                }
                let mut response = [0u8; 32768];
                let read = match stream.read(&mut response) {
                    Ok(read) => read,
                    Err(err) => {
                        last_err = Some(err);
                        std::thread::sleep(Duration::from_millis(50));
                        continue;
                    }
                };
                return String::from_utf8_lossy(&response[..read]).to_string();
            }
            Err(err) => {
                last_err = Some(err);
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    }
    panic!("failed to perform metrics request after retries: {last_err:?}");
}

#[test]
fn snapshot_put_cell_response_shape() {
    let dir = tempfile::tempdir().unwrap();
    let response = handle_http(dir.path(), "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\nhello");
    assert!(response.contains(r#""seq":1"#));
    assert!(response.contains(r#""cell_id":1"#));
}

#[test]
fn snapshot_flush_response_shape() {
    let dir = tempfile::tempdir().unwrap();
    handle_http(dir.path(), "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\nhello");
    let response = handle_http(dir.path(), "POST /v1/flush HTTP/1.1\r\n\r\n");
    assert!(response.contains(r#""checkpoint_seq":"#));
    assert!(response.contains(r#""cells_flushed":"#));
}

#[test]
fn snapshot_ingest_text_response_shape() {
    let dir = tempfile::tempdir().unwrap();
    let response = handle_http(
        dir.path(),
        "POST /v1/ingest/text?scope=default HTTP/1.1\r\n\r\nhello world",
    );
    assert!(response.contains(r#""chunks_ingested":"#));
    assert!(response.contains(r#""facts_ingested":"#));
    assert!(response.contains(r#""rows_ingested":"#));
    assert!(response.contains(r#""first_cell_id":"#));
    assert!(response.contains(r#""validation_report":"#));
}

#[test]
fn snapshot_error_response_shape() {
    let dir = tempfile::tempdir().unwrap();
    let response = handle_http(dir.path(), "GET /v1/cell HTTP/1.1\r\n\r\n");
    assert!(response.contains(r#""error":"#));
    assert!(response.contains(r#""message":"#));
}

#[test]
fn snapshot_context_pack_response_shape() {
    let dir = tempfile::tempdir().unwrap();
    let put = concat!(
        "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\n",
        "scope=project:investments\nstatus=ready\nsource=doc-a\nalpha budget"
    );
    handle_http(dir.path(), put);

    let request = concat!(
        "POST /v1/context?scope=project:investments HTTP/1.1\r\n\r\n",
        "RETRIEVE CONTEXT FOR TASK \"budget\" IN BRAIN investment_projects ",
        "WHERE space = project:investments AND status = \"ready\" LIMIT 10 CANDIDATES;"
    );
    let response = handle_http(dir.path(), request);
    assert!(response.contains(r#""schema_version":"context_pack.v1""#));
    assert!(response.contains(r#""cells":"#));
    assert!(response.contains(r#""token_budget_tokens":"#));
    assert!(response.contains(r#""estimated_tokens":"#));
    assert!(response.contains(r#""truncated":"#));
    assert!(response.contains(r#""citations_required":"#));
    assert!(response.contains(r#""anomalies":"#));
}
