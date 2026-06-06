use std::env;
use std::fs;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

pub(crate) struct DoctorCheck {
    pub(crate) name: &'static str,
    pub(crate) ok: bool,
    pub(crate) detail: String,
}

impl DoctorCheck {
    pub(crate) fn ok(name: &'static str, detail: impl Into<String>) -> Self {
        Self {
            name,
            ok: true,
            detail: detail.into(),
        }
    }

    pub(crate) fn fail(name: &'static str, detail: impl Into<String>) -> Self {
        Self {
            name,
            ok: false,
            detail: detail.into(),
        }
    }
}

pub(crate) fn validate_tenant_id(tenant: &str) -> bool {
    if tenant.is_empty() || tenant.len() > 64 {
        return false;
    }
    tenant
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

pub(crate) fn tenant_check(tenant: Option<&str>, db_path: &Path) -> DoctorCheck {
    match tenant {
        Some(value) if validate_tenant_id(value) => DoctorCheck::ok(
            "tenant",
            format!("tenant={value} realm_path={}", db_path.display()),
        ),
        Some(value) => DoctorCheck::fail(
            "tenant",
            format!("invalid tenant={value}; allowed: ASCII alphanumeric, '_' and '-'"),
        ),
        None => DoctorCheck::ok("tenant", "tenant=default"),
    }
}

pub(crate) fn lock_check_after_open(path: &Path) -> DoctorCheck {
    let lock_path = path.join("db.lock");
    match fs::read_to_string(&lock_path) {
        Ok(value) if value.contains("format=cortexdb-lock-v1") => DoctorCheck::ok(
            "db_lock",
            format!("lock acquired at {}", lock_path.display()),
        ),
        Ok(_) => DoctorCheck::fail(
            "db_lock",
            format!("lock file has unknown format at {}", lock_path.display()),
        ),
        Err(error) => DoctorCheck::fail(
            "db_lock",
            format!(
                "lock file missing after open at {}: {error}",
                lock_path.display()
            ),
        ),
    }
}

pub(crate) fn lock_check_without_open(path: &Path) -> DoctorCheck {
    let lock_path = path.join("db.lock");
    if lock_path.exists() {
        DoctorCheck::fail(
            "db_lock",
            format!(
                "lock exists at {}; if stale, run cortexdb unlock {} --force",
                lock_path.display(),
                path.display()
            ),
        )
    } else {
        DoctorCheck::fail("db_lock", "database did not open; lock state unavailable")
    }
}

pub(crate) fn backup_age_check(path: &Path) -> DoctorCheck {
    let (roots, configured) = backup_roots(path);
    let mut latest: Option<SystemTime> = None;
    let mut existing_roots = Vec::new();
    for root in &roots {
        if !root.exists() {
            continue;
        }
        existing_roots.push(root.display().to_string());
        latest = latest.max(latest_modified(root));
    }

    match latest {
        Some(modified) => {
            let age = SystemTime::now()
                .duration_since(modified)
                .unwrap_or(Duration::ZERO)
                .as_secs();
            DoctorCheck::ok(
                "backup_age",
                format!(
                    "latest_backup_age_seconds={age} roots={}",
                    existing_roots.join(",")
                ),
            )
        }
        None if configured => DoctorCheck::fail(
            "backup_age",
            format!(
                "configured backup root has no readable backups: {}",
                roots
                    .iter()
                    .map(|root| root.display().to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            ),
        ),
        None => DoctorCheck::ok(
            "backup_age",
            "no backup root configured; set CORTEXDB_BACKUP_ROOT or run cortexdb backup-drill",
        ),
    }
}

pub(crate) fn server_health_check() -> DoctorCheck {
    let Some(raw) = env::var("CORTEXDB_SERVER_URL")
        .ok()
        .or_else(|| env::var("CORTEXDB_SERVER_ADDR").ok())
    else {
        return DoctorCheck::ok(
            "server_health",
            "no CORTEXDB_SERVER_URL/CORTEXDB_SERVER_ADDR configured; local checks only",
        );
    };

    let Some(addr) = parse_server_addr(&raw) else {
        return DoctorCheck::fail(
            "server_health",
            format!("cannot parse server endpoint: {raw}"),
        );
    };
    match TcpStream::connect_timeout(&addr, Duration::from_millis(500)) {
        Ok(_) => DoctorCheck::ok("server_health", format!("tcp_connect_ok={addr}")),
        Err(error) => DoctorCheck::fail(
            "server_health",
            format!("tcp_connect_failed={addr}: {error}"),
        ),
    }
}

pub(crate) fn auth_check() -> DoctorCheck {
    let inline = env_present("CORTEXDB_AUTH_TOKEN") || env_present("CORTEXDB_AUTH_TOKENS");
    let token_file = env_file_status("CORTEXDB_AUTH_TOKENS_FILE");
    let policy_file = env_file_status("CORTEXDB_AUTH_POLICY_STORE_FILE");

    if let Some(error) = token_file.as_ref().and_then(|status| status.err.as_ref()) {
        return DoctorCheck::fail("auth", format!("CORTEXDB_AUTH_TOKENS_FILE {error}"));
    }
    if let Some(error) = policy_file.as_ref().and_then(|status| status.err.as_ref()) {
        return DoctorCheck::fail("auth", format!("CORTEXDB_AUTH_POLICY_STORE_FILE {error}"));
    }

    let mut sources = Vec::new();
    if inline {
        sources.push("inline_env");
    }
    if token_file.is_some() {
        sources.push("tokens_file");
    }
    if policy_file.is_some() {
        sources.push("policy_store_file");
    }
    if sources.is_empty() {
        DoctorCheck::ok(
            "auth",
            "auth env not configured; verify server startup policy separately",
        )
    } else {
        DoctorCheck::ok("auth", format!("configured_sources={}", sources.join(",")))
    }
}

pub(crate) fn repair_advice(all_ok: bool, path: &Path) -> DoctorCheck {
    if all_ok {
        DoctorCheck::ok(
            "repair_advice",
            format!(
                "no repair needed; after failures run cortexdb repair --dry-run {}",
                path.display()
            ),
        )
    } else {
        DoctorCheck::ok(
            "repair_advice",
            format!(
                "run cortexdb repair --dry-run {}; use cortexdb unlock {} --force only for stale locks",
                path.display(),
                path.display()
            ),
        )
    }
}

fn backup_roots(path: &Path) -> (Vec<PathBuf>, bool) {
    match env::var("CORTEXDB_BACKUP_ROOT") {
        Ok(value) if !value.trim().is_empty() => (vec![PathBuf::from(value)], true),
        _ => (vec![path.join("backups"), path.join("backup")], false),
    }
}

fn latest_modified(path: &Path) -> Option<SystemTime> {
    let mut latest = fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok();
    let entries = fs::read_dir(path).ok()?;
    for entry in entries.flatten() {
        let child = entry.path();
        let child_modified = fs::metadata(&child)
            .and_then(|metadata| metadata.modified())
            .ok();
        latest = latest.max(child_modified);
    }
    latest
}

fn parse_server_addr(raw: &str) -> Option<SocketAddr> {
    let host_port = raw
        .strip_prefix("http://")
        .or_else(|| raw.strip_prefix("https://"))
        .unwrap_or(raw)
        .split('/')
        .next()?;
    host_port.to_socket_addrs().ok()?.next()
}

struct EnvFileStatus {
    err: Option<String>,
}

fn env_present(name: &str) -> bool {
    env::var(name).is_ok_and(|value| !value.trim().is_empty())
}

fn env_file_status(name: &str) -> Option<EnvFileStatus> {
    let value = env::var(name).ok()?;
    let path = PathBuf::from(value.trim());
    if value.trim().is_empty() {
        return Some(EnvFileStatus {
            err: Some("is empty".to_owned()),
        });
    }
    match fs::metadata(&path) {
        Ok(metadata) if metadata.len() > 0 => Some(EnvFileStatus { err: None }),
        Ok(_) => Some(EnvFileStatus {
            err: Some(format!("is empty at {}", path.display())),
        }),
        Err(error) => Some(EnvFileStatus {
            err: Some(format!("is unreadable at {}: {error}", path.display())),
        }),
    }
}
