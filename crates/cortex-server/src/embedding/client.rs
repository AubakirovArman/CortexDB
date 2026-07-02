use std::time::Duration;

use cortex_engine::EmbeddingClientConfig;
use serde_json::{Map, Value};

use crate::responses::RouterError;

pub(super) fn embed_query_with_config(
    config: &EmbeddingClientConfig,
    text: &str,
) -> Result<Vec<i16>, RouterError> {
    if text.trim().is_empty() {
        return Err(RouterError::BadRequest(
            "embed_query requires non-empty query text".to_owned(),
        ));
    }
    let body = embedding_request_body(config, text)?;
    let timeout = Duration::from_millis(config.timeout_ms.max(1));
    // `ureq` with the `tls` feature speaks both http:// and https://, so the
    // client can reach TLS embedding providers (e.g. a LiteLLM proxy) instead
    // of the previous http-only raw-socket transport.
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
        .map_err(|error| RouterError::BadRequest(format!("embedding request failed: {error}")))?;
    let payload = response
        .into_string()
        .map_err(|error| RouterError::BadRequest(format!("embedding request failed: {error}")))?;
    let json: Value = serde_json::from_str(&payload).map_err(|error| {
        RouterError::BadRequest(format!("embedding provider returned invalid JSON: {error}"))
    })?;
    vector_from_embedding_json(&json)
        .filter(|vector| !vector.is_empty())
        .ok_or_else(|| {
            RouterError::BadRequest(
                "embedding provider response must contain non-empty vector".to_owned(),
            )
        })
}

pub(crate) fn format_vector_literal(vector: &[i16]) -> String {
    vector
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn embedding_request_body(
    config: &EmbeddingClientConfig,
    text: &str,
) -> Result<String, RouterError> {
    let mut body = Map::new();
    body.insert("input".to_owned(), Value::String(text.to_owned()));
    if let Some(model) = config.model.as_deref().filter(|value| !value.is_empty()) {
        body.insert("model".to_owned(), Value::String(model.to_owned()));
    }
    serde_json::to_string(&Value::Object(body)).map_err(RouterError::from)
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

    use cortex_engine::EmbeddingClientConfig;

    use super::*;

    #[test]
    fn parses_openai_compatible_embedding_json() {
        let json: serde_json::Value =
            serde_json::from_str(r#"{"data":[{"embedding":[0.0,0.5,-0.5]}]}"#).unwrap();

        let vector = vector_from_embedding_json(&json).unwrap();

        assert_eq!(vector, vec![0, 16_384, -16_384]);
    }

    #[test]
    fn embedding_client_posts_to_local_embedder() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                .unwrap();
            let mut request = String::new();
            let mut buffer = [0u8; 1024];
            // A single read can return a partial request under load; keep
            // reading until the full JSON body has arrived.
            while !request.contains("\"input\":\"semantic question\"") {
                match stream.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(read) => request.push_str(&String::from_utf8_lossy(&buffer[..read])),
                    Err(_) => break,
                }
            }
            assert!(request.contains("POST /embed HTTP/1.1"));
            assert!(request.contains("\"model\":\"test-model\""));
            assert!(request.contains("\"input\":\"semantic question\""));
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

        let vector = embed_query_with_config(&config, "semantic question").unwrap();

        assert_eq!(vector, vec![0, 100]);
        server.join().unwrap();
    }

    // Live check against a real (TLS) embedding provider. Ignored by default;
    // run with `--ignored` and CORTEXDB_EMBEDDING_URL/MODEL/API_KEY set.
    #[test]
    #[ignore = "requires network and CORTEXDB_EMBEDDING_* env vars"]
    fn live_embed_query_over_https() {
        let config = EmbeddingClientConfig {
            url: std::env::var("CORTEXDB_EMBEDDING_URL").expect("CORTEXDB_EMBEDDING_URL"),
            model: std::env::var("CORTEXDB_EMBEDDING_MODEL").ok(),
            api_key: std::env::var("CORTEXDB_EMBEDDING_API_KEY").ok(),
            timeout_ms: 30_000,
        };
        let vector = embed_query_with_config(&config, "solar plant capital budget").unwrap();
        assert!(
            vector.len() >= 256,
            "expected a real embedding vector, got len {}",
            vector.len()
        );
        assert!(
            vector.iter().any(|&value| value != 0),
            "embedding vector must not be all zeros"
        );
    }
}
