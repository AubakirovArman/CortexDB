//! Snapshot/golden tests for stable API response shapes.
//! These tests guard against accidental JSON schema drift.

use crate::handle_http;

#[test]
fn snapshot_health_response() {
    let dir = tempfile::tempdir().unwrap();
    let response = handle_http(dir.path(), "GET /v1/health HTTP/1.1\r\n\r\n");
    assert!(response.contains(r#""status":"ok""#));
    assert!(response.contains(r#""version":"v1""#));
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
    assert!(response.contains(r#""cells":"#));
    assert!(response.contains(r#""token_budget_tokens":"#));
    assert!(response.contains(r#""estimated_tokens":"#));
    assert!(response.contains(r#""truncated":"#));
    assert!(response.contains(r#""citations_required":"#));
    assert!(response.contains(r#""anomalies":"#));
}
