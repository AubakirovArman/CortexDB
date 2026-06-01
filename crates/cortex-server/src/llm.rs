use serde::Deserialize;

use crate::responses::{LlmInferenceAuditResponse, LlmInferenceResponse, RouterError};
pub(crate) use audit::{LlmInferenceDecisionAudit, LlmInferenceRejection, LlmInferenceResult};
use safety::{validate_llm_runtime_safety_config, LlmRuntimeSafetyConfig};

mod audit;
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
) -> Result<LlmInferenceResult, LlmInferenceRejection> {
    if !enabled {
        return Err(rejection(
            RouterError::Forbidden("LLM inference test-double endpoint is disabled".to_owned()),
            LlmInferenceDecisionAudit::denied("test_double_disabled"),
        ));
    }
    validate_llm_runtime_safety_config(&test_double_runtime_safety_config(enabled)).map_err(
        |failure| {
            rejection(
                RouterError::Internal(format!("unsafe LLM runtime safety config: {failure:?}")),
                LlmInferenceDecisionAudit::denied("unsafe_runtime_config"),
            )
        },
    )?;
    let request = serde_json::from_slice::<LlmInferenceRequest>(body).map_err(|error| {
        rejection(
            RouterError::BadRequest(format!("invalid inference JSON: {error}")),
            LlmInferenceDecisionAudit::denied("invalid_json"),
        )
    })?;
    if request.schema_version != REQUEST_SCHEMA_VERSION {
        return Err(rejection(
            RouterError::BadRequest("unsupported inference request schema_version".to_owned()),
            LlmInferenceDecisionAudit::denied_for_request(
                "unsupported_schema",
                request.context_pack.cells.len() as u64,
                citation_count(&request) as u64,
                request_has_api_key(&request),
            ),
        ));
    }
    if !request.enabled {
        return Err(rejection(
            RouterError::BadRequest(
                "request enabled must be true for the deterministic test-double".to_owned(),
            ),
            LlmInferenceDecisionAudit::denied_for_request(
                "request_disabled",
                request.context_pack.cells.len() as u64,
                citation_count(&request) as u64,
                request_has_api_key(&request),
            ),
        ));
    }
    if request.provider != TEST_DOUBLE_PROVIDER {
        return Err(rejection(
            RouterError::BadRequest(
                "only provider=test_double is supported by this local endpoint".to_owned(),
            ),
            LlmInferenceDecisionAudit::denied_for_request(
                "unsupported_provider",
                request.context_pack.cells.len() as u64,
                citation_count(&request) as u64,
                request_has_api_key(&request),
            ),
        ));
    }
    if request.model.as_deref().unwrap_or(TEST_DOUBLE_MODEL) != TEST_DOUBLE_MODEL {
        return Err(rejection(
            RouterError::BadRequest(
                "only model=deterministic-echo-v1 is supported by this local endpoint".to_owned(),
            ),
            LlmInferenceDecisionAudit::denied_for_request(
                "unsupported_model",
                request.context_pack.cells.len() as u64,
                citation_count(&request) as u64,
                request_has_api_key(&request),
            ),
        ));
    }
    if request
        .api_key
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
    {
        return Err(rejection(
            RouterError::BadRequest(
                "api_key must not be sent to the local deterministic test-double".to_owned(),
            ),
            LlmInferenceDecisionAudit::denied_for_request(
                "request_api_key_present",
                request.context_pack.cells.len() as u64,
                citation_count(&request) as u64,
                true,
            ),
        ));
    }
    if request.prompt.trim().is_empty() {
        return Err(rejection(
            RouterError::BadRequest("prompt must not be empty".to_owned()),
            LlmInferenceDecisionAudit::denied_for_request(
                "empty_prompt",
                request.context_pack.cells.len() as u64,
                citation_count(&request) as u64,
                false,
            ),
        ));
    }
    if request.context_pack.cells.is_empty() {
        return Err(rejection(
            RouterError::BadRequest("context_pack.cells must not be empty".to_owned()),
            LlmInferenceDecisionAudit::denied_for_request("empty_context_pack", 0, 0, false),
        ));
    }

    let used_context_cell_ids = request
        .context_pack
        .cells
        .iter()
        .map(|cell| cell.cell_id)
        .collect::<Vec<_>>();
    let citations = citations(&request);
    let citation_count = citations.len() as u64;
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
    let body = serde_json::to_string(&response).map_err(|error| {
        rejection(
            RouterError::from(error),
            LlmInferenceDecisionAudit::denied("response_serialization_failed"),
        )
    })?;
    Ok(LlmInferenceResult {
        body,
        audit: LlmInferenceDecisionAudit::allowed(
            request.context_pack.cells.len() as u64,
            citation_count,
        ),
    })
}

fn rejection(error: RouterError, audit: LlmInferenceDecisionAudit) -> LlmInferenceRejection {
    LlmInferenceRejection::new(error, audit)
}

fn request_has_api_key(request: &LlmInferenceRequest) -> bool {
    request
        .api_key
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
}

fn citations(request: &LlmInferenceRequest) -> Vec<String> {
    request
        .context_pack
        .cells
        .iter()
        .filter_map(|cell| cell.citation.clone().or_else(|| cell.source_ref.clone()))
        .collect()
}

fn citation_count(request: &LlmInferenceRequest) -> usize {
    request
        .context_pack
        .cells
        .iter()
        .filter(|cell| cell.citation.is_some() || cell.source_ref.is_some())
        .count()
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
mod tests;
