use super::{audit::LlmInferenceAuditOutcome, handle_inference_test_double};
use crate::responses::RouterError;

const REQUEST: &[u8] = br#"{
  "schema_version":"cortexdb.llm_inference.smoke_request.v1",
  "enabled":true,
  "provider":"test_double",
  "model":"deterministic-echo-v1",
  "prompt":"Summarize only the supplied context.",
  "context_pack":{
    "cells":[
      {"cell_id":7,"citation":"doc://alpha#p1","text":"Alpha budget has a cited risk."}
    ]
  }
}"#;

#[test]
fn test_double_requires_explicit_enablement() {
    let error = handle_inference_test_double(REQUEST, false).unwrap_err();
    assert!(matches!(error.error, RouterError::Forbidden(_)));
    assert_eq!(error.audit.outcome, LlmInferenceAuditOutcome::Denied);
    assert_eq!(error.audit.reason, "test_double_disabled");
}

#[test]
fn test_double_uses_context_pack_only() {
    let result = handle_inference_test_double(REQUEST, true).unwrap();
    let response = result.body;
    assert!(response.contains(r#""schema_version":"cortexdb.llm_inference.smoke_response.v1""#));
    assert!(response.contains(r#""provider":"test_double""#));
    assert!(response.contains(r#""used_context_cell_ids":[7]"#));
    assert!(response.contains(r#""context_pack_only":true"#));
    assert!(response.contains(r#""prompt_body_logged":false"#));
    assert!(response.contains(r#""secrets_logged":false"#));
    assert_eq!(result.audit.outcome, LlmInferenceAuditOutcome::Allowed);
    assert_eq!(result.audit.reason, "test_double_completed");
    assert_eq!(result.audit.context_cell_count, 1);
    assert_eq!(result.audit.citation_count, 1);
}

#[test]
fn test_double_rejects_request_api_key() {
    let request = br#"{
      "schema_version":"cortexdb.llm_inference.smoke_request.v1",
      "enabled":true,
      "provider":"test_double",
      "model":"deterministic-echo-v1",
      "api_key":"not-allowed",
      "prompt":"Summarize only the supplied context.",
      "context_pack":{"cells":[{"cell_id":7,"text":"alpha"}]}
    }"#;
    let error = handle_inference_test_double(request, true).unwrap_err();
    assert!(matches!(error.error, RouterError::BadRequest(_)));
    assert_eq!(error.audit.reason, "request_api_key_present");
    assert!(error.audit.request_api_key_present);
}
