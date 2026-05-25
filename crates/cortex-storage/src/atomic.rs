use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;

use crate::error::StorageResult;
use crate::wal::checksum::crc32c;

pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> StorageResult<()> {
    let tmp = path.with_extension(format!(
        "{}.tmp",
        path.extension()
            .and_then(|value| value.to_str())
            .unwrap_or("tmp")
    ));
    {
        let mut file = File::create(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    if let Some(parent) = path.parent() {
        OpenOptions::new().read(true).open(parent)?.sync_all()?;
    }
    Ok(())
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
