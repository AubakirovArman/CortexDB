use std::time::Duration;

use serde::de::DeserializeOwned;
use thiserror::Error;

mod http;
mod types;

use http::{append_query_param, parse_response, path};
pub use types::{
    AnnEvaluationResponse, AnnSearchReport, AqlCellResponse, AqlResponse, CellLookupResponse,
    CellResponse, ContextPackAnomalyResponse, ContextPackCellResponse, ContextPackResponse,
    EvidenceResponse, ExplainResponse, GuardResponse, HealthResponse, IngestResponse,
    NumericConflictResponse, PutCellResponse, RememberResponse, SearchResponse, SearchResult,
    SourceRefResponse, StatsResponse, ValidationResponse, VectorAlgorithm,
    VerificationReportResponse,
};

#[cfg(test)]
mod tests;

#[derive(Debug, Error)]
pub enum SdkError {
    #[error("transport error: {0}")]
    Transport(String),
    #[error("http status {status}: {body}")]
    HttpStatus { status: u16, body: String },
    #[error("json decode error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type SdkResult<T> = Result<T, SdkError>;

#[derive(Clone, Debug)]
pub struct CortexDbClient {
    base_url: String,
    token: Option<String>,
    tenant: Option<String>,
    agent: ureq::Agent,
}

impl CortexDbClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self::with_timeout(base_url, Duration::from_secs(10))
    }

    pub fn with_timeout(base_url: impl Into<String>, timeout: Duration) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_owned(),
            token: None,
            tenant: None,
            agent: ureq::AgentBuilder::new().timeout(timeout).build(),
        }
    }

    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    pub fn with_tenant(mut self, tenant: impl Into<String>) -> Self {
        self.tenant = Some(tenant.into());
        self
    }

    pub fn health(&self) -> SdkResult<serde_json::Value> {
        self.get("/v1/health")
    }

    pub fn health_response(&self) -> SdkResult<HealthResponse> {
        decode_value(self.health()?)
    }

    pub fn stats(&self) -> SdkResult<serde_json::Value> {
        self.get("/v1/stats")
    }

    pub fn stats_response(&self) -> SdkResult<StatsResponse> {
        decode_value(self.stats()?)
    }

    pub fn validate(&self) -> SdkResult<serde_json::Value> {
        self.get("/v1/validate")
    }

    pub fn validate_response(&self) -> SdkResult<ValidationResponse> {
        decode_value(self.validate()?)
    }

    pub fn put_cell(&self, cell_id: u64, payload: &str) -> SdkResult<serde_json::Value> {
        self.post(
            &path("/v1/cell", &[("cell_id", &cell_id.to_string())]),
            payload,
        )
    }

    pub fn put_cell_response(&self, cell_id: u64, payload: &str) -> SdkResult<PutCellResponse> {
        decode_value(self.put_cell(cell_id, payload)?)
    }

    pub fn get_cell(&self, cell_id: u64) -> SdkResult<serde_json::Value> {
        self.get(&path("/v1/cell", &[("cell_id", &cell_id.to_string())]))
    }

    pub fn get_cell_response(&self, cell_id: u64) -> SdkResult<CellLookupResponse> {
        decode_value(self.get_cell(cell_id)?)
    }

    pub fn tombstone_cell(&self, cell_id: u64) -> SdkResult<serde_json::Value> {
        self.delete(&path("/v1/cell", &[("cell_id", &cell_id.to_string())]))
    }

    pub fn flush(&self) -> SdkResult<serde_json::Value> {
        self.post("/v1/flush", "")
    }

    pub fn compact(&self) -> SdkResult<serde_json::Value> {
        self.post("/v1/compact", "")
    }

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

    pub fn aql(&self, scope: &str, statement: &str) -> SdkResult<serde_json::Value> {
        self.post(&path("/v1/aql", &[("scope", scope)]), statement)
    }

    pub fn aql_response(&self, scope: &str, statement: &str) -> SdkResult<AqlResponse> {
        decode_value(self.aql(scope, statement)?)
    }

    pub fn context(&self, scope: &str, statement: &str) -> SdkResult<serde_json::Value> {
        self.post(&path("/v1/context", &[("scope", scope)]), statement)
    }

    pub fn context_response(&self, scope: &str, statement: &str) -> SdkResult<ContextPackResponse> {
        decode_value(self.context(scope, statement)?)
    }

    pub fn verify(&self, scope: &str, statement: &str) -> SdkResult<serde_json::Value> {
        self.post(&path("/v1/verify", &[("scope", scope)]), statement)
    }

    pub fn verify_response(
        &self,
        scope: &str,
        statement: &str,
    ) -> SdkResult<VerificationReportResponse> {
        decode_value(self.verify(scope, statement)?)
    }

    pub fn remember(&self, scope: &str, statement: &str) -> SdkResult<serde_json::Value> {
        self.post(&path("/v1/remember", &[("scope", scope)]), statement)
    }

    pub fn remember_response(&self, scope: &str, statement: &str) -> SdkResult<RememberResponse> {
        decode_value(self.remember(scope, statement)?)
    }

    pub fn ingest_text(
        &self,
        scope: &str,
        source: &str,
        text: &str,
    ) -> SdkResult<serde_json::Value> {
        self.post(
            &path("/v1/ingest/text", &[("scope", scope), ("source", source)]),
            text,
        )
    }

    pub fn ingest_text_response(
        &self,
        scope: &str,
        source: &str,
        text: &str,
    ) -> SdkResult<IngestResponse> {
        decode_value(self.ingest_text(scope, source, text)?)
    }

    pub fn ingest_json(
        &self,
        scope: &str,
        source: &str,
        document: &str,
    ) -> SdkResult<serde_json::Value> {
        self.post(
            &path("/v1/ingest/json", &[("scope", scope), ("source", source)]),
            document,
        )
    }

    pub fn ingest_json_response(
        &self,
        scope: &str,
        source: &str,
        document: &str,
    ) -> SdkResult<IngestResponse> {
        decode_value(self.ingest_json(scope, source, document)?)
    }

    pub fn ingest_csv(
        &self,
        scope: &str,
        source: &str,
        document: &str,
    ) -> SdkResult<serde_json::Value> {
        self.post(
            &path("/v1/ingest/csv", &[("scope", scope), ("source", source)]),
            document,
        )
    }

    pub fn ingest_csv_response(
        &self,
        scope: &str,
        source: &str,
        document: &str,
    ) -> SdkResult<IngestResponse> {
        decode_value(self.ingest_csv(scope, source, document)?)
    }

    pub fn ingestion_job(&self, job_id: u64) -> SdkResult<serde_json::Value> {
        self.get(&format!("/v1/ingest/jobs/{job_id}"))
    }

    fn get(&self, path: &str) -> SdkResult<serde_json::Value> {
        self.finish(self.authorized(self.agent.get(&self.url(path))).call())
    }

    fn delete(&self, path: &str) -> SdkResult<serde_json::Value> {
        self.finish(self.authorized(self.agent.delete(&self.url(path))).call())
    }

    fn post(&self, path: &str, body: &str) -> SdkResult<serde_json::Value> {
        self.finish(
            self.authorized(self.agent.post(&self.url(path)))
                .send_string(body),
        )
    }

    fn authorized(&self, request: ureq::Request) -> ureq::Request {
        if let Some(token) = &self.token {
            request.set("authorization", &format!("Bearer {token}"))
        } else {
            request
        }
    }

    fn finish(&self, result: Result<ureq::Response, ureq::Error>) -> SdkResult<serde_json::Value> {
        match result {
            Ok(response) => parse_response(response),
            Err(ureq::Error::Status(status, response)) => {
                let body = response.into_string().unwrap_or_default();
                Err(SdkError::HttpStatus { status, body })
            }
            Err(error) => Err(SdkError::Transport(error.to_string())),
        }
    }

    fn url(&self, path: &str) -> String {
        let scoped_path = match self.tenant.as_deref() {
            Some("default") | None => path.to_owned(),
            Some(tenant) => append_query_param(path, "tenant", tenant),
        };
        format!("{}{}", self.base_url, scoped_path)
    }
}

fn decode_value<T: DeserializeOwned>(value: serde_json::Value) -> SdkResult<T> {
    serde_json::from_value(value).map_err(SdkError::Json)
}
