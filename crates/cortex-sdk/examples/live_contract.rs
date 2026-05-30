//! Live SDK contract smoke test.
//!
//! This example is used by `scripts/check_sdk_contract.py` to prove that the
//! Rust SDK decodes real `cortex-server` responses, not only canned JSON.
//! Set `CORTEXDB_URL` to target an already running server, or run it from the
//! repository root after building `cortex-server`.

use std::env;
use std::fs;
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use cortex_sdk::{CortexDbClient, ErrorCode, SdkError};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = ServerGuard::start_if_needed()?;
    let client = CortexDbClient::new(server.url());

    let health = client.health_response()?;
    assert_eq!(health.status, "ok");
    assert_eq!(health.version, "v1");
    assert!(!health.server_version.is_empty());
    println!("OK: health_response");

    let payload = "scope=default\nstatus=ready\ntype=fact\nsource=rust_sdk\n\nhello rust sdk";
    let put = client.put_cell_response(1, payload)?;
    assert_eq!(put.seq, 1);
    assert_eq!(put.cell_id, 1);
    println!("OK: put_cell_response");

    let lookup = client.get_cell_response(1)?;
    let cell = lookup.cell.expect("cell should exist");
    assert_eq!(cell.cell_id, 1);
    assert!(cell.payload.contains("hello rust sdk"));
    println!("OK: get_cell_response");

    let search = client.search_keyword_response("default", "rust", 10)?;
    assert_eq!(search.search_mode, "keyword");
    println!("OK: search_keyword_response");

    let stats = client.stats_response()?;
    assert!(stats.current_seq >= 1);
    println!("OK: stats_response");

    let validation = client.validate_response()?;
    assert!(validation.ok);
    println!("OK: validate_response");

    let retrieve =
        "RETRIEVE CONTEXT FOR TASK \"rust\" IN BRAIN default WHERE space = default AND status = \"ready\" LIMIT 10 CANDIDATES;";
    let aql = client.aql_response("default", retrieve)?;
    assert!(!aql.cells.is_empty());
    println!("OK: aql_response");

    let context = client.context_response("default", retrieve)?;
    assert_eq!(context.schema_version, "context_pack.v1");
    assert!(context.token_budget_tokens > 0);
    println!("OK: context_response");

    let verify = client.verify_response(
        "default",
        "VERIFY FACT \"hello rust sdk\" IN BRAIN default;",
    )?;
    assert_eq!(verify.fact, "hello rust sdk");
    println!("OK: verify_response");

    let remember = client.remember_response(
        "default",
        "REMEMBER \"rust sdk memory\" IN SCOPE default AS TYPE decision TTL 3600 SECONDS;",
    )?;
    assert!(remember.seq > 0);
    println!("OK: remember_response");

    let ingest = client.ingest_text_response("default", "rust_sdk", "hello rust sdk ingestion")?;
    assert!(ingest.chunks_ingested >= 1);
    println!("OK: ingest_text_response");

    match client.aql_response(
        "default",
        "RETRIEVE CONTEXT FOR TASK \"rust\" IN BRAIN default USING MODE turbo;",
    ) {
        Err(SdkError::CortexDb(error)) => {
            assert_eq!(error.code, ErrorCode::InvalidAql);
            println!("OK: invalid_aql_error_contract");
        }
        other => return Err(format!("expected invalid_aql error, got {other:?}").into()),
    }

    match client.ingestion_job_response(999_999) {
        Err(SdkError::CortexDb(error)) => {
            assert_eq!(error.code, ErrorCode::NotFound);
            println!("OK: not_found_error_contract");
        }
        other => return Err(format!("expected not_found error, got {other:?}").into()),
    }

    match client.clone().with_tenant("../bad").stats_response() {
        Err(SdkError::CortexDb(error)) => {
            assert_eq!(error.code, ErrorCode::InvalidTenant);
            println!("OK: invalid_tenant_error_contract");
        }
        other => return Err(format!("expected invalid_tenant error, got {other:?}").into()),
    }

    println!("\nAll Rust SDK live contract checks passed.");
    Ok(())
}

struct ServerGuard {
    url: String,
    child: Option<Child>,
    db_dir: Option<PathBuf>,
}

impl ServerGuard {
    fn start_if_needed() -> Result<Self, Box<dyn std::error::Error>> {
        if let Ok(url) = env::var("CORTEXDB_URL") {
            return Ok(Self {
                url,
                child: None,
                db_dir: None,
            });
        }

        let repo = repo_root()?;
        let binary = server_binary(&repo)?;
        let port = free_port()?;
        let db_dir = temp_db_dir()?;
        fs::create_dir_all(&db_dir)?;

        let child = Command::new(binary)
            .arg(&db_dir)
            .arg(format!("127.0.0.1:{port}"))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        wait_for_server(port, Duration::from_secs(10))?;

        Ok(Self {
            url: format!("http://127.0.0.1:{port}"),
            child: Some(child),
            db_dir: Some(db_dir),
        })
    }

    fn url(&self) -> &str {
        &self.url
    }
}

impl Drop for ServerGuard {
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = child.kill();
            let _ = child.wait();
        }
        if let Some(db_dir) = &self.db_dir {
            let _ = fs::remove_dir_all(db_dir);
        }
    }
}

fn repo_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut path = env::current_dir()?;
    loop {
        if path.join("Cargo.toml").exists() && path.join("crates/cortex-server").exists() {
            return Ok(path);
        }
        if !path.pop() {
            return Err("could not find CortexDB repository root".into());
        }
    }
}

fn server_binary(repo: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Ok(binary) = env::var("CORTEXDB_SERVER_BIN") {
        return Ok(PathBuf::from(binary));
    }
    let release = repo.join("target/release/cortex-server");
    if release.exists() {
        return Ok(release);
    }
    let debug = repo.join("target/debug/cortex-server");
    if debug.exists() {
        return Ok(debug);
    }
    Err("cortex-server binary not found; run `cargo build -p cortex-server` first".into())
}

fn free_port() -> Result<u16, Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    Ok(listener.local_addr()?.port())
}

fn temp_db_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    Ok(env::temp_dir().join(format!(
        "cortex_rust_sdk_smoke_{}_{}",
        std::process::id(),
        nanos
    )))
}

fn wait_for_server(port: u16, timeout: Duration) -> Result<(), Box<dyn std::error::Error>> {
    let deadline = SystemTime::now() + timeout;
    while SystemTime::now() < deadline {
        if TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(100));
    }
    Err(format!("cortex-server did not start on port {port}").into())
}
