use std::time::Duration;

use thiserror::Error;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VectorAlgorithm {
    Ann,
    Exact,
}

impl VectorAlgorithm {
    fn as_str(self) -> &'static str {
        match self {
            Self::Ann => "ann",
            Self::Exact => "exact",
        }
    }
}

#[derive(Clone, Debug)]
pub struct CortexDbClient {
    base_url: String,
    token: Option<String>,
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
            agent: ureq::AgentBuilder::new().timeout(timeout).build(),
        }
    }

    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    pub fn health(&self) -> SdkResult<serde_json::Value> {
        self.get("/v1/health")
    }

    pub fn stats(&self) -> SdkResult<serde_json::Value> {
        self.get("/v1/stats")
    }

    pub fn validate(&self) -> SdkResult<serde_json::Value> {
        self.get("/v1/validate")
    }

    pub fn put_cell(&self, cell_id: u64, payload: &str) -> SdkResult<serde_json::Value> {
        self.post(
            &path("/v1/cell", &[("cell_id", &cell_id.to_string())]),
            payload,
        )
    }

    pub fn get_cell(&self, cell_id: u64) -> SdkResult<serde_json::Value> {
        self.get(&path("/v1/cell", &[("cell_id", &cell_id.to_string())]))
    }

    pub fn tombstone_cell(&self, cell_id: u64) -> SdkResult<serde_json::Value> {
        self.delete(&path("/v1/cell", &[("cell_id", &cell_id.to_string())]))
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

    pub fn aql(&self, scope: &str, statement: &str) -> SdkResult<serde_json::Value> {
        self.post(&path("/v1/aql", &[("scope", scope)]), statement)
    }

    pub fn context(&self, scope: &str, statement: &str) -> SdkResult<serde_json::Value> {
        self.post(&path("/v1/context", &[("scope", scope)]), statement)
    }

    pub fn verify(&self, scope: &str, statement: &str) -> SdkResult<serde_json::Value> {
        self.post(&path("/v1/verify", &[("scope", scope)]), statement)
    }

    pub fn remember(&self, scope: &str, statement: &str) -> SdkResult<serde_json::Value> {
        self.post(&path("/v1/remember", &[("scope", scope)]), statement)
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
        format!("{}{}", self.base_url, path)
    }
}

fn parse_response(response: ureq::Response) -> SdkResult<serde_json::Value> {
    Ok(serde_json::from_str(
        &response.into_string().unwrap_or_default(),
    )?)
}

fn path(base: &str, query: &[(&str, &str)]) -> String {
    let encoded = query
        .iter()
        .map(|(key, value)| format!("{}={}", escape(key), escape(value)))
        .collect::<Vec<_>>()
        .join("&");
    if encoded.is_empty() {
        base.to_owned()
    } else {
        format!("{base}?{encoded}")
    }
}

fn escape(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_encodes_search_query_contract() {
        let value = path(
            "/v1/search",
            &[
                ("scope", "project:investments"),
                ("mode", "keyword"),
                ("q", "solar budget"),
                ("limit", "10"),
            ],
        );
        assert_eq!(
            value,
            "/v1/search?scope=project%3Ainvestments&mode=keyword&q=solar+budget&limit=10"
        );
    }

    #[test]
    fn vector_algorithm_is_wire_stable() {
        assert_eq!(VectorAlgorithm::Ann.as_str(), "ann");
        assert_eq!(VectorAlgorithm::Exact.as_str(), "exact");
    }
}
