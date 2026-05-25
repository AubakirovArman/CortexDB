use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::StorageResult;
use crate::wal::checksum::crc32c;

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(1);

pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> StorageResult<()> {
    let tmp = temp_path(path);
    {
        let mut file = File::create(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    if let Some(parent) = path.parent() {
        OpenOptions::new().read(true).open(parent)?.sync_all()?;
    }
    let legacy = legacy_temp_path(path);
    if legacy != tmp {
        let _ = fs::remove_file(legacy);
    }
    Ok(())
}

fn temp_path(path: &Path) -> std::path::PathBuf {
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("tmp");
    path.with_file_name(format!(
        "{file_name}.tmp.{}.{}",
        std::process::id(),
        counter
    ))
}

fn legacy_temp_path(path: &Path) -> std::path::PathBuf {
    path.with_extension(format!(
        "{}.tmp",
        path.extension()
            .and_then(|value| value.to_str())
            .unwrap_or("tmp")
    ))
}

pub(crate) fn append_crc32c(bytes: &mut Vec<u8>) {
    let checksum = crc32c(bytes);
    bytes.extend_from_slice(&checksum.to_le_bytes());
}

pub(crate) fn verify_crc32c(bytes: &[u8]) -> Option<&[u8]> {
    if bytes.len() < 4 {
        return None;
    }
    let payload_len = bytes.len() - 4;
    let expected = u32::from_le_bytes(bytes[payload_len..].try_into().ok()?);
    if crc32c(&bytes[..payload_len]) != expected {
        return None;
    }
    Some(&bytes[..payload_len])
}
