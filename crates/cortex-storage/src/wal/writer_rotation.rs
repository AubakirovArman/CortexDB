use std::fs::{File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use super::codec::WalCodec;

pub(super) fn rotate_if_needed(
    path: &Path,
    max_size: Option<u64>,
    file_opt: &mut Option<File>,
    next_lsn: &mut u64,
) {
    let Some(max_size) = max_size else {
        return;
    };
    if !needs_rotation(file_opt.as_ref(), max_size) {
        return;
    }
    let _ = rotate_now(path, file_opt, next_lsn);
}

/// Rotate the active WAL to a timestamped archive and open a fresh active file.
/// Returns the path of the archived file on success.
pub(super) fn rotate_now(
    path: &Path,
    file_opt: &mut Option<File>,
    next_lsn: &mut u64,
) -> Option<PathBuf> {
    if let Some(file) = file_opt.take() {
        let _ = file.sync_data();
    }
    let rotated_path = path.with_file_name(format!("db.{}.aclog", timestamp_micros()));
    if std::fs::rename(path, &rotated_path).is_err() {
        return None;
    }
    if let Ok(mut new_file) = OpenOptions::new().create(true).append(true).open(path) {
        if new_file.metadata().map_or(true, |m| m.len() == 0) {
            let _ = new_file.write_all(&WalCodec::file_header());
            let _ = new_file.sync_data();
        }
        *next_lsn = new_file.seek(SeekFrom::End(0)).unwrap_or(0);
        *file_opt = Some(new_file);
        Some(rotated_path)
    } else {
        None
    }
}

fn needs_rotation(file: Option<&File>, max_size: u64) -> bool {
    file.and_then(|file| file.metadata().ok())
        .is_some_and(|metadata| metadata.len() >= max_size)
}

fn timestamp_micros() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros()
}
