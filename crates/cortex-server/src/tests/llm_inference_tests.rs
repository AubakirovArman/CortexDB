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

fn body_json(response: &str) -> Value {
    let (_, body) = response.split_once("\r\n\r\n").unwrap();
    serde_json::from_str(body).unwrap()
}
