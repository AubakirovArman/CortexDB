use serde::de::DeserializeOwned;

use super::AsyncCortexDbClient;
use crate::http::append_query_param;
use crate::{ErrorResponse, SdkError, SdkResult};

impl AsyncCortexDbClient {
    pub(crate) async fn get(&self, path: &str) -> SdkResult<serde_json::Value> {
        self.execute(reqwest::Method::GET, path, None).await
    }

    pub(crate) async fn delete(&self, path: &str) -> SdkResult<serde_json::Value> {
        self.execute(reqwest::Method::DELETE, path, None).await
    }

    pub(crate) async fn post(&self, path: &str, body: &str) -> SdkResult<serde_json::Value> {
        self.execute(reqwest::Method::POST, path, Some(body.to_owned()))
            .await
    }

    pub(crate) async fn post_text(&self, path: &str, body: &str) -> SdkResult<String> {
        self.execute_text(reqwest::Method::POST, path, Some(body.to_owned()))
            .await
    }

    async fn execute(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<String>,
    ) -> SdkResult<serde_json::Value> {
        let text = self.execute_text(method, path, body).await?;
        serde_json::from_str(&text).map_err(SdkError::Json)
    }

    async fn execute_text(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<String>,
    ) -> SdkResult<String> {
        let mut attempt = 0u32;
        loop {
            match self.send_once(method.clone(), path, body.clone()).await {
                Ok(response) => {
                    let status = response.status().as_u16();
                    if response.status().is_success() {
                        return response
                            .text()
                            .await
                            .map_err(|error| SdkError::Transport(error.to_string()));
                    }
                    if self.is_retryable(status) && attempt < self.max_retries {
                        attempt += 1;
                        self.sleep_before_retry(attempt).await;
                        continue;
                    }
                    let body = response
                        .text()
                        .await
                        .map_err(|error| SdkError::Transport(error.to_string()))?;
                    if let Ok(error_response) = serde_json::from_str::<ErrorResponse>(&body) {
                        return Err(SdkError::CortexDb(error_response));
                    }
                    return Err(SdkError::HttpStatus { status, body });
                }
                Err(error) => {
                    if attempt < self.max_retries {
                        attempt += 1;
                        self.sleep_before_retry(attempt).await;
                        continue;
                    }
                    return Err(SdkError::Transport(error.to_string()));
                }
            }
        }
    }

    async fn send_once(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<String>,
    ) -> Result<reqwest::Response, reqwest::Error> {
        let mut request = self
            .client
            .request(method, self.url(path))
            .timeout(self.timeout);
        if let Some(token) = &self.token {
            request = request.bearer_auth(token);
        }
        if let Some(body) = body {
            request = request.body(body);
        }
        request.send().await
    }

    async fn sleep_before_retry(&self, attempt: u32) {
        tokio::time::sleep(self.retry_delay * attempt).await;
    }

    fn is_retryable(&self, status: u16) -> bool {
        matches!(status, 500 | 502 | 503 | 504)
    }

    pub(crate) fn url(&self, path: &str) -> String {
        let scoped_path = match self.tenant.as_deref() {
            Some("default") | None => path.to_owned(),
            Some(tenant) => append_query_param(path, "tenant", tenant),
        };
        format!("{}{}", self.base_url, scoped_path)
    }
}

pub(crate) fn decode_value<T: DeserializeOwned>(value: serde_json::Value) -> SdkResult<T> {
    serde_json::from_value(value).map_err(SdkError::Json)
}
