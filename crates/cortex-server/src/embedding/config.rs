use cortex_engine::EmbeddingClientConfig;

use crate::responses::RouterError;

use super::client::embed_query_with_config;
use super::MISSING_VECTOR_OR_CONFIG;

pub(crate) fn embed_query_from_env(text: &str) -> Result<Vec<i16>, RouterError> {
    let config = embedding_config_from_env()?.ok_or_else(missing_vector_or_config_error)?;
    embed_query_with_config(&config, text)
}

/// Builds an [`HttpEmbedder`](super::embedder::HttpEmbedder) from the
/// `CORTEXDB_EMBEDDING_*` env vars, or `None` when no endpoint is configured.
pub(crate) fn embedder_from_env() -> Result<Option<super::embedder::HttpEmbedder>, RouterError> {
    Ok(embedding_config_from_env()?.map(super::embedder::HttpEmbedder::new))
}

pub(crate) fn missing_vector_or_config_error() -> RouterError {
    RouterError::BadRequest(MISSING_VECTOR_OR_CONFIG.to_owned())
}

pub(crate) fn parse_bool_param(raw: Option<String>, name: &str) -> Result<bool, RouterError> {
    let Some(value) = raw else {
        return Ok(false);
    };
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(RouterError::BadRequest(format!("{name} must be boolean"))),
    }
}

fn embedding_config_from_env() -> Result<Option<EmbeddingClientConfig>, RouterError> {
    cortex_embed_client::config_from_env().map_err(RouterError::BadRequest)
}
