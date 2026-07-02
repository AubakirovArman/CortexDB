use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use crate::{ReceiptSigningKey, ServerOptions};

use super::helpers::{read_audit_event, request};

#[test]
fn audit_log_file_records_route_metadata_without_query() {
    let dir = tempfile::tempdir().unwrap();
    let audit_path = dir.path().join("audit").join("http.jsonl");
    let addr = "127.0.0.1:0";
    let listener = std::net::TcpListener::bind(addr).unwrap();
    let local_addr = listener.local_addr().unwrap();
    drop(listener);

    let root_path = dir.path().join("db");
    let audit_path_for_server = audit_path.clone();
    std::thread::spawn(move || {
        let options = ServerOptions {
            audit_log_enabled: true,
            audit_log_path: Some(audit_path_for_server),
            audit_log_mac_key: Some(test_audit_mac_key()),
            ..Default::default()
        };
        let _ = crate::serve_with_options(&root_path, &local_addr.to_string(), options);
    });

    std::thread::sleep(std::time::Duration::from_millis(100));
    let response = request(local_addr, "GET /v1/health?tenant=alpha HTTP/1.1\r\n\r\n");
    assert!(response.contains("200 OK"), "health failed: {response}");

    let (line, value) = read_audit_event(&audit_path, "health", "/v1/health");
    assert_eq!(value["audit_event"], "http_response");
    assert_eq!(value["audit_action"], "health");
    assert_eq!(value["method"], "GET");
    assert_eq!(value["path"], "/v1/health");
    assert_eq!(value["tenant"], "alpha");
    assert!(value["request_id"]
        .as_str()
        .is_some_and(|request_id| request_id.starts_with("cortexdb-")));
    assert_eq!(value["status"], 200);
    assert!(!line.contains("tenant=alpha"));
}

#[test]
fn audit_log_file_redacts_ingestion_query_and_body() {
    let dir = tempfile::tempdir().unwrap();
    let audit_path = dir.path().join("audit").join("http.jsonl");
    let addr = "127.0.0.1:0";
    let listener = std::net::TcpListener::bind(addr).unwrap();
    let local_addr = listener.local_addr().unwrap();
    drop(listener);

    let root_path = dir.path().join("db");
    let audit_path_for_server = audit_path.clone();
    std::thread::spawn(move || {
        let options = ServerOptions {
            audit_log_enabled: true,
            audit_log_path: Some(audit_path_for_server),
            audit_log_mac_key: Some(test_audit_mac_key()),
            ..Default::default()
        };
        let _ = crate::serve_with_options(&root_path, &local_addr.to_string(), options);
    });

    std::thread::sleep(std::time::Duration::from_millis(100));
    let ingest_request = "POST /v1/ingest/text?tenant=alpha&scope=project%3Ainvestments&source=secret-source HTTP/1.1\r\ncontent-length: 20\r\n\r\nsecret payload token";
    let response = request(local_addr, ingest_request);
    assert!(response.contains("200 OK"), "ingest failed: {response}");

    let (line, value) = read_audit_event(&audit_path, "ingest", "/v1/ingest/text");
    assert_eq!(value["audit_action"], "ingest");
    assert_eq!(value["path"], "/v1/ingest/text");
    assert_eq!(value["tenant"], "alpha");
    assert!(!line.contains("secret-source"));
    assert!(!line.contains("secret payload token"));
    assert!(!line.contains("project%3Ainvestments"));
}

