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
    assert!(stats.contains(r#""wal_writer_records":0"#));

    let validation = handle_http(dir.path(), "GET /v1/validate HTTP/1.1\r\n\r\n");
    assert!(validation.contains(r#""ok":true"#));
    assert!(validation.contains(r#""vector_indexes_checked":0"#));
    assert!(validation.contains(r#""wal_records_checked":1"#));
}

#[test]
fn v1_cell_miss_returns_typed_null_cell() {
    let dir = tempfile::tempdir().unwrap();
    let response = handle_http(dir.path(), "GET /v1/cell?cell_id=99 HTTP/1.1\r\n\r\n");
    assert!(response.contains(r#""cell":null"#));
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

#[test]
fn v1_search_returns_scope_filtered_results() {
    let dir = tempfile::tempdir().unwrap();
    let put_a = concat!(
        "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\n",
        "scope=project:investments\nstatus=ready\nalpha budget"
    );
    let put_b = concat!(
        "POST /v1/cell?cell_id=2 HTTP/1.1\r\n\r\n",
        "scope=tenant:private\nstatus=ready\nhidden budget"
    );
    assert!(handle_http(dir.path(), put_a).contains(r#""seq":1"#));
    assert!(handle_http(dir.path(), put_b).contains(r#""seq":2"#));

    let request = "POST /v1/search?scope=project:investments&q=budget HTTP/1.1\r\n\r\n";
    let response = handle_http(dir.path(), request);
    assert!(response.contains(r#""cell_id":1"#));
    assert!(!response.contains(r#""cell_id":2"#));
}

#[test]
fn v1_vector_search_accepts_query_vector() {
    let dir = tempfile::tempdir().unwrap();
    let put_a = concat!(
        "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\n",
        "scope=project:investments\nstatus=ready\nvector=5,0\nalpha"
    );
    let put_b = concat!(
        "POST /v1/cell?cell_id=2 HTTP/1.1\r\n\r\n",
        "scope=tenant:private\nstatus=ready\nvector=9,0\nhidden"
    );
    assert!(handle_http(dir.path(), put_a).contains(r#""seq":1"#));
    assert!(handle_http(dir.path(), put_b).contains(r#""seq":2"#));

    let request =
        "POST /v1/search?scope=project:investments&mode=vector&vector=2,0 HTTP/1.1\r\n\r\n";
    let response = handle_http(dir.path(), request);
    assert!(response.contains(r#""cell_id":1"#));
    assert!(response.contains(r#""vector_score":"#));
    assert!(!response.contains(r#""cell_id":2"#));
}

#[test]
fn v1_remember_and_verify_work() {
    let dir = tempfile::tempdir().unwrap();
    let remember = concat!(
        "POST /v1/remember?scope=project:investments HTTP/1.1\r\n\r\n",
        "REMEMBER \"ABC budget approved\" IN SCOPE project:investments AS TYPE decision ",
        "TTL 60 SECONDS;"
    );
    let remember_response = handle_http(dir.path(), remember);
    assert!(remember_response.contains(r#""seq":1"#));
    assert!(remember_response.contains(r#""ttl_seconds":60"#));

    let verify = concat!(
        "POST /v1/verify?scope=project:investments HTTP/1.1\r\n\r\n",
        "VERIFY FACT \"ABC budget approved\" IN BRAIN investment_projects;"
    );
    let verify_response = handle_http(dir.path(), verify);
    assert!(verify_response.contains(r#""status":"supported""#));
    assert!(verify_response.contains(r#""matched_terms":"#));
}

#[test]
fn test_server_concurrency_and_size_limit() {
    let dir = tempfile::tempdir().unwrap();
    let addr = "127.0.0.1:0";
    let listener = std::net::TcpListener::bind(addr).unwrap();
    let local_addr = listener.local_addr().unwrap();
    drop(listener);

    let root_path = dir.path().to_owned();
    std::thread::spawn(move || {
        let _ = super::serve(&root_path, &local_addr.to_string());
    });

    std::thread::sleep(std::time::Duration::from_millis(100));

    {
        use std::io::{Read, Write};
        use std::net::TcpStream;
        let mut stream = TcpStream::connect(local_addr).unwrap();
        let huge_size = 2100 * 1024;
        let mut huge_request = Vec::with_capacity(huge_size + 100);
        huge_request.extend_from_slice(b"POST /put?cell_id=1 HTTP/1.1\r\nContent-Length: ");
        huge_request.extend_from_slice(huge_size.to_string().as_bytes());
        huge_request.extend_from_slice(b"\r\n\r\n");
        huge_request.resize(huge_request.len() + huge_size, b'A');

        let _ = stream.write_all(&huge_request);

        let mut response = [0u8; 1024];
        let read = stream.read(&mut response).unwrap();
        let resp_str = String::from_utf8_lossy(&response[..read]);
        assert!(resp_str.contains("413 Payload Too Large"));
    }

    {
        use std::io::{Read, Write};
        use std::net::TcpStream;

        let mut threads = vec![];
        for _ in 0..5 {
            threads.push(std::thread::spawn(move || {
                let mut stream = TcpStream::connect(local_addr).unwrap();
                stream
                    .write_all(b"GET /v1/health HTTP/1.1\r\n\r\n")
                    .unwrap();
                let mut resp = [0u8; 1024];
                let read = stream.read(&mut resp).unwrap();
                let resp_str = String::from_utf8_lossy(&resp[..read]);
                assert!(resp_str.contains(r#""status":"ok""#));
            }));
        }

        for thread in threads {
            thread.join().unwrap();
        }
    }
}

#[test]
fn empty_ingestion_endpoints_safety() {
    let dir = tempfile::tempdir().unwrap();

    let text_request = "POST /v1/ingest/text?scope=project:investments HTTP/1.1\r\n\r\n";
    let text_response = handle_http(dir.path(), text_request);
    assert!(text_response.contains(r#""chunks_ingested":0"#));
    assert!(text_response.contains(r#""first_cell_id":null"#));

    let json_request = "POST /v1/ingest/json?scope=project:investments HTTP/1.1\r\n\r\n{}";
    let json_response = handle_http(dir.path(), json_request);
    assert!(json_response.contains(r#""facts_ingested":0"#));
    assert!(json_response.contains(r#""first_cell_id":null"#));

    let csv_request = "POST /v1/ingest/csv?scope=project:investments HTTP/1.1\r\n\r\n";
    let csv_response = handle_http(dir.path(), csv_request);
    assert!(csv_response.contains(r#""rows_ingested":0"#));
    assert!(csv_response.contains(r#""first_cell_id":null"#));
}
