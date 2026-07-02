use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use serde_json::Value;

use crate::{handle_http_with_options, ServerOptions};

const REQUEST: &str = r#"{
  "schema_version":"cortexdb.llm_inference.smoke_request.v1",
  "enabled":true,
  "provider":"test_double",
  "model":"deterministic-echo-v1",
  "prompt":"Summarize only the supplied context.",
  "context_pack":{
    "schema_version":"cortexdb.context_pack.v1",
    "cells":[
      {
        "cell_id":101,
        "scope":"project:investments",
        "source_ref":"doc://investment-risk#p1",
        "text":"Project Alpha has a documented budget variance risk and a cited mitigation plan."
      }
    ]
  }
}"#;

#[test]
fn inference_endpoint_is_disabled_by_default() {
    let dir = tempfile::tempdir().unwrap();
    let response = handle_http_with_options(
        dir.path(),
        &format!("POST /v1/inference HTTP/1.1\r\n\r\n{REQUEST}"),
        &ServerOptions::default(),
    );
    assert!(response.contains("403 Forbidden"), "{response}");
    assert!(
        response.contains("test-double endpoint is disabled"),
        "{response}"
    );
}

#[test]
fn inference_test_double_uses_explicit_context_pack_only() {
    let dir = tempfile::tempdir().unwrap();
    let options = ServerOptions {
        llm_test_double_enabled: true,
        ..Default::default()
    };
    let response = handle_http_with_options(
        dir.path(),
        &format!("POST /v1/inference HTTP/1.1\r\n\r\n{REQUEST}"),
        &options,
    );
    assert!(response.contains("200 OK"), "{response}");
    let body = body_json(&response);
    assert_eq!(
        body["schema_version"],
        "cortexdb.llm_inference.smoke_response.v1"
    );
    assert_eq!(body["provider"], "test_double");
    assert_eq!(body["model"], "deterministic-echo-v1");
    assert_eq!(body["used_context_cell_ids"], serde_json::json!([101]));
    assert_eq!(
        body["citations"],
        serde_json::json!(["doc://investment-risk#p1"])
    );
    assert_eq!(body["grounding"]["answer_supported"], false);
    assert_eq!(body["grounding"]["unsupported_span_count"], 1);
    assert_eq!(
        body["grounding"]["spans"][0]["supported_by_cell_ids"],
        serde_json::json!([101])
    );
    assert_eq!(body["audit"]["context_pack_only"], true);
    assert_eq!(body["audit"]["prompt_body_logged"], false);
    assert_eq!(body["audit"]["secrets_logged"], false);
}

#[test]
fn inference_test_double_rejects_provider_keys_in_request_body() {
    let dir = tempfile::tempdir().unwrap();
    let options = ServerOptions {
        llm_test_double_enabled: true,
        ..Default::default()
    };
    let request = REQUEST.replace(
        r#""provider":"test_double","#,
        r#""provider":"test_double","api_key":"not-allowed","#,
    );
    let response = handle_http_with_options(
        dir.path(),
        &format!("POST /v1/inference HTTP/1.1\r\n\r\n{request}"),
        &options,
    );
    assert!(response.contains("400 Bad Request"), "{response}");
    assert!(response.contains("api_key must not be sent"), "{response}");
}

