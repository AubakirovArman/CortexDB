use super::{handle_http, handle_http_with_options, ServerOptions};

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
fn v1_api_requires_bearer_token_when_configured() {
    let dir = tempfile::tempdir().unwrap();
    let options = ServerOptions {
        auth_token: Some("secret".to_owned()),
    };
    let denied = handle_http_with_options(dir.path(), "GET /v1/health HTTP/1.1\r\n\r\n", &options);
    assert!(denied.contains("401 Unauthorized"));

    let allowed = handle_http_with_options(
        dir.path(),
        "GET /v1/health HTTP/1.1\r\nAuthorization: Bearer secret\r\n\r\n",
        &options,
    );
    assert!(allowed.contains(r#""status":"ok""#));
}

#[test]
fn v1_stats_and_validate_report_storage_state() {
    let dir = tempfile::tempdir().unwrap();
    let put = "POST /v1/cell?cell_id=1 HTTP/1.1\r\ncontent-length: 5\r\n\r\nhello";
    assert!(handle_http(dir.path(), put).contains(r#""seq":1"#));

    let stats = handle_http(dir.path(), "GET /v1/stats HTTP/1.1\r\n\r\n");
    assert!(stats.contains(r#""current_seq":1"#));
    assert!(stats.contains(r#""memtable_cells":1"#));

    let validation = handle_http(dir.path(), "GET /v1/validate HTTP/1.1\r\n\r\n");
    assert!(validation.contains(r#""ok":true"#));
    assert!(validation.contains(r#""wal_records_checked":1"#));
}

#[test]
fn v1_context_returns_context_pack() {
    let dir = tempfile::tempdir().unwrap();
    let put = concat!(
        "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\n",
        "scope=project:investments\nstatus=ready\nsource=doc-a\nalpha budget"
    );
    assert!(handle_http(dir.path(), put).contains(r#""seq":1"#));

    let request = concat!(
        "POST /v1/context?scope=project:investments HTTP/1.1\r\n\r\n",
        "RETRIEVE CONTEXT FOR TASK \"budget\" IN BRAIN investment_projects ",
        "WHERE space = project:investments AND status = \"ready\" LIMIT 10 CANDIDATES;"
    );
    let response = handle_http(dir.path(), request);
    assert!(response.contains(r#""cells":[{"cell_id":1"#));
    assert!(response.contains(r#""citation":"doc-a""#));
}

#[test]
fn v1_aql_returns_retrieved_cells() {
    let dir = tempfile::tempdir().unwrap();
    let put = concat!(
        "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\n",
        "scope=project:investments\nstatus=ready\nalpha budget"
    );
    assert!(handle_http(dir.path(), put).contains(r#""seq":1"#));

    let request = concat!(
        "POST /v1/aql?scope=project:investments HTTP/1.1\r\n\r\n",
        "RETRIEVE CONTEXT FOR TASK \"budget\" IN BRAIN investment_projects ",
        "WHERE space = project:investments AND status = \"ready\" LIMIT 10 CANDIDATES;"
    );
    let response = handle_http(dir.path(), request);
    assert!(response.contains(r#""cells":[{"cell_id":1"#));
    assert!(response.contains("alpha budget"));
}
