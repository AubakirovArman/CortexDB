use serde::Deserialize;

use crate::embedding;
use crate::responses::RouterError;
use crate::router::query_param_opt_decoded;

#[derive(Deserialize)]
pub(super) struct ContextRequest {
    pub(super) retrieve_aql: String,
    #[serde(default)]
    pub(super) embed_query: bool,
    #[serde(default)]
    pub(super) query_text: Option<String>,
}

pub(super) fn context_request(query: &str, body: &[u8]) -> Result<ContextRequest, RouterError> {
    let raw = String::from_utf8_lossy(body);
    let trimmed = raw.trim();
    let embed_query =
        embedding::parse_bool_param(query_param_opt_decoded(query, "embed_query"), "embed_query")?;
    if trimmed.starts_with('{') {
        let mut request: ContextRequest = serde_json::from_str(trimmed)
            .map_err(|error| RouterError::BadRequest(error.to_string()))?;
        request.embed_query |= embed_query;
        validate_context_request(request)
    } else if trimmed.is_empty() {
        Err(RouterError::BadRequest(
            "context request body must contain retrieve AQL".to_owned(),
        ))
    } else {
        validate_context_request(ContextRequest {
            retrieve_aql: trimmed.to_owned(),
            embed_query,
            query_text: None,
        })
    }
}

fn validate_context_request(request: ContextRequest) -> Result<ContextRequest, RouterError> {
    if request.retrieve_aql.trim().is_empty() {
        return Err(RouterError::BadRequest(
            "retrieve_aql must not be empty".to_owned(),
        ));
    }
    if request
        .query_text
        .as_deref()
        .is_some_and(|value| value.trim().is_empty())
    {
        return Err(RouterError::BadRequest(
            "query_text must not be empty when provided".to_owned(),
        ));
    }
    Ok(request)
}
