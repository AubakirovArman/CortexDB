use serde::de::DeserializeOwned;

use super::{CortexDbClient, SdkError, SdkResult};
use crate::http::{append_query_param, parse_response};
use crate::ErrorResponse;

impl CortexDbClient {
    pub(crate) fn get(&self, path: &str) -> SdkResult<serde_json::Value> {
        self.execute(|this| {
            this.authorized(this.agent.get(&this.url(path)))
                .call()
                .map_err(Box::new)
        })
    }

    pub(crate) fn delete(&self, path: &str) -> SdkResult<serde_json::Value> {
        self.execute(|this| {
            this.authorized(this.agent.delete(&this.url(path)))
                .call()
                .map_err(Box::new)
        })
    }

    pub(crate) fn post(&self, path: &str, body: &str) -> SdkResult<serde_json::Value> {
        self.execute(|this| {
            this.authorized(this.agent.post(&this.url(path)))
                .send_string(body)
                .map_err(Box::new)
        })
    }

    pub(crate) fn post_text(&self, path: &str, body: &str) -> SdkResult<String> {
        self.execute_text(|this| {
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

    fn execute_text(
        &self,
        call: impl Fn(&Self) -> Result<ureq::Response, Box<ureq::Error>>,
    ) -> SdkResult<String> {
        let mut attempt = 0u32;
        loop {
            match call(self) {
                Ok(response) => return Ok(response.into_string().unwrap_or_default()),
                Err(boxed) => match *boxed {
                    ureq::Error::Status(status, response) => {
                        let body = response.into_string().unwrap_or_default();
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
