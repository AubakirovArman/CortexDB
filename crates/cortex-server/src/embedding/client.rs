use cortex_engine::EmbeddingClientConfig;

use crate::responses::RouterError;

/// Embeds `text` through the shared [`cortex_embed_client`] transport, mapping
/// its human-readable error to a `RouterError`. The request shape and i16
/// quantization live in the shared crate so the server and CLI stay identical.
pub(super) fn embed_query_with_config(
    config: &EmbeddingClientConfig,
    text: &str,
) -> Result<Vec<i16>, RouterError> {
    cortex_embed_client::embed_query(config, text).map_err(RouterError::BadRequest)
}

pub(crate) fn format_vector_literal(vector: &[i16]) -> String {
    vector
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use cortex_engine::EmbeddingClientConfig;

    use super::*;

    // Live check against a real (TLS) embedding provider. Ignored by default;
    // run with `--ignored` and CORTEXDB_EMBEDDING_URL/MODEL/API_KEY set.
    #[test]
    #[ignore = "requires network and CORTEXDB_EMBEDDING_* env vars"]
    fn live_embed_query_over_https() {
        let config = EmbeddingClientConfig {
            url: std::env::var("CORTEXDB_EMBEDDING_URL").expect("CORTEXDB_EMBEDDING_URL"),
            model: std::env::var("CORTEXDB_EMBEDDING_MODEL").ok(),
            api_key: std::env::var("CORTEXDB_EMBEDDING_API_KEY").ok(),
            timeout_ms: 30_000,
        };
        let vector = embed_query_with_config(&config, "solar plant capital budget").unwrap();
        assert!(
            vector.len() >= 256,
            "expected a real embedding vector, got len {}",
            vector.len()
        );
        assert!(
            vector.iter().any(|&value| value != 0),
            "embedding vector must not be all zeros"
        );
    }
}
