//! Blocking Rust HTTP client for CortexDB.
//!
//! `cortex-sdk` provides a synchronous, ergonomic client for the CortexDB
//! Core Alpha HTTP API. All methods return strongly-typed responses or
//! `serde_json::Value` for raw access.
//!
//! # Quickstart
//!
//! ```no_run
//! use cortex_sdk::CortexDbClient;
//!
//! let client = CortexDbClient::new("http://127.0.0.1:8181");
//! let health = client.health_response().unwrap();
//! println!("Server version: {}", health.server_version);
//! ```

use std::time::Duration;

use serde::de::DeserializeOwned;
use thiserror::Error;

mod http;
mod types;

use http::{append_query_param, parse_response, path};
pub use types::{
    AnnEvaluationResponse, AnnNoFallbackDecision, AnnSearchReport, AqlCellResponse, AqlResponse,
    CellLookupResponse, CellResponse, ContextPackAnomalyResponse, ContextPackCellResponse,
    ContextPackResponse, DeleteJobResponse, ErrorCode, ErrorResponse, EvidenceResponse,
    ExplainResponse, GuardResponse, HealthResponse, IngestResponse, IngestionJobResponse,
    IngestionJobStatus, NumericConflictResponse, PutCellResponse, RememberResponse, SearchResponse,
    SearchResult, SourceRefResponse, StatsResponse, ValidationResponse, VectorAlgorithm,
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
    #[error("cortexdb error: {0:?}")]
    CortexDb(ErrorResponse),
    #[error("json decode error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type SdkResult<T> = Result<T, SdkError>;

/// Blocking HTTP client for the CortexDB API.
///
/// Create with [`CortexDbClient::new`] and chain configuration:
///
/// ```no_run
/// use cortex_sdk::CortexDbClient;
/// let client = CortexDbClient::new("http://127.0.0.1:8181")
///     .with_token("secret")
///     .with_tenant("tenant:alpha");
/// ```
#[derive(Clone, Debug)]
pub struct CortexDbClient {
    base_url: String,
    token: Option<String>,
    tenant: Option<String>,
    agent: ureq::Agent,
    max_retries: u32,
    retry_delay: Duration,
}

impl CortexDbClient {
    /// Create a client with the default 10-second timeout.
    pub fn new(base_url: impl Into<String>) -> Self {
        Self::with_timeout(base_url, Duration::from_secs(10))
    }

    /// Create a client with a custom timeout.
    pub fn with_timeout(base_url: impl Into<String>, timeout: Duration) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_owned(),
            token: None,
            tenant: None,
            agent: ureq::AgentBuilder::new().timeout(timeout).build(),
            max_retries: 0,
            retry_delay: Duration::from_millis(500),
        }
    }

    /// Set the Bearer token for authenticated requests.
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// Set the tenant ID for per-tenant database routing.
    pub fn with_tenant(mut self, tenant: impl Into<String>) -> Self {
        self.tenant = Some(tenant.into());
        self
    }

    /// Set retry behaviour for idempotent requests.
    pub fn with_retries(mut self, max_retries: u32, retry_delay: Duration) -> Self {
        self.max_retries = max_retries;
        self.retry_delay = retry_delay;
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

    pub fn ingestion_job_response(&self, job_id: u64) -> SdkResult<IngestionJobResponse> {
        decode_value(self.ingestion_job(job_id)?)
    }

    pub fn delete_ingestion_job(&self, job_id: u64) -> SdkResult<DeleteJobResponse> {
        decode_value(self.delete(&format!("/v1/ingest/jobs/{job_id}"))?)
    }

    pub fn retry_ingestion_job(&self, job_id: u64) -> SdkResult<IngestionJobResponse> {
        decode_value(self.post(&format!("/v1/ingest/jobs/{job_id}/retry"), "")?)
    }

    fn get(&self, path: &str) -> SdkResult<serde_json::Value> {
        self.execute(|this| {
            this.authorized(this.agent.get(&this.url(path)))
                .call()
                .map_err(Box::new)
        })
    }

    fn delete(&self, path: &str) -> SdkResult<serde_json::Value> {
        self.execute(|this| {
            this.authorized(this.agent.delete(&this.url(path)))
                .call()
                .map_err(Box::new)
        })
    }

    fn post(&self, path: &str, body: &str) -> SdkResult<serde_json::Value> {
        self.execute(|this| {
            this.authorized(this.agent.post(&this.url(path)))
                .send_string(body)
                .map_err(Box::new)
        })
    }

    fn authorized(&self, request: ureq::Request) -> ureq::Request {
        if let Some(token) = &self.token {
            request.set("authorization", &format!("Bearer {token}"))
        } else {
            request
        }
    }

    fn execute(
        &self,
        call: impl Fn(&Self) -> Result<ureq::Response, Box<ureq::Error>>,
    ) -> SdkResult<serde_json::Value> {
        let mut attempt = 0u32;
        loop {
            match call(self) {
                Ok(response) => return parse_response(response),
                Err(boxed) => match *boxed {
                    ureq::Error::Status(status, response) => {
                        let body = response.into_string().unwrap_or_default();
                        // Try to decode structured error response.
                        if let Ok(error_response) = serde_json::from_str::<ErrorResponse>(&body) {
                            return Err(SdkError::CortexDb(error_response));
                        }
                        if self.is_retryable(status) && attempt < self.max_retries {
                            attempt += 1;
                            std::thread::sleep(self.retry_delay * attempt);
                            continue;
                        }
                        return Err(SdkError::HttpStatus { status, body });
                    }
                    error => {
                        if attempt < self.max_retries {
                            attempt += 1;
                            std::thread::sleep(self.retry_delay * attempt);
                            continue;
                        }
                        return Err(SdkError::Transport(error.to_string()));
                    }
                },
            }
        }
    }

    fn is_retryable(&self, status: u16) -> bool {
        matches!(status, 500 | 502 | 503 | 504)
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
