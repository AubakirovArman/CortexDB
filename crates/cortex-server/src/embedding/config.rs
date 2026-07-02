use cortex_engine::EmbeddingClientConfig;

use crate::responses::RouterError;

use super::client::embed_query_with_config;
use super::MISSING_VECTOR_OR_CONFIG;

const DEFAULT_TIMEOUT_MS: u64 = 2_000;

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
    let url = match std::env::var("CORTEXDB_EMBEDDING_URL") {
        Ok(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return Err(RouterError::BadRequest(
                    "CORTEXDB_EMBEDDING_URL must not be empty".to_owned(),
                ));
            }
            trimmed.to_owned()
        }
        Err(std::env::VarError::NotPresent) => return Ok(None),
        Err(error) => {
            return Err(RouterError::BadRequest(format!(
                "invalid CORTEXDB_EMBEDDING_URL: {error}"
            )))
        }
    };
    Ok(Some(EmbeddingClientConfig {
        url,
        model: optional_env("CORTEXDB_EMBEDDING_MODEL"),
        api_key: optional_env("CORTEXDB_EMBEDDING_API_KEY"),
        timeout_ms: embedding_timeout_ms_from_env()?,
    }))
}

fn embedding_timeout_ms_from_env() -> Result<u64, RouterError> {
    match std::env::var("CORTEXDB_EMBEDDING_TIMEOUT_MS") {
        Ok(raw) => {
            let value = raw.trim().parse::<u64>().map_err(|_| {
                RouterError::BadRequest(
                    "CORTEXDB_EMBEDDING_TIMEOUT_MS must be a positive integer".to_owned(),
                )
            })?;
            if value == 0 {
                return Err(RouterError::BadRequest(
                    "CORTEXDB_EMBEDDING_TIMEOUT_MS must be greater than zero".to_owned(),
                ));
            }
            Ok(value)
        }
        Err(std::env::VarError::NotPresent) => Ok(DEFAULT_TIMEOUT_MS),
        Err(error) => Err(RouterError::BadRequest(format!(
            "invalid CORTEXDB_EMBEDDING_TIMEOUT_MS: {error}"
        ))),
    }
}

fn optional_env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}
