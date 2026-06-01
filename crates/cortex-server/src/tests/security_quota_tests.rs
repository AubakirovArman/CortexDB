use crate::ServerOptions;

#[test]
fn policy_store_principal_quota_is_isolated_per_principal() {
    let dir = tempfile::tempdir().unwrap();
    let policy_store = dir.path().join("auth-policy.json");
    std::fs::write(
        &policy_store,
        r#"{
          "schema_version": "cortexdb.auth_policy.v1",
          "principals": [
            {"principal_id":"principal-a","token":"token-a","role":"data","request_quota_per_minute":1},
            {"principal_id":"principal-b","token":"token-b","role":"data","request_quota_per_minute":1}
          ]
        }"#,
    )
    .unwrap();
    let addr = "127.0.0.1:0";
    let listener = std::net::TcpListener::bind(addr).unwrap();
    let local_addr = listener.local_addr().unwrap();
    drop(listener);

    let root_path = dir.path().join("db");
    std::thread::spawn(move || {
        let options = ServerOptions {
            auth_policy_store_file: Some(policy_store),
            ..Default::default()
        };
        let _ = crate::serve_with_options(&root_path, &local_addr.to_string(), options);
    });

    std::thread::sleep(std::time::Duration::from_millis(100));

    let first_a = request(
        local_addr,
        "GET /v1/health HTTP/1.1\r\nAuthorization: Bearer token-a\r\n\r\n",
    );
    assert!(
        first_a.contains("200 OK"),
        "first principal-a request failed: {first_a}"
    );

    let second_a = request(
        local_addr,
        "GET /v1/health HTTP/1.1\r\nAuthorization: Bearer token-a\r\n\r\n",
    );
    assert!(
        second_a.contains("429 Too Many Requests"),
        "second principal-a request should be quota limited: {second_a}"
    );
    assert!(
        second_a.contains("rate_limited"),
        "quota response should use typed error code: {second_a}"
    );

    let first_b = request(
        local_addr,
        "GET /v1/health HTTP/1.1\r\nAuthorization: Bearer token-b\r\n\r\n",
    );
    assert!(
        first_b.contains("200 OK"),
        "principal-b quota should be independent: {first_b}"
    );
}

#[test]
fn policy_store_principal_body_quota_limits_uploaded_bytes_and_reports_metrics() {
    let dir = tempfile::tempdir().unwrap();
    let policy_store = dir.path().join("auth-policy.json");
    std::fs::write(
        &policy_store,
        r#"{
          "schema_version": "cortexdb.auth_policy.v1",
          "principals": [
            {"principal_id":"principal-a","token":"token-a","role":"data","body_quota_bytes_per_minute":5},
            {"principal_id":"admin-a","token":"admin-token","role":"admin"}
          ]
        }"#,
    )
    .unwrap();
    let local_addr = spawn_server(dir.path().join("db"), policy_store);

    let first_body = "12345";
    let first = request(
        local_addr,
        &format!(
            "POST /v1/cell?cell_id=1 HTTP/1.1\r\nAuthorization: Bearer token-a\r\ncontent-length: {}\r\n\r\n{}",
            first_body.len(),
            first_body
        ),
    );
    assert!(
        first.contains("200 OK"),
        "body within principal quota should pass: {first}"
    );

    let second_body = "1";
    let second = request(
        local_addr,
        &format!(
            "POST /v1/cell?cell_id=2 HTTP/1.1\r\nAuthorization: Bearer token-a\r\ncontent-length: {}\r\n\r\n{}",
            second_body.len(),
            second_body
        ),
    );
    assert!(
        second.contains("429 Too Many Requests"),
        "body over principal quota should be rejected: {second}"
    );
    assert!(
        second.contains("rate_limited"),
        "body quota response should use typed error code: {second}"
    );

    let metrics = request(
        local_addr,
        "GET /v1/metrics HTTP/1.1\r\nAuthorization: Bearer admin-token\r\n\r\n",
    );
    assert!(metrics.contains("200 OK"), "metrics should load: {metrics}");
    let value = body_json(&metrics);
    assert_eq!(value["principal_quota_body_bytes_allowed"], 5);
    assert_eq!(value["principal_quota_body_bytes_rejected"], 1);
}

#[test]
fn policy_store_principal_queue_quota_reports_actor_queue_metrics() {
    let dir = tempfile::tempdir().unwrap();
    let policy_store = dir.path().join("auth-policy.json");
    std::fs::write(
        &policy_store,
        r#"{
          "schema_version": "cortexdb.auth_policy.v1",
          "principals": [
            {"principal_id":"principal-a","token":"token-a","role":"data","queue_quota":1},
            {"principal_id":"admin-a","token":"admin-token","role":"admin"}
          ]
        }"#,
    )
    .unwrap();
    let local_addr = spawn_server(dir.path().join("db"), policy_store);

    let search = request(
        local_addr,
        "POST /v1/search?scope=default&q=none HTTP/1.1\r\nAuthorization: Bearer token-a\r\ncontent-length: 0\r\n\r\n",
    );
    assert!(
        search.contains("200 OK"),
        "queue quota should allow a single actor command: {search}"
    );

    let metrics = request(
        local_addr,
        "GET /v1/metrics HTTP/1.1\r\nAuthorization: Bearer admin-token\r\n\r\n",
    );
    let value = body_json(&metrics);
    assert_eq!(value["principal_quota_queue_acquired"], 1);
    assert_eq!(value["principal_quota_queue_rejected"], 0);
}

fn request(addr: std::net::SocketAddr, request: &str) -> String {
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
                let mut response = [0u8; 16384];
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
    panic!("failed to perform request after retries: {:?}", last_err);
}

fn spawn_server(
    root_path: std::path::PathBuf,
    policy_store: std::path::PathBuf,
) -> std::net::SocketAddr {
    let addr = "127.0.0.1:0";
    let listener = std::net::TcpListener::bind(addr).unwrap();
    let local_addr = listener.local_addr().unwrap();
    drop(listener);

    std::thread::spawn(move || {
        let options = ServerOptions {
            auth_policy_store_file: Some(policy_store),
            ..Default::default()
        };
        let _ = crate::serve_with_options(&root_path, &local_addr.to_string(), options);
    });

    std::thread::sleep(std::time::Duration::from_millis(100));
    local_addr
}

fn body_json(response: &str) -> serde_json::Value {
    let (_, body) = response.split_once("\r\n\r\n").unwrap();
    serde_json::from_str(body).unwrap()
}
