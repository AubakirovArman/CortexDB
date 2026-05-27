use crate::handle_http;

#[test]
fn put_get_and_flush_over_http() {
    let dir = tempfile::tempdir().unwrap();
    let put = "POST /put?cell_id=1 HTTP/1.1\r\ncontent-length: 5\r\n\r\nhello";
    assert!(handle_http(dir.path(), put).contains(r#""seq":1"#));
    let get = "GET /get?cell_id=1 HTTP/1.1\r\n\r\n";
    assert!(handle_http(dir.path(), get).contains(r#""payload":"hello""#));
    let flush = "POST /flush HTTP/1.1\r\ncontent-length: 0\r\n\r\n";
    assert!(handle_http(dir.path(), flush).contains(r#""cells_flushed":1"#));
}

#[test]
fn v1_cell_miss_returns_typed_null_cell() {
    let dir = tempfile::tempdir().unwrap();
    let response = handle_http(dir.path(), "GET /v1/cell?cell_id=99 HTTP/1.1\r\n\r\n");
    assert!(response.contains(r#""cell":null"#));
}

#[test]
fn v1_stats_and_validate_report_storage_state() {
    let dir = tempfile::tempdir().unwrap();
    let put = "POST /v1/cell?cell_id=1 HTTP/1.1\r\ncontent-length: 5\r\n\r\nhello";
    assert!(handle_http(dir.path(), put).contains(r#""seq":1"#));

    let stats = handle_http(dir.path(), "GET /v1/stats HTTP/1.1\r\n\r\n");
    assert!(stats.contains(r#""current_seq":1"#));
    assert!(stats.contains(r#""memtable_cells":1"#));
    assert!(stats.contains(r#""wal_writer_records":0"#));

    let validation = handle_http(dir.path(), "GET /v1/validate HTTP/1.1\r\n\r\n");
    assert!(validation.contains(r#""ok":true"#));
    assert!(validation.contains(r#""vector_indexes_checked":0"#));
    assert!(validation.contains(r#""wal_records_checked":1"#));
}
