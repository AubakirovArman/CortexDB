use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;

use cortex_core::CellId;
use cortex_engine::Database;

pub fn serve(root: &Path, addr: &str) -> std::io::Result<()> {
    let listener = TcpListener::bind(addr)?;
    for stream in listener.incoming() {
        handle_stream(root, stream?)?;
    }
    Ok(())
}

pub fn handle_http(root: &Path, request: &str) -> String {
    let Some((head, body)) = request.split_once("\r\n\r\n") else {
        return response(400, "bad request");
    };
    let Some(first_line) = head.lines().next() else {
        return response(400, "bad request");
    };
    let parts = first_line.split_whitespace().collect::<Vec<_>>();
    if parts.len() != 3 {
        return response(400, "bad request");
    }
    match route(root, parts[0], parts[1], body.as_bytes()) {
        Ok(value) => response(200, &value),
        Err(error) => response(400, &error),
    }
}

fn handle_stream(root: &Path, mut stream: TcpStream) -> std::io::Result<()> {
    let mut buffer = vec![0; 8192];
    let read = stream.read(&mut buffer)?;
    let request = String::from_utf8_lossy(&buffer[..read]);
    stream.write_all(handle_http(root, &request).as_bytes())
}

fn route(root: &Path, method: &str, target: &str, body: &[u8]) -> Result<String, String> {
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    match (method, path) {
        ("GET", "/get") => {
            let cell_id = cell_id(query)?;
            let db = Database::open(root).map_err(|error| error.to_string())?;
            Ok(db
                .get_latest_cell(cell_id)
                .map(|payload| String::from_utf8_lossy(&payload).into_owned())
                .unwrap_or_else(|| "null".to_owned()))
        }
        ("POST", "/put") => {
            let cell_id = cell_id(query)?;
            let mut db = Database::open(root).map_err(|error| error.to_string())?;
            let seq = db
                .put_cell(cell_id, body.to_vec())
                .map_err(|error| error.to_string())?;
            Ok(format!("seq={}", seq.0))
        }
        ("POST", "/tombstone") => {
            let cell_id = cell_id(query)?;
            let mut db = Database::open(root).map_err(|error| error.to_string())?;
            let seq = db
                .tombstone_cell(cell_id)
                .map_err(|error| error.to_string())?;
            Ok(format!("seq={}", seq.0))
        }
        ("POST", "/flush") => {
            let mut db = Database::open(root).map_err(|error| error.to_string())?;
            let stats = db.checkpoint().map_err(|error| error.to_string())?;
            Ok(format!(
                "checkpoint_seq={} cells_flushed={}",
                stats.checkpoint_seq.0, stats.cells_flushed
            ))
        }
        _ => Err("unknown route".to_owned()),
    }
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

fn response(status: u16, body: &str) -> String {
    let reason = if status == 200 { "OK" } else { "Bad Request" };
    format!(
        "HTTP/1.1 {status} {reason}\r\ncontent-type: text/plain\r\ncontent-length: {}\r\n\r\n{body}",
        body.len()
    )
}

#[cfg(test)]
mod tests {
    use super::handle_http;

    #[test]
    fn put_get_and_flush_over_http() {
        let dir = tempfile::tempdir().unwrap();
        let put = "POST /put?cell_id=1 HTTP/1.1\r\ncontent-length: 5\r\n\r\nhello";
        assert!(handle_http(dir.path(), put).contains("seq=1"));
        let get = "GET /get?cell_id=1 HTTP/1.1\r\n\r\n";
        assert!(handle_http(dir.path(), get).ends_with("hello"));
        let flush = "POST /flush HTTP/1.1\r\ncontent-length: 0\r\n\r\n";
        assert!(handle_http(dir.path(), flush).contains("cells_flushed=1"));
    }
}
