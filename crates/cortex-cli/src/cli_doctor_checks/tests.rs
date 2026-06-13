use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::*;

#[test]
fn tenant_check_rejects_path_escape_tenant() {
    let check = tenant_check(Some("../escape"), Path::new("/tmp/cortexdb"));
    assert!(!check.ok);
    assert!(check.detail.contains("invalid tenant"));
}

#[test]
fn lock_check_without_open_reports_stale_lock_command() {
    let path = unique_path("cortexdb-doctor-lock");
    fs::create_dir_all(&path).unwrap();
    fs::write(path.join("db.lock"), b"stale").unwrap();

    let check = lock_check_without_open(&path);

    assert!(!check.ok);
    assert!(check.detail.contains("lock exists"));
    assert!(check.detail.contains("unlock"));
    let _ = fs::remove_dir_all(path);
}

#[test]
fn configured_backup_root_without_backups_is_a_problem() {
    let path = unique_path("cortexdb-doctor-backup");
    let check = backup_age_check_for_roots(vec![path.clone()], true);
    assert!(!check.ok);
    assert!(check
        .detail
        .contains("configured backup root has no readable backups"));
    let _ = fs::remove_dir_all(path);
}

fn unique_path(prefix: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "{prefix}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}
