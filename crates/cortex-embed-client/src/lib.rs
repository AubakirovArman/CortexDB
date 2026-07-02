//! Shared HTTP embedding client for CortexDB. Both the server (`/v1/search`,
//! `/v1/ingest/text?embed=true`) and the offline CLI embed query/chunk text
//! through this one module, so the request shape and — critically — the i16
//! quantization are identical everywhere and can never drift between callers.
//!
//! The engine stays network-free; this crate is the single network-bearing
//! embedding transport. Errors are human-readable strings that each caller maps
//! to its own error type.

use std::time::Duration;

use serde_json::{Map, Value};

pub use cortex_engine::EmbeddingClientConfig;

const DEFAULT_TIMEOUT_MS: u64 = 2_000;

/// Embeds `text` via the configured OpenAI-compatible embedding endpoint and
/// returns the provider's vector quantized to `i16`. Speaks both `http://` and
/// `https://` (e.g. a LiteLLM proxy). Fails on empty text, transport errors,
/// invalid JSON, or an empty/missing vector in the response.
pub fn embed_query(config: &EmbeddingClientConfig, text: &str) -> Result<Vec<i16>, String> {
    if text.trim().is_empty() {
        return Err("embed_query requires non-empty query text".to_owned());
    }
    let body = embedding_request_body(config, text)?;
    let timeout = Duration::from_millis(config.timeout_ms.max(1));
    let agent = ureq::AgentBuilder::new().timeout(timeout).build();
    let mut request = agent
        .post(&config.url)
        .set("Content-Type", "application/json")
        .set("Accept", "application/json");
    if let Some(api_key) = config.api_key.as_deref().filter(|value| !value.is_empty()) {
        request = request.set("Authorization", &format!("Bearer {api_key}"));
    }
    let response = request
        .send_string(&body)
        .map_err(|error| format!("embedding request failed: {error}"))?;
    let payload = response
        .into_string()
        .map_err(|error| format!("embedding request failed: {error}"))?;
    let json: Value = serde_json::from_str(&payload)
        .map_err(|error| format!("embedding provider returned invalid JSON: {error}"))?;
    vector_from_embedding_json(&json)
        .filter(|vector| !vector.is_empty())
        .ok_or_else(|| "embedding provider response must contain non-empty vector".to_owned())
}

/// Builds an [`EmbeddingClientConfig`] from the `CORTEXDB_EMBEDDING_*` env vars,
/// or `None` when `CORTEXDB_EMBEDDING_URL` is unset.
pub fn config_from_env() -> Result<Option<EmbeddingClientConfig>, String> {
    let url = match std::env::var("CORTEXDB_EMBEDDING_URL") {
        Ok(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return Err("CORTEXDB_EMBEDDING_URL must not be empty".to_owned());
            }
            trimmed.to_owned()
        }
        Err(std::env::VarError::NotPresent) => return Ok(None),
        Err(error) => return Err(format!("invalid CORTEXDB_EMBEDDING_URL: {error}")),
    };
    Ok(Some(EmbeddingClientConfig {
        url,
        model: optional_env("CORTEXDB_EMBEDDING_MODEL"),
        api_key: optional_env("CORTEXDB_EMBEDDING_API_KEY"),
        timeout_ms: timeout_ms_from_env()?,
    }))
}

fn timeout_ms_from_env() -> Result<u64, String> {
    match std::env::var("CORTEXDB_EMBEDDING_TIMEOUT_MS") {
        Ok(raw) => {
            let value = raw.trim().parse::<u64>().map_err(|_| {
                "CORTEXDB_EMBEDDING_TIMEOUT_MS must be a positive integer".to_owned()
            })?;
            if value == 0 {
                return Err("CORTEXDB_EMBEDDING_TIMEOUT_MS must be greater than zero".to_owned());
            }
            Ok(value)
        }
        Err(std::env::VarError::NotPresent) => Ok(DEFAULT_TIMEOUT_MS),
        Err(error) => Err(format!("invalid CORTEXDB_EMBEDDING_TIMEOUT_MS: {error}")),
    }
}

fn optional_env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn embedding_request_body(config: &EmbeddingClientConfig, text: &str) -> Result<String, String> {
    let mut body = Map::new();
    body.insert("input".to_owned(), Value::String(text.to_owned()));
    if let Some(model) = config.model.as_deref().filter(|value| !value.is_empty()) {
        body.insert("model".to_owned(), Value::String(model.to_owned()));
    }
    serde_json::to_string(&Value::Object(body)).map_err(|error| error.to_string())
}

fn vector_from_embedding_json(json: &Value) -> Option<Vec<i16>> {
    json.get("vector")
        .and_then(vector_array)
        .or_else(|| json.get("embedding").and_then(vector_array))
        .or_else(|| {
            json.get("data")?
                .as_array()?
                .first()
                .and_then(|item| item.get("embedding").and_then(vector_array))
        })
        .or_else(|| {
            json.get("data")?
                .as_array()?
                .first()
                .and_then(|item| item.get("vector").and_then(vector_array))
        })
}

fn vector_array(value: &Value) -> Option<Vec<i16>> {
    value
        .as_array()?
        .iter()
        .map(json_number_to_i16)
        .collect::<Option<Vec<_>>>()
}

fn json_number_to_i16(value: &Value) -> Option<i16> {
    let rendered = value.to_string();
    if !rendered.contains(['.', 'e', 'E']) {
        let number = value.as_i64()?;
        return i16::try_from(number).ok();
    }
    let number = value.as_f64()?;
    if !number.is_finite() {
        return None;
    }
    if number.fract() == 0.0 && number >= f64::from(i16::MIN) && number <= f64::from(i16::MAX) {
        return Some(number as i16);
    }
    let scaled = (number.clamp(-1.0, 1.0) * f64::from(i16::MAX)).round();
    Some(scaled as i16)
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    use super::*;

    #[test]
    fn parses_openai_compatible_embedding_json() {
        let json: serde_json::Value =
            serde_json::from_str(r#"{"data":[{"embedding":[0.0,0.5,-0.5]}]}"#).unwrap();

        let vector = vector_from_embedding_json(&json).unwrap();

        assert_eq!(vector, vec![0, 16_384, -16_384]);
    }

    #[test]
    fn rejects_empty_query_text() {
        let config = EmbeddingClientConfig {
            url: "http://127.0.0.1:1/embed".to_owned(),
            model: None,
            api_key: None,
            timeout_ms: 1_000,
        };
        assert!(embed_query(&config, "   ").is_err());
    }

    #[test]
    fn embedding_client_posts_and_quantizes() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                .unwrap();
            let mut request = String::new();
            let mut buffer = [0u8; 1024];
            while !request.contains("\"input\":\"semantic question\"") {
                match stream.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(read) => request.push_str(&String::from_utf8_lossy(&buffer[..read])),
                    Err(_) => break,
                }
            }
            assert!(request.contains("POST /embed HTTP/1.1"));
            assert!(request.contains("\"model\":\"test-model\""));
            let body = r#"{"vector":[0,100]}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).unwrap();
        });
        let config = EmbeddingClientConfig {
            url: format!("http://{addr}/embed"),
            model: Some("test-model".to_owned()),
            api_key: Some("test-key".to_owned()),
            timeout_ms: 1_000,
        };

        let vector = embed_query(&config, "semantic question").unwrap();

        assert_eq!(vector, vec![0, 100]);
        server.join().unwrap();
    }
}
