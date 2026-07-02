use super::transport::decode_value;
use super::CortexDbClient;
use crate::http::path;
use crate::{
    AnnEvaluationResponse, SdkResult, SearchExplainResponse, SearchResponse, VectorAlgorithm,
};

impl CortexDbClient {
    pub fn search_keyword(
        &self,
        scope: &str,
        query: &str,
        limit: usize,
    ) -> SdkResult<serde_json::Value> {
        self.post(
            &path(
                "/v1/search",
                &[
                    ("scope", scope),
                    ("mode", "keyword"),
                    ("q", query),
                    ("limit", &limit.to_string()),
                ],
            ),
            "",
        )
    }

    pub fn search_keyword_response(
        &self,
        scope: &str,
        query: &str,
        limit: usize,
    ) -> SdkResult<SearchResponse> {
        decode_value(self.search_keyword(scope, query, limit)?)
    }

    /// Searches with the query text embedded server-side at request time
    /// (`embed_query=true`), so the caller retrieves by meaning without holding
    /// an embedding client. `mode` is a `/v1/search` mode (`vector`, `hybrid`,
    /// or `auto`). Requires the server to have an embedding endpoint configured;
    /// otherwise the server responds fail-closed.
    pub fn search_embedded(
        &self,
        scope: &str,
        query: &str,
        mode: &str,
        limit: usize,
    ) -> SdkResult<serde_json::Value> {
        self.post(
            &path(
                "/v1/search",
                &[
                    ("scope", scope),
                    ("mode", mode),
                    ("q", query),
                    ("embed_query", "true"),
                    ("limit", &limit.to_string()),
                ],
            ),
            "",
        )
    }

    pub fn search_embedded_response(
        &self,
        scope: &str,
        query: &str,
        mode: &str,
        limit: usize,
    ) -> SdkResult<SearchResponse> {
        decode_value(self.search_embedded(scope, query, mode, limit)?)
    }

    pub fn search_auto(
        &self,
        scope: &str,
        query: &str,
        vector: Option<&str>,
        limit: usize,
    ) -> SdkResult<serde_json::Value> {
        let limit = limit.to_string();
        let mut params = vec![
            ("scope", scope),
            ("mode", "auto"),
            ("q", query),
            ("limit", limit.as_str()),
        ];
        if let Some(vector) = vector {
            params.push(("vector", vector));
        }
        self.post(&path("/v1/search", &params), "")
    }

    pub fn search_auto_response(
        &self,
        scope: &str,
        query: &str,
        vector: Option<&str>,
        limit: usize,
    ) -> SdkResult<SearchResponse> {
        decode_value(self.search_auto(scope, query, vector, limit)?)
    }

    pub fn search_explain(
        &self,
        scope: &str,
        query: &str,
        mode: &str,
        vector: Option<&str>,
        limit: usize,
    ) -> SdkResult<serde_json::Value> {
        let limit = limit.to_string();
        let mut params = vec![
            ("scope", scope),
            ("mode", mode),
            ("q", query),
            ("limit", limit.as_str()),
        ];
        if let Some(vector) = vector {
            params.push(("vector", vector));
        }
        self.post(&path("/v1/search/explain", &params), "")
    }

    pub fn search_explain_response(
        &self,
        scope: &str,
        query: &str,
        mode: &str,
        vector: Option<&str>,
        limit: usize,
    ) -> SdkResult<SearchExplainResponse> {
        decode_value(self.search_explain(scope, query, mode, vector, limit)?)
    }

    pub fn search_vector(
        &self,
        scope: &str,
        vector: &[i16],
        algorithm: VectorAlgorithm,
        limit: usize,
    ) -> SdkResult<serde_json::Value> {
        let literal = vector
            .iter()
            .map(i16::to_string)
            .collect::<Vec<_>>()
            .join(",");
        self.post(
            &path(
                "/v1/search",
                &[
                    ("scope", scope),
                    ("mode", "vector"),
                    ("algorithm", algorithm.as_str()),
                    ("vector", &literal),
                    ("limit", &limit.to_string()),
                ],
            ),
            "",
        )
    }

    pub fn search_vector_response(
        &self,
        scope: &str,
        vector: &[i16],
        algorithm: VectorAlgorithm,
        limit: usize,
    ) -> SdkResult<SearchResponse> {
        decode_value(self.search_vector(scope, vector, algorithm, limit)?)
    }

    pub fn evaluate_ann(
        &self,
        scope: &str,
        vector: &[i16],
        limit: usize,
    ) -> SdkResult<serde_json::Value> {
        let literal = vector
            .iter()
            .map(i16::to_string)
            .collect::<Vec<_>>()
            .join(",");
        self.post(
            &path(
                "/v1/search/ann-evaluate",
                &[
                    ("scope", scope),
                    ("vector", &literal),
                    ("limit", &limit.to_string()),
                ],
            ),
            "",
        )
    }

    pub fn evaluate_ann_response(
        &self,
        scope: &str,
        vector: &[i16],
        limit: usize,
    ) -> SdkResult<AnnEvaluationResponse> {
        decode_value(self.evaluate_ann(scope, vector, limit)?)
    }
}