#[test]
fn inference_test_double_rejects_non_test_provider() {
    let dir = tempfile::tempdir().unwrap();
    let options = ServerOptions {
        llm_test_double_enabled: true,
        ..Default::default()
    };
    let request = REQUEST.replace(r#""provider":"test_double""#, r#""provider":"openai""#);
    let response = handle_http_with_options(
        dir.path(),
        &format!("POST /v1/inference HTTP/1.1\r\n\r\n{request}"),
        &options,
    );
    assert!(response.contains("400 Bad Request"), "{response}");
    assert!(response.contains("only provider=test_double"), "{response}");
}

#[test]
fn inference_audit_log_records_decisions_without_prompt_or_secret() {
    let dir = tempfile::tempdir().unwrap();
    let root_path = dir.path().join("db");
    let audit_path = dir.path().join("audit").join("http.jsonl");
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let local_addr = listener.local_addr().unwrap();
    drop(listener);

    let audit_path_for_server = audit_path.clone();
    std::thread::spawn(move || {
        let options = ServerOptions {
            audit_log_enabled: true,
            audit_log_path: Some(audit_path_for_server),
            audit_log_mac_key: Some(test_audit_mac_key()),
            llm_test_double_enabled: true,
            ..Default::default()
        };
        let _ = crate::serve_with_options(&root_path, &local_addr.to_string(), options);
    });

    let response = request_http(local_addr, &post_inference_request(REQUEST));
    assert!(response.contains("200 OK"), "{response}");

    let rejected = REQUEST.replace(
        r#""provider":"test_double","#,
        r#""provider":"test_double","api_key":"not-allowed-secret","#,
    );
    let response = request_http(local_addr, &post_inference_request(&rejected));
    assert!(response.contains("400 Bad Request"), "{response}");

    let values = read_decision_audit_events(&audit_path);
    let allowed = values
        .iter()
        .find(|value| value["llm"]["outcome"] == "allowed")
        .expect("allowed LLM audit decision");
    assert_eq!(allowed["audit_event"], "llm_inference_decision");
    assert_eq!(allowed["audit_action"], "inference");
    assert_eq!(allowed["path"], "/v1/inference");
    assert_eq!(allowed["tenant"], "alpha");
    assert_eq!(allowed["llm"]["reason"], "test_double_completed");
    assert_eq!(allowed["llm"]["provider"], "test_double");
    assert_eq!(allowed["llm"]["model"], "deterministic-echo-v1");
    assert_eq!(allowed["llm"]["context_cell_count"], 1);
    assert_eq!(allowed["llm"]["citation_count"], 1);
    assert_eq!(allowed["llm"]["prompt_body_logged"], false);
    assert_eq!(allowed["llm"]["secrets_logged"], false);

    let denied = values
        .iter()
        .find(|value| value["llm"]["reason"] == "request_api_key_present")
        .expect("denied LLM audit decision");
    assert_eq!(denied["status"], 400);
    assert_eq!(denied["error_code"], "bad_request");
    assert_eq!(denied["llm"]["request_api_key_present"], true);

    let audit_raw = std::fs::read_to_string(audit_path).unwrap();
    for leaked in [
        "Summarize only the supplied context",
        "Project Alpha has a documented budget variance risk",
        "doc://investment-risk#p1",
        "not-allowed-secret",
    ] {
        assert!(
            !audit_raw.contains(leaked),
            "LLM audit leaked sensitive request data {leaked:?}: {audit_raw}"
        );
    }
}

fn test_audit_mac_key() -> crate::AuditMacKey {
    crate::AuditMacKey::from_hex(
        "test-audit-key",
        "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
    )
    .unwrap()
}

fn body_json(response: &str) -> Value {
    let (_, body) = response.split_once("\r\n\r\n").unwrap();
    serde_json::from_str(body).unwrap()
}

fn post_inference_request(body: &str) -> String {
    format!(
        "POST /v1/inference?tenant=alpha HTTP/1.1\r\ncontent-length: {}\r\n\r\n{}",
        body.len(),
        body
    )
}

fn request_http(addr: SocketAddr, request: &str) -> String {
    let mut last_err = None;
    for _ in 0..20 {
        match TcpStream::connect(addr) {
            Ok(mut stream) => {
                if let Err(err) = stream.write_all(request.as_bytes()) {
                    last_err = Some(err);
                    std::thread::sleep(Duration::from_millis(50));
                    continue;
                }
                let mut response = [0u8; 8192];
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

fn read_decision_audit_events(path: &std::path::Path) -> Vec<Value> {
    let mut last_audit = String::new();
    for _ in 0..20 {
        if let Ok(audit) = std::fs::read_to_string(path) {
            last_audit = audit.clone();
            let values = audit
                .lines()
                .filter_map(|line| serde_json::from_str::<Value>(line).ok())
                .filter(|value| value["audit_event"] == "llm_inference_decision")
                .collect::<Vec<_>>();
            if values.len() >= 2 {
                return values;
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("LLM decision audit events not found in audit log:\n{last_audit}");
}
