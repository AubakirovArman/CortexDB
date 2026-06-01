use serde::Deserialize;

use crate::responses::{LlmInferenceAuditResponse, LlmInferenceResponse, RouterError};
use safety::{validate_llm_runtime_safety_config, LlmRuntimeSafetyConfig};

mod safety;

const REQUEST_SCHEMA_VERSION: &str = "cortexdb.llm_inference.smoke_request.v1";
const RESPONSE_SCHEMA_VERSION: &str = "cortexdb.llm_inference.smoke_response.v1";
const TEST_DOUBLE_PROVIDER: &str = "test_double";
const TEST_DOUBLE_MODEL: &str = "deterministic-echo-v1";

#[derive(Deserialize)]
struct LlmInferenceRequest {
    schema_version: String,
    enabled: bool,
    provider: String,
    model: Option<String>,
    #[serde(default)]
    api_key: Option<String>,
    prompt: String,
    context_pack: LlmContextPackRequest,
}

#[derive(Deserialize)]
struct LlmContextPackRequest {
    cells: Vec<LlmContextCellRequest>,
}

#[derive(Deserialize)]
struct LlmContextCellRequest {
    cell_id: u64,
    #[serde(default)]
    source_ref: Option<String>,
    #[serde(default)]
    citation: Option<String>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    payload_text: Option<String>,
}

pub(crate) fn handle_inference_test_double(
    body: &[u8],
    enabled: bool,
) -> Result<String, RouterError> {
    if !enabled {
        return Err(RouterError::Forbidden(
            "LLM inference test-double endpoint is disabled".to_owned(),
        ));
    }
    validate_llm_runtime_safety_config(&test_double_runtime_safety_config(enabled)).map_err(
        |failure| RouterError::Internal(format!("unsafe LLM runtime safety config: {failure:?}")),
    )?;
    let request = serde_json::from_slice::<LlmInferenceRequest>(body)
        .map_err(|error| RouterError::BadRequest(format!("invalid inference JSON: {error}")))?;
    if request.schema_version != REQUEST_SCHEMA_VERSION {
        return Err(RouterError::BadRequest(
            "unsupported inference request schema_version".to_owned(),
        ));
    }
    if !request.enabled {
        return Err(RouterError::BadRequest(
            "request enabled must be true for the deterministic test-double".to_owned(),
        ));
    }
    if request.provider != TEST_DOUBLE_PROVIDER {
        return Err(RouterError::BadRequest(
            "only provider=test_double is supported by this local endpoint".to_owned(),
        ));
    }
    if request.model.as_deref().unwrap_or(TEST_DOUBLE_MODEL) != TEST_DOUBLE_MODEL {
        return Err(RouterError::BadRequest(
            "only model=deterministic-echo-v1 is supported by this local endpoint".to_owned(),
        ));
    }
    if request
        .api_key
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
    {
        return Err(RouterError::BadRequest(
            "api_key must not be sent to the local deterministic test-double".to_owned(),
        ));
    }
    if request.prompt.trim().is_empty() {
        return Err(RouterError::BadRequest(
            "prompt must not be empty".to_owned(),
        ));
    }
    if request.context_pack.cells.is_empty() {
        return Err(RouterError::BadRequest(
            "context_pack.cells must not be empty".to_owned(),
        ));
    }

    let used_context_cell_ids = request
        .context_pack
        .cells
        .iter()
        .map(|cell| cell.cell_id)
        .collect::<Vec<_>>();
    let citations = request
        .context_pack
        .cells
        .iter()
        .filter_map(|cell| cell.citation.clone().or_else(|| cell.source_ref.clone()))
        .collect::<Vec<_>>();
    let first_text = request
        .context_pack
        .cells
        .iter()
        .find_map(|cell| cell.text.as_deref().or(cell.payload_text.as_deref()))
        .unwrap_or("")
        .trim();
    let output = if first_text.is_empty() {
        "No context text was supplied to the deterministic test-double provider.".to_owned()
    } else {
        summarize_from_context(first_text)
    };
    let response = LlmInferenceResponse {
        schema_version: RESPONSE_SCHEMA_VERSION,
        provider: TEST_DOUBLE_PROVIDER.to_owned(),
        model: request
            .model
            .unwrap_or_else(|| TEST_DOUBLE_MODEL.to_owned()),
        output,
        used_context_cell_ids,
        citations,
        audit: LlmInferenceAuditResponse {
            context_pack_only: true,
            prompt_body_logged: false,
            secrets_logged: false,
        },
    };
    Ok(serde_json::to_string(&response)?)
}

fn summarize_from_context(text: &str) -> String {
    let snippet = text.chars().take(180).collect::<String>();
    format!("Test-double answer from explicit ContextPack only: {snippet}")
}

fn test_double_runtime_safety_config(enabled: bool) -> LlmRuntimeSafetyConfig {
    LlmRuntimeSafetyConfig {
        enabled,
        provider: TEST_DOUBLE_PROVIDER.to_owned(),
        model: TEST_DOUBLE_MODEL.to_owned(),
        max_prompt_bytes: 16 * 1024,
        max_context_cells: 32,
        max_output_tokens: 512,
        request_timeout_ms: 5_000,
        queue_capacity: 64,
        max_concurrent_requests: 4,
        request_api_keys_allowed: false,
        prompt_body_logging_enabled: false,
    }
}

#[cfg(test)]
mod tests {
    use super::handle_inference_test_double;
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
        assert!(matches!(error, RouterError::Forbidden(_)));
    }

    #[test]
    fn test_double_uses_context_pack_only() {
        let response = handle_inference_test_double(REQUEST, true).unwrap();
        assert!(response.contains(r#""schema_version":"cortexdb.llm_inference.smoke_response.v1""#));
        assert!(response.contains(r#""provider":"test_double""#));
        assert!(response.contains(r#""used_context_cell_ids":[7]"#));
        assert!(response.contains(r#""context_pack_only":true"#));
        assert!(response.contains(r#""prompt_body_logged":false"#));
        assert!(response.contains(r#""secrets_logged":false"#));
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
        assert!(matches!(error, RouterError::BadRequest(_)));
    }
}
