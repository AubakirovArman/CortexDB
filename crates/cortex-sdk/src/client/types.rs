use std::time::Duration;

use thiserror::Error;

use crate::{AqlBuildError, ErrorResponse};

#[derive(Debug, Error)]
pub enum SdkError {
    #[error("aql build error: {0}")]
    AqlBuild(#[from] AqlBuildError),
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
    pub(crate) base_url: String,
    pub(crate) token: Option<String>,
    pub(crate) tenant: Option<String>,
    pub(crate) agent: ureq::Agent,
    pub(crate) max_retries: u32,
    pub(crate) retry_delay: Duration,
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
}
