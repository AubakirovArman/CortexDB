use crate::ServerOptions;

use super::helpers::{request, request_bytes};

#[test]
fn x_request_id_is_propagated_or_generated() {
    let dir = tempfile::tempdir().unwrap();
    let addr = "127.0.0.1:0";
    let listener = std::net::TcpListener::bind(addr).unwrap();
    let local_addr = listener.local_addr().unwrap();
    drop(listener);

    let root_path = dir.path().to_owned();
    std::thread::spawn(move || {
        let _ = crate::serve(&root_path, &local_addr.to_string());
    });

    let propagated = request(
        local_addr,
        "GET /v1/health HTTP/1.1\r\nx-request-id: req-test-123\r\n\r\n",
    )
    .to_ascii_lowercase();
    assert!(propagated.contains("x-request-id: req-test-123"));

    let generated = request(local_addr, "GET /v1/health HTTP/1.1\r\n\r\n").to_ascii_lowercase();
    assert!(generated.contains("x-request-id: cortexdb-"));

    let metrics = request(local_addr, "GET /v1/metrics HTTP/1.1\r\n\r\n");
    assert!(
        metrics.contains(r#""request_id_client_provided":1"#),
        "client request-id counter missing from metrics: {metrics}"
    );
    assert!(
        metrics.contains(r#""request_id_generated":2"#),
        "generated request-id counter missing from metrics: {metrics}"
    );
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
        let _ = crate::serve(&root_path, &local_addr.to_string());
    });

    {
        let huge_size = 2100 * 1024;
        let mut huge_request = Vec::with_capacity(huge_size + 100);
        huge_request.extend_from_slice(b"POST /put?cell_id=1 HTTP/1.1\r\nContent-Length: ");
        huge_request.extend_from_slice(huge_size.to_string().as_bytes());
        huge_request.extend_from_slice(b"\r\n\r\n");
        huge_request.resize(huge_request.len() + huge_size, b'A');

        let resp_str = request_bytes(local_addr, &huge_request);
        assert!(resp_str.contains("413 Payload Too Large"));
    }

    {
        let mut threads = vec![];
        for _ in 0..5 {
            threads.push(std::thread::spawn(move || {
                let resp_str = request(local_addr, "GET /v1/health HTTP/1.1\r\n\r\n");
                assert!(resp_str.contains(r#""status":"ok""#));
            }));
        }

        for thread in threads {
            thread.join().unwrap();
        }
    }
}

#[test]
fn cors_preflight_is_only_enabled_for_configured_origin() {
    let dir = tempfile::tempdir().unwrap();
    let addr = "127.0.0.1:0";
    let listener = std::net::TcpListener::bind(addr).unwrap();
    let local_addr = listener.local_addr().unwrap();
    drop(listener);

    let root_path = dir.path().to_owned();
    std::thread::spawn(move || {
        let options = ServerOptions {
            cors_allowed_origin: Some("https://app.example".to_owned()),
            ..Default::default()
        };
        let _ = crate::serve_with_options(&root_path, &local_addr.to_string(), options);
    });

    let response = request(
        local_addr,
        "OPTIONS /v1/health HTTP/1.1\r\n\
         Origin: https://app.example\r\n\
         Access-Control-Request-Method: GET\r\n\
         Access-Control-Request-Headers: authorization,content-type\r\n\r\n",
    );
    let resp_str = response.to_ascii_lowercase();
    assert!(resp_str.contains("access-control-allow-origin: https://app.example"));
    assert!(resp_str.contains("access-control-allow-methods"));
    assert!(resp_str.contains("access-control-allow-headers"));
}

#[test]
fn rate_limit_returns_typed_429_when_enabled() {
    let dir = tempfile::tempdir().unwrap();
    let addr = "127.0.0.1:0";
    let listener = std::net::TcpListener::bind(addr).unwrap();
    let local_addr = listener.local_addr().unwrap();
    drop(listener);

    let root_path = dir.path().to_owned();
    std::thread::spawn(move || {
        let options = ServerOptions {
            request_rate_limit_per_minute: Some(1),
            ..Default::default()
        };
        let _ = crate::serve_with_options(&root_path, &local_addr.to_string(), options);
    });

    std::thread::sleep(std::time::Duration::from_millis(100));

    let first = request(local_addr, "GET /v1/health HTTP/1.1\r\n\r\n");
    assert!(first.contains("200 OK"), "first request failed: {first}");

    let second = request(local_addr, "GET /v1/health HTTP/1.1\r\n\r\n");
    assert!(
        second.contains("429 Too Many Requests"),
        "second request should be rate limited: {second}"
    );
    assert!(
        second.contains("rate_limited"),
        "rate limit response should use typed error code: {second}"
    );
}
