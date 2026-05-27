use crate::{handle_http_with_options, ServerOptions};

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
fn test_tenant_validation_and_path_traversal() {
    // 1. Test validate_tenant_id helper directly
    assert!(crate::validate_tenant_id("default"));
    assert!(crate::validate_tenant_id("tenant1"));
    assert!(crate::validate_tenant_id("tenant-1"));
    assert!(crate::validate_tenant_id("tenant_1"));
    assert!(crate::validate_tenant_id("tenant:1"));
    assert!(!crate::validate_tenant_id("../../escape"));
    assert!(!crate::validate_tenant_id("..%2f..%2fescape"));
    assert!(!crate::validate_tenant_id("a/b"));
    assert!(!crate::validate_tenant_id(""));
    assert!(!crate::validate_tenant_id(&"a".repeat(65)));

    // 2. Test path traversal block over the HTTP service
    let dir = tempfile::tempdir().unwrap();
    let addr = "127.0.0.1:0";
    let listener = std::net::TcpListener::bind(addr).unwrap();
    let local_addr = listener.local_addr().unwrap();
    drop(listener);

    let root_path = dir.path().to_owned();
    std::thread::spawn(move || {
        let _ = crate::serve(&root_path, &local_addr.to_string());
    });

    std::thread::sleep(std::time::Duration::from_millis(100));

    {
        use std::io::{Read, Write};
        use std::net::TcpStream;
        let mut stream = TcpStream::connect(local_addr).unwrap();
        stream
            .write_all(b"GET /v1/health?tenant=../../escape HTTP/1.1\r\n\r\n")
            .unwrap();

        let mut response = [0u8; 1024];
        let read = stream.read(&mut response).unwrap();
        let resp_str = String::from_utf8_lossy(&response[..read]);
        assert!(resp_str.contains("400 Bad Request"));
        assert!(resp_str.contains("invalid_tenant"));
    }
}
