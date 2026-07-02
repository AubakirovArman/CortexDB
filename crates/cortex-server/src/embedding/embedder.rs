use cortex_engine::{Embedder, EmbeddingClientConfig, EngineError, EngineResult};

use super::client::embed_query_with_config;

/// An [`Embedder`] backed by the server's HTTP(S) embedding client, so the engine
/// can auto-embed cell bodies at ingest without doing network I/O itself.
pub(crate) struct HttpEmbedder {
    config: EmbeddingClientConfig,
}

impl HttpEmbedder {
    pub(crate) fn new(config: EmbeddingClientConfig) -> Self {
        Self { config }
    }
}

impl Embedder for HttpEmbedder {
    fn dimension(&self) -> usize {
        // Determined by the provider; the ingest path stores whatever vector
        // length the provider returns and does not consult this value.
        0
    }

    fn embed_batch(&self, texts: &[&str]) -> EngineResult<Vec<Vec<i16>>> {
        texts
            .iter()
            .map(|text| {
                embed_query_with_config(&self.config, text).map_err(|error| {
                    EngineError::StorageInvariant(format!("embedding failed: {error}"))
                })
            })
            .collect()
    }
}
