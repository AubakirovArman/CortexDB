use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
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
    let endpoint = HttpEndpoint::parse(&config.url).map_err(|error| {
        RouterError::BadRequest(format!("invalid CORTEXDB_EMBEDDING_URL: {error}"))
    })?;
    let timeout = Duration::from_millis(config.timeout_ms.max(1));
    let mut stream = connect(&endpoint, timeout)
        .map_err(|error| RouterError::BadRequest(format!("embedding request failed: {error}")))?;
    stream
        .set_read_timeout(Some(timeout))
        .map_err(RouterError::from)?;
    stream
        .set_write_timeout(Some(timeout))
        .map_err(RouterError::from)?;

    let body = embedding_request_body(config, text)?;
    let mut request = format!(
        "POST {} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nAccept: application/json\r\nContent-Length: {}\r\nConnection: close\r\n",
        endpoint.path,
        endpoint.host_header,
        body.len()
    );
    if let Some(api_key) = config.api_key.as_deref().filter(|value| !value.is_empty()) {
        request.push_str("Authorization: Bearer ");
        request.push_str(api_key);
        request.push_str("\r\n");
    }
    request.push_str("\r\n");
    request.push_str(&body);

    stream
        .write_all(request.as_bytes())
        .map_err(|error| RouterError::BadRequest(format!("embedding request failed: {error}")))?;
    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .map_err(|error| RouterError::BadRequest(format!("embedding request failed: {error}")))?;
    parse_embedding_http_response(&response)
}

pub(crate) fn format_vector_literal(vector: &[i16]) -> String {
    vector
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn connect(endpoint: &HttpEndpoint, timeout: Duration) -> std::io::Result<TcpStream> {
    let mut addrs = endpoint.connect_addr.to_socket_addrs()?;
    let Some(addr) = addrs.next() else {
        return Err(std::io::Error::other(
            "embedding host resolved to no addresses",
        ));
    };
    TcpStream::connect_timeout(&addr, timeout)
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

fn parse_embedding_http_response(response: &[u8]) -> Result<Vec<i16>, RouterError> {
    let text = String::from_utf8_lossy(response);
    let (head, body) = text.split_once("\r\n\r\n").ok_or_else(|| {
        RouterError::BadRequest("embedding provider returned invalid HTTP response".to_owned())
    })?;
    let status_line = head.lines().next().unwrap_or_default();
    let status = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(0);
    if !(200..300).contains(&status) {
        return Err(RouterError::BadRequest(format!(
            "embedding provider returned HTTP {status}"
        )));
    }
    let json: Value = serde_json::from_str(body.trim()).map_err(|error| {
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

struct HttpEndpoint {
    host_header: String,
    connect_addr: String,
    path: String,
}

impl HttpEndpoint {
    fn parse(url: &str) -> Result<Self, &'static str> {
        let rest = url
            .strip_prefix("http://")
            .ok_or("only http:// embedding URLs are supported")?;
        let (authority, path) = match rest.split_once('/') {
            Some((authority, path)) => (authority, format!("/{path}")),
            None => (rest, "/".to_owned()),
        };
        if authority.trim().is_empty() {
            return Err("host is required");
        }
        let connect_addr = if authority.rsplit_once(':').is_some() {
            authority.to_owned()
        } else {
            format!("{authority}:80")
        };
        Ok(Self {
            host_header: authority.to_owned(),
            connect_addr,
            path,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    use cortex_engine::EmbeddingClientConfig;

    use super::*;

    #[test]
    fn parses_openai_compatible_embedding_response() {
        let body =
            b"HTTP/1.1 200 OK\r\nContent-Length: 42\r\n\r\n{\"data\":[{\"embedding\":[0.0,0.5,-0.5]}]}";

        let vector = parse_embedding_http_response(body).unwrap();

        assert_eq!(vector, vec![0, 16_384, -16_384]);
    }

    #[test]
    fn embedding_client_posts_to_local_embedder() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 2048];
            let read = stream.read(&mut request).unwrap();
            let request = String::from_utf8_lossy(&request[..read]);
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
}