#[test]
fn audit_log_file_records_policy_store_principal_without_token() {
    let dir = tempfile::tempdir().unwrap();
    let audit_path = dir.path().join("audit").join("http.jsonl");
    let policy_store = dir.path().join("auth-policy.json");
    std::fs::write(
        &policy_store,
        r#"{
          "schema_version": "cortexdb.auth_policy.v1",
          "principals": [
            {"principal_id":"finance-agent","token":"finance-token","role":"data","agent_id":11}
          ]
        }"#,
    )
    .unwrap();
    let addr = "127.0.0.1:0";
    let listener = std::net::TcpListener::bind(addr).unwrap();
    let local_addr = listener.local_addr().unwrap();
    drop(listener);

    let root_path = dir.path().join("db");
    let audit_path_for_server = audit_path.clone();
    std::thread::spawn(move || {
        let options = ServerOptions {
            auth_policy_store_file: Some(policy_store),
            audit_log_enabled: true,
            audit_log_path: Some(audit_path_for_server),
            audit_log_mac_key: Some(test_audit_mac_key()),
            ..Default::default()
        };
        let _ = crate::serve_with_options(&root_path, &local_addr.to_string(), options);
    });

    std::thread::sleep(std::time::Duration::from_millis(100));
    let response = request(
        local_addr,
        "GET /v1/health?tenant=alpha HTTP/1.1\r\nAuthorization: Bearer finance-token\r\n\r\n",
    );
    assert!(
        response.contains("200 OK"),
        "policy-store principal should access health: {response}"
    );

    let (line, value) = read_audit_event(&audit_path, "health", "/v1/health");
    assert_eq!(value["audit_event"], "http_response");
    assert_eq!(value["audit_action"], "health");
    assert_eq!(value["principal_id"], "finance-agent");
    assert_eq!(value["auth_role"], "data");
    assert_eq!(value["auth_agent_id"], 11);
    assert!(!line.contains("finance-token"));
}

#[test]
fn audit_log_binds_emitted_context_receipt_hash() {
    let dir = tempfile::tempdir().unwrap();
    let audit_path = dir.path().join("audit").join("http.jsonl");
    let addr = "127.0.0.1:0";
    let listener = std::net::TcpListener::bind(addr).unwrap();
    let local_addr = listener.local_addr().unwrap();
    drop(listener);

    let root_path = dir.path().join("db");
    let audit_path_for_server = audit_path.clone();
    std::thread::spawn(move || {
        let options = ServerOptions {
            audit_log_enabled: true,
            audit_log_path: Some(audit_path_for_server),
            audit_log_mac_key: Some(test_audit_mac_key()),
            receipt_signing_key: Some(
                ReceiptSigningKey::from_seed_hex(
                    "receipt-key-audit-test",
                    "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
                )
                .unwrap(),
            ),
            ..Default::default()
        };
        let _ = crate::serve_with_options(&root_path, &local_addr.to_string(), options);
    });

    std::thread::sleep(std::time::Duration::from_millis(100));
    let ingest_body = "scope=project:investments\nstatus=ready\nsource=doc-a\n\nalpha";
    let ingest_response = request(
        local_addr,
        &format!(
            "POST /v1/cell?scope=project%3Ainvestments&cell_id=1 HTTP/1.1\r\ncontent-length: {}\r\n\r\n{}",
            ingest_body.len(),
            ingest_body
        ),
    );
    assert!(
        ingest_response.contains("200 OK"),
        "ingest failed: {ingest_response}"
    );

    let body = concat!(
        "RETRIEVE CONTEXT FOR TASK \"alpha\" IN BRAIN default ",
        "WHERE space = project:investments AND status = \"ready\" LIMIT 10 CANDIDATES;"
    );
    let response = request_full(
        local_addr,
        &format!(
            "POST /v1/context?scope=project:investments HTTP/1.1\r\nconnection: close\r\ncontent-length: {}\r\n\r\n{}",
            body.len(),
            body
        ),
    );
    assert!(response.contains("200 OK"), "context failed: {response}");
    let (_, body) = response.split_once("\r\n\r\n").unwrap();
    let value = serde_json::from_str::<serde_json::Value>(body).unwrap();
    let expected_hash =
        crate::receipt::accountability_receipt_audit_hash(&value["accountability_receipt"]);

    let (_, audit) = read_audit_event(&audit_path, "context", "/v1/context");
    assert_eq!(audit["accountability_receipt_hash"], expected_hash);
}

fn test_audit_mac_key() -> crate::AuditMacKey {
    crate::AuditMacKey::from_hex(
        "test-audit-key",
        "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
    )
    .unwrap()
}

fn request_full(addr: SocketAddr, request: &str) -> String {
    let mut stream = TcpStream::connect(addr).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    stream.write_all(request.as_bytes()).unwrap();
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).unwrap();
    String::from_utf8_lossy(&buf).to_string()
}
