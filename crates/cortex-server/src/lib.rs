use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;

use cortex_core::CellId;
use cortex_engine::Database;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ServerOptions {
    pub auth_token: Option<String>,
}

pub fn serve(root: &Path, addr: &str) -> std::io::Result<()> {
    serve_with_options(root, addr, ServerOptions::default())
}

pub fn serve_with_options(root: &Path, addr: &str, options: ServerOptions) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr)?;
    for stream in listener.incoming() {
        handle_stream(root, &options, stream?)?;
    }
    Ok(())
}

pub fn handle_http(root: &Path, request: &str) -> String {
    handle_http_with_options(root, request, &ServerOptions::default())
}

pub fn handle_http_with_options(root: &Path, request: &str, options: &ServerOptions) -> String {
    let Some((head, body)) = request.split_once("\r\n\r\n") else {
        return json_error(400, "bad_request", "bad request");
    };
    let Some(first_line) = head.lines().next() else {
        return json_error(400, "bad_request", "bad request");
    };
    let parts = first_line.split_whitespace().collect::<Vec<_>>();
    if parts.len() != 3 {
        return json_error(400, "bad_request", "bad request");
    }
    if !authorized(head, options) {
        return json_error(401, "unauthorized", "missing or invalid authorization");
    }
    match route(root, parts[0], parts[1], body.as_bytes()) {
        Ok(value) => json_response(200, &value),
        Err(error) => json_error(400, "bad_request", &error),
    }
}

fn handle_stream(
    root: &Path,
    options: &ServerOptions,
    mut stream: TcpStream,
) -> std::io::Result<()> {
    let mut buffer = vec![0; 8192];
    let read = stream.read(&mut buffer)?;
    let request = String::from_utf8_lossy(&buffer[..read]);
    stream.write_all(handle_http_with_options(root, &request, options).as_bytes())
}

fn route(root: &Path, method: &str, target: &str, body: &[u8]) -> Result<String, String> {
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    match (method, path) {
        ("GET", "/v1/health") => Ok(r#"{"status":"ok","version":"v1"}"#.to_owned()),
        ("GET", "/get") | ("GET", "/v1/cell") => {
            let cell_id = cell_id(query)?;
            let db = Database::open(root).map_err(|error| error.to_string())?;
            let value = db.get_latest_cell(cell_id).map(|payload| {
                format!(
                    r#"{{"cell_id":{},"payload":"{}"}}"#,
                    cell_id.0,
                    escape_json(&String::from_utf8_lossy(&payload))
                )
            });
            Ok(value.unwrap_or_else(|| r#"{"cell":null}"#.to_owned()))
        }
        ("POST", "/put") | ("POST", "/v1/cell") => {
            let cell_id = cell_id(query)?;
            let mut db = Database::open(root).map_err(|error| error.to_string())?;
            let seq = db
                .put_cell(cell_id, body.to_vec())
                .map_err(|error| error.to_string())?;
            Ok(format!(r#"{{"seq":{},"cell_id":{}}}"#, seq.0, cell_id.0))
        }
        ("POST", "/tombstone") | ("DELETE", "/v1/cell") => {
            let cell_id = cell_id(query)?;
            let mut db = Database::open(root).map_err(|error| error.to_string())?;
            let seq = db
                .tombstone_cell(cell_id)
                .map_err(|error| error.to_string())?;
            Ok(format!(r#"{{"seq":{},"cell_id":{}}}"#, seq.0, cell_id.0))
        }
        ("POST", "/flush") | ("POST", "/v1/flush") => {
            let mut db = Database::open(root).map_err(|error| error.to_string())?;
            let stats = db.checkpoint().map_err(|error| error.to_string())?;
            Ok(format!(
                r#"{{"checkpoint_seq":{},"cells_flushed":{}}}"#,
                stats.checkpoint_seq.0, stats.cells_flushed
            ))
        }
        ("POST", "/v1/compact") => {
            let mut db = Database::open(root).map_err(|error| error.to_string())?;
            let stats = db.compact().map_err(|error| error.to_string())?;
            Ok(format!(
                r#"{{"checkpoint_seq":{},"cells_flushed":{}}}"#,
                stats.checkpoint_seq.0, stats.cells_flushed
            ))
        }
        _ => Err("unknown route".to_owned()),
    }
}

fn authorized(head: &str, options: &ServerOptions) -> bool {
    let Some(token) = &options.auth_token else {
        return true;
    };
    head.lines().skip(1).any(|line| {
        line.strip_prefix("authorization:")
            .or_else(|| line.strip_prefix("Authorization:"))
            .is_some_and(|value| value.trim() == format!("Bearer {token}"))
    })
}

fn cell_id(query: &str) -> Result<CellId, String> {
    query
        .split('&')
        .find_map(|pair| pair.strip_prefix("cell_id="))
        .ok_or_else(|| "missing cell_id".to_owned())?
        .parse::<u64>()
        .map(CellId)
        .map_err(|_| "cell_id must be u64".to_owned())
}

fn json_response(status: u16, body: &str) -> String {
    let reason = reason(status);
    format!(
        "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{body}",
        body.len()
    )
}

fn json_error(status: u16, code: &str, message: &str) -> String {
    json_response(
        status,
        &format!(
            r#"{{"error":{{"code":"{}","message":"{}"}}}}"#,
            escape_json(code),
            escape_json(message)
        ),
    )
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        401 => "Unauthorized",
        _ => "Bad Request",
    }
}

fn escape_json(value: &str) -> String {
    value
        .chars()
        .flat_map(|character| match character {
            '"' => "\\\"".chars().collect::<Vec<_>>(),
            '\\' => "\\\\".chars().collect(),
            '\n' => "\\n".chars().collect(),
            '\r' => "\\r".chars().collect(),
            '\t' => "\\t".chars().collect(),
            other => vec![other],
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{handle_http, handle_http_with_options, ServerOptions};

    #[test]
    fn put_get_and_flush_over_http() {
        let dir = tempfile::tempdir().unwrap();
        let put = "POST /put?cell_id=1 HTTP/1.1\r\ncontent-length: 5\r\n\r\nhello";
        assert!(handle_http(dir.path(), put).contains(r#""seq":1"#));
        let get = "GET /get?cell_id=1 HTTP/1.1\r\n\r\n";
        assert!(handle_http(dir.path(), get).contains(r#""payload":"hello""#));
        let flush = "POST /flush HTTP/1.1\r\ncontent-length: 0\r\n\r\n";
        assert!(handle_http(dir.path(), flush).contains(r#""cells_flushed":1"#));
    }

    #[test]
    fn v1_api_requires_bearer_token_when_configured() {
        let dir = tempfile::tempdir().unwrap();
        let options = ServerOptions {
            auth_token: Some("secret".to_owned()),
        };
        let denied =
            handle_http_with_options(dir.path(), "GET /v1/health HTTP/1.1\r\n\r\n", &options);
        assert!(denied.contains("401 Unauthorized"));

        let allowed = handle_http_with_options(
            dir.path(),
            "GET /v1/health HTTP/1.1\r\nAuthorization: Bearer secret\r\n\r\n",
            &options,
        );
        assert!(allowed.contains(r#""status":"ok""#));
    }
}
