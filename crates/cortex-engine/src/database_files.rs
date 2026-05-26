use std::path::{Path, PathBuf};

use crate::error::EngineResult;

pub(crate) fn truncate_wal_tail(path: &Path, safe_offset: u64) -> EngineResult<()> {
    let exists = path.exists();
    match std::fs::metadata(path) {
        Ok(metadata) if metadata.len() > safe_offset => {
            std::fs::OpenOptions::new()
                .write(true)
                .open(path)?
                .set_len(safe_offset)?;
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    if safe_offset == 0 && exists {
        remove_rotated_wals(path);
    }
    Ok(())
}

pub(crate) fn find_wal_files(active_wal: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let parent = active_wal.parent().unwrap_or_else(|| Path::new("."));
    if let Ok(entries) = std::fs::read_dir(parent) {
        for entry in entries.flatten() {
            let path = entry.path();
            if is_rotated_wal(&path) {
                files.push(path);
            }
        }
    }
    files.sort();
    files.push(active_wal.to_owned());
    files
}

fn remove_rotated_wals(active_wal: &Path) {
    let parent = active_wal.parent().unwrap_or_else(|| Path::new("."));
    if let Ok(entries) = std::fs::read_dir(parent) {
        for entry in entries.flatten() {
            let path = entry.path();
            if is_rotated_wal(&path) {
                let _ = std::fs::remove_file(path);
            }
        }
    }
}

fn is_rotated_wal(path: &Path) -> bool {
    path.is_file()
        && path
            .file_name()
            .and_then(|f| f.to_str())
            .is_some_and(|filename| {
                filename.starts_with("db.")
                    && filename.ends_with(".aclog")
                    && filename != "db.aclog"
            })
}
