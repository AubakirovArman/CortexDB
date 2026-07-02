use super::{handle_http, handle_http_with_options, ServerOptions};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Mutex, MutexGuard};
use std::thread;

static EMBEDDING_ENV_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn v1_ann_evaluate_reports_recall_for_checkpointed_vectors() {
    let dir = tempfile::tempdir().unwrap();
    put_vector_hnsw(dir.path(), 1, "project:investments", "10,0");
    put_vector_hnsw(dir.path(), 2, "project:investments", "0,10");
    put_vector_hnsw(dir.path(), 3, "tenant:private", "0,11");
    assert!(handle_hnsw(dir.path(), "POST /v1/flush HTTP/1.1\r\n\r\n")
        .contains(r#""checkpoint_seq":3"#));

    let request = "POST /v1/search/ann-evaluate?scope=project:investments&vector=0,10&limit=2&fallback=false&min_recall=1.0&require_slo=true&no_fallback_rollout=true&no_fallback_min_recall=1.0 HTTP/1.1\r\n\r\n";
    let response = handle_hnsw(dir.path(), request);

    assert!(response.contains(r#""available":true"#));
    assert!(response.contains(r#""recall_q16":65535"#));
    assert!(response.contains(r#""min_recall_q16":65535"#));
    assert!(response.contains(r#""require_slo":true"#));
    assert!(response.contains(r#""production_safe":true"#));
    assert!(response.contains(r#""no_fallback_decision":{"allowed":true,"reasons":[]}"#));
    assert!(response.contains(r#""exact_top_k":[2,1]"#));
    assert!(response.contains(r#""ann_top_k":[2,1]"#));
}

#[test]
fn v1_ann_evaluate_reports_unavailable_before_checkpoint() {
    let dir = tempfile::tempdir().unwrap();
    put_vector(dir.path(), 1, "project:investments", "10,0");

    let request =
        "POST /v1/search/ann-evaluate?scope=project:investments&vector=10,0 HTTP/1.1\r\n\r\n";
    let response = handle_http(dir.path(), request);

    assert!(response.contains(r#""available":false"#));
    assert!(response.contains("requires_persisted_checkpoint_without_wal_tail"));
}

#[test]
fn context_semantic_without_vector_or_embedding_config_errors() {
    let _env = EmbeddingEnvGuard::without_config();
    let dir = tempfile::tempdir().unwrap();
    let request = concat!(
        "POST /v1/context?scope=project:embed HTTP/1.1\r\n\r\n",
        "RETRIEVE CONTEXT FOR TASK \"semantic lookup\" IN BRAIN default ",
        "USING MODE semantic LIMIT 2 CANDIDATES;"
    );

    let response = handle_http(dir.path(), request);

    assert!(response.contains(r#""code":"bad_request""#));
    assert!(response.contains("semantic requires vector or embedding config"));
}

#[test]
fn context_embed_query_uses_local_embedder() {
    let dir = tempfile::tempdir().unwrap();
    put_vector(dir.path(), 1, "project:embed", "100,0");
    put_vector(dir.path(), 2, "project:embed", "0,100");
    let (url, server) = spawn_embedder(r#"{"data":[{"embedding":[0,100]}]}"#);
    let _env = EmbeddingEnvGuard::with_url(&url);
    let request = concat!(
        "POST /v1/context?scope=project:embed HTTP/1.1\r\n",
        "Content-Type: application/json\r\n\r\n",
        "{\"retrieve_aql\":\"RETRIEVE CONTEXT FOR TASK \\\"semantic lookup\\\" ",
        "IN BRAIN default USING MODE semantic LIMIT 2 CANDIDATES;\",",
        "\"embed_query\":true}"
    );

    let response = handle_http(dir.path(), request);

    let first = response.find(r#""cell_id":2"#).unwrap();
    let second = response.find(r#""cell_id":1"#).unwrap();
    assert!(first < second, "{response}");
    server.join().unwrap();
}

#[test]
fn search_embed_query_uses_local_embedder() {
    let dir = tempfile::tempdir().unwrap();
    put_vector(dir.path(), 1, "project:embed", "100,0");
    put_vector(dir.path(), 2, "project:embed", "0,100");
    let (url, server) = spawn_embedder(r#"{"vector":[0,100]}"#);
    let _env = EmbeddingEnvGuard::with_url(&url);
    let request = concat!(
        "POST /v1/search?scope=project:embed&mode=vector&algorithm=exact",
        "&q=semantic%20lookup&embed_query=true&limit=2 HTTP/1.1\r\n\r\n"
    );

    let response = handle_http(dir.path(), request);

    let first = response.find(r#""cell_id":2"#).unwrap();
    let second = response.find(r#""cell_id":1"#).unwrap();
    assert!(first < second, "{response}");
    server.join().unwrap();
}

fn put_vector(path: &std::path::Path, cell_id: u64, scope: &str, vector: &str) {
    let request = format!(
        "POST /v1/cell?cell_id={cell_id} HTTP/1.1\r\n\r\nscope={scope}\nstatus=ready\nvector={vector}\n\nbody"
    );
    assert!(handle_http(path, &request).contains(&format!(r#""cell_id":{cell_id}"#)));
}

fn put_vector_hnsw(path: &std::path::Path, cell_id: u64, scope: &str, vector: &str) {
    let request = format!(
        "POST /v1/cell?cell_id={cell_id} HTTP/1.1\r\n\r\nscope={scope}\nstatus=ready\nvector={vector}\n\nbody"
    );
    assert!(handle_hnsw(path, &request).contains(&format!(r#""cell_id":{cell_id}"#)));
}

fn handle_hnsw(path: &std::path::Path, request: &str) -> String {
    handle_http_with_options(path, request, &hnsw_options())
}

struct EmbeddingEnvGuard {
    _guard: MutexGuard<'static, ()>,
}

impl EmbeddingEnvGuard {
    fn without_config() -> Self {
        let guard = EMBEDDING_ENV_LOCK.lock().unwrap();
        clear_embedding_env();
        Self { _guard: guard }
    }

    fn with_url(url: &str) -> Self {
        let guard = EMBEDDING_ENV_LOCK.lock().unwrap();
        clear_embedding_env();
        std::env::set_var("CORTEXDB_EMBEDDING_URL", url);
        std::env::set_var("CORTEXDB_EMBEDDING_MODEL", "test-model");
        std::env::set_var("CORTEXDB_EMBEDDING_API_KEY", "test-key");
        std::env::set_var("CORTEXDB_EMBEDDING_TIMEOUT_MS", "1000");
        Self { _guard: guard }
    }
}

impl Drop for EmbeddingEnvGuard {
    fn drop(&mut self) {
        clear_embedding_env();
    }
}

fn clear_embedding_env() {
    std::env::remove_var("CORTEXDB_EMBEDDING_URL");
    std::env::remove_var("CORTEXDB_EMBEDDING_MODEL");
    std::env::remove_var("CORTEXDB_EMBEDDING_API_KEY");
    std::env::remove_var("CORTEXDB_EMBEDDING_TIMEOUT_MS");
}

fn spawn_embedder(body: &'static str) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(2)))
            .unwrap();
        let mut request = String::new();
        let mut buffer = [0u8; 1024];
        // A single read can return a partial request under load; read until the
        // full JSON body (closing brace) has arrived.
        while !request.contains('}') {
            match stream.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => request.push_str(&String::from_utf8_lossy(&buffer[..read])),
                Err(_) => break,
            }
        }
        assert!(request.contains("POST /embed HTTP/1.1"));
        assert!(request.contains("\"model\":\"test-model\""));
        assert!(request.contains("\"input\":\"semantic lookup\""));
        assert!(request.contains("Authorization: Bearer test-key"));
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        stream.write_all(response.as_bytes()).unwrap();
    });
    (format!("http://{addr}/embed"), handle)
}

fn hnsw_options() -> ServerOptions {
    ServerOptions {
        engine_database_options: cortex_engine::DatabaseOptions {
            feature_flags: cortex_engine::EngineFeatureFlags::production_safe()
                .with_experimental_hnsw(true),
            ..cortex_engine::DatabaseOptions::default()
        },
        ..ServerOptions::default()
    }
}
