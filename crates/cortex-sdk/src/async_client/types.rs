use std::time::Duration;

/// Async HTTP client for the CortexDB API.
///
/// Enable with the `async` feature:
///
/// ```toml
/// cortex-sdk = { version = "...", features = ["async"] }
/// ```
#[derive(Clone, Debug)]
pub struct AsyncCortexDbClient {
    pub(crate) base_url: String,
    pub(crate) token: Option<String>,
    pub(crate) tenant: Option<String>,
    pub(crate) client: reqwest::Client,
    pub(crate) timeout: Duration,
    pub(crate) max_retries: u32,
    pub(crate) retry_delay: Duration,
}

impl AsyncCortexDbClient {
    /// Create an async client with the default 10-second per-request timeout.
    pub fn new(base_url: impl Into<String>) -> Self {
        Self::with_timeout(base_url, Duration::from_secs(10))
    }

    /// Create an async client with a custom per-request timeout.
    pub fn with_timeout(base_url: impl Into<String>, timeout: Duration) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_owned(),
            token: None,
            tenant: None,
            client: reqwest::Client::new(),
            timeout,
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

    /// Set retry behaviour for transient requests.
    pub fn with_retries(mut self, max_retries: u32, retry_delay: Duration) -> Self {
        self.max_retries = max_retries;
        self.retry_delay = retry_delay;
        self
    }
}
