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
        second_a.contains("quota_exceeded"),
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
        second.contains("quota_exceeded"),
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

    let value = wait_for_metric_value(
        local_addr,
        "GET /v1/metrics HTTP/1.1\r\nAuthorization: Bearer admin-token\r\n\r\n",
        "principal_quota_queue_acquired",
        1,
    );
    assert_eq!(value["principal_quota_queue_acquired"], 1);
    assert_eq!(value["principal_quota_queue_rejected"], 0);
}

#[test]
fn tenant_max_cells_quota_is_isolated_per_tenant() {
    let dir = tempfile::tempdir().unwrap();
    let local_addr = spawn_server_with_options(
        dir.path().join("db"),
        ServerOptions {
            tenant_max_cells: Some(1),
            ..Default::default()
        },
    );

    let first_alpha_body = "scope=default\nstatus=ready\none";
    let first_alpha = request(
        local_addr,
        &format!(
            "POST /v1/cell?tenant=alpha&cell_id=1 HTTP/1.1\r\ncontent-length: {}\r\n\r\n{}",
            first_alpha_body.len(),
            first_alpha_body
        ),
    );
    assert!(
        first_alpha.contains("200 OK"),
        "first alpha write should pass: {first_alpha}"
    );

    let second_alpha_body = "scope=default\nstatus=ready\ntwo";
    let second_alpha = request(
        local_addr,
        &format!(
            "POST /v1/cell?tenant=alpha&cell_id=2 HTTP/1.1\r\ncontent-length: {}\r\n\r\n{}",
            second_alpha_body.len(),
            second_alpha_body
        ),
    );
    assert!(
        second_alpha.contains("429 Too Many Requests"),
        "second alpha write should hit tenant cell quota: {second_alpha}"
    );
    assert!(second_alpha.contains("quota_exceeded"));

    let first_beta_body = "scope=default\nstatus=ready\nbeta";
    let first_beta = request(
        local_addr,
        &format!(
            "POST /v1/cell?tenant=beta&cell_id=1 HTTP/1.1\r\ncontent-length: {}\r\n\r\n{}",
            first_beta_body.len(),
            first_beta_body
        ),
    );
    assert!(
        first_beta.contains("200 OK"),
        "beta tenant quota should be independent: {first_beta}"
    );
}

#[test]
fn tenant_max_memory_quota_rejects_projected_payload_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let local_addr = spawn_server_with_options(
        dir.path().join("db"),
        ServerOptions {
            tenant_max_memory_bytes: Some(8),
            ..Default::default()
        },
    );

    let body = "scope=default\nstatus=ready\npayload-too-large";
    let rejected = request(
        local_addr,
        &format!(
            "POST /v1/cell?tenant=alpha&cell_id=1 HTTP/1.1\r\ncontent-length: {}\r\n\r\n{}",
            body.len(),
            body
        ),
    );
    assert!(
        rejected.contains("429 Too Many Requests"),
        "payload should hit tenant memory quota: {rejected}"
    );
    assert!(rejected.contains("quota_exceeded"));

    let missing = request(
        local_addr,
        "GET /v1/cell?tenant=alpha&cell_id=1 HTTP/1.1\r\n\r\n",
    );
    assert!(
        missing.contains(r#""cell":null"#),
        "rejected quota write must not create the cell: {missing}"
    );
}

#[test]
fn tenant_quota_50_tenant_load_smoke() {
    let dir = tempfile::tempdir().unwrap();
    let local_addr = spawn_server_with_options(
        dir.path().join("db"),
        ServerOptions {
            tenant_max_cells: Some(2),
            tenant_max_memory_bytes: Some(8 * 1024),
            tenant_queue_quota: Some(4),
            ..Default::default()
        },
    );

    for index in 0..50 {
        let tenant = format!("tenant_{index}");
        let payload = format!("scope=default\nstatus=ready\npayload-{index}");
        let put = request(
            local_addr,
            &format!(
                "POST /v1/cell?tenant={tenant}&cell_id=1 HTTP/1.1\r\ncontent-length: {}\r\n\r\n{}",
                payload.len(),
                payload
            ),
        );
        assert!(put.contains("200 OK"), "put failed for {tenant}: {put}");

        let get = request(
            local_addr,
            &format!("GET /v1/cell?tenant={tenant}&cell_id=1 HTTP/1.1\r\n\r\n"),
        );
        assert!(
            get.contains(&format!("payload-{index}")),
            "get failed for {tenant}: {get}"
        );
    }

    let third_body = "scope=default\nstatus=ready\nthird";
    let second_body = "scope=default\nstatus=ready\nsecond";
    let second = request(
        local_addr,
        &format!(
            "POST /v1/cell?tenant=tenant_0&cell_id=2 HTTP/1.1\r\ncontent-length: {}\r\n\r\n{}",
            second_body.len(),
            second_body
        ),
    );
    assert!(
        second.contains("200 OK"),
        "second cell should fit: {second}"
    );
    let third = request(
        local_addr,
        &format!(
            "POST /v1/cell?tenant=tenant_0&cell_id=3 HTTP/1.1\r\ncontent-length: {}\r\n\r\n{}",
            third_body.len(),
            third_body
        ),
    );
    assert!(
        third.contains("429 Too Many Requests") && third.contains("quota_exceeded"),
        "third cell should exceed per-tenant quota: {third}"
    );
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
    spawn_server_with_options(
        root_path,
        ServerOptions {
            auth_policy_store_file: Some(policy_store),
            ..Default::default()
        },
    )
}

fn spawn_server_with_options(
    root_path: std::path::PathBuf,
    options: ServerOptions,
) -> std::net::SocketAddr {
    let addr = "127.0.0.1:0";
    let listener = std::net::TcpListener::bind(addr).unwrap();
    let local_addr = listener.local_addr().unwrap();
    drop(listener);

    std::thread::spawn(move || {
        let _ = crate::serve_with_options(&root_path, &local_addr.to_string(), options);
    });

    std::thread::sleep(std::time::Duration::from_millis(100));
    local_addr
}

fn body_json(response: &str) -> serde_json::Value {
    let (_, body) = response.split_once("\r\n\r\n").unwrap();
    serde_json::from_str(body).unwrap()
}

fn wait_for_metric_value(
    addr: std::net::SocketAddr,
    request_line: &str,
    key: &str,
    expected: u64,
) -> serde_json::Value {
    let mut latest = serde_json::Value::Null;
    for _ in 0..20 {
        let metrics = request(addr, request_line);
        latest = body_json(&metrics);
        if latest[key].as_u64() == Some(expected) {
            return latest;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    latest
}
