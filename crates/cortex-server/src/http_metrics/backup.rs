use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub(super) fn backup_latest_age_seconds(root: &Path) -> i64 {
    let mut roots = Vec::new();
    if let Ok(value) = std::env::var("CORTEXDB_BACKUP_ROOT") {
        if !value.trim().is_empty() {
            roots.push(PathBuf::from(value.trim()));
        }
    }
    if roots.is_empty() {
        roots.push(root.join("backups"));
        roots.push(root.join("backup"));
    }

    let latest = roots.iter().filter_map(|root| latest_modified(root)).max();
    latest
        .and_then(|modified| SystemTime::now().duration_since(modified).ok())
        .map_or(-1, |age| age.as_secs() as i64)
}

fn latest_modified(path: &Path) -> Option<SystemTime> {
    let mut latest = fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok();
    let entries = fs::read_dir(path).ok()?;
    for entry in entries.flatten() {
        let child_modified = fs::metadata(entry.path())
            .and_then(|metadata| metadata.modified())
            .ok();
        latest = latest.max(child_modified);
    }
    latest
}
