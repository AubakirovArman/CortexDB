use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::error::{StorageError, StorageResult};

use super::codec::WalCodec;
use super::record::DecodedWalRecord;

#[derive(Clone, Debug)]
pub struct WalReader {
    path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalScan {
    pub records: Vec<DecodedWalRecord>,
    pub safe_truncate_offset: u64,
}

impl WalReader {
    pub fn open(path: impl AsRef<Path>) -> StorageResult<Self> {
        let path = path.as_ref().to_owned();
        File::open(&path)
            .map_err(|error| wal_path_error(&path, "failed to open WAL file", error))?;
        Ok(Self { path })
    }

    pub fn scan_path(path: impl AsRef<Path>) -> StorageResult<WalScan> {
        Self::open(path)?.scan()
    }

    pub fn scan_best_effort_path(path: impl AsRef<Path>) -> StorageResult<WalScan> {
        Self::open(path)?.scan_best_effort()
    }

    pub fn scan(&self) -> StorageResult<WalScan> {
        let mut file = File::open(&self.path)
            .map_err(|error| wal_path_error(&self.path, "failed to open WAL file", error))?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|error| wal_path_error(&self.path, "failed to read WAL file", error))?;
        scan_bytes(&bytes, true)
    }

    pub fn scan_best_effort(&self) -> StorageResult<WalScan> {
        let mut file = File::open(&self.path)
            .map_err(|error| wal_path_error(&self.path, "failed to open WAL file", error))?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .map_err(|error| wal_path_error(&self.path, "failed to read WAL file", error))?;
        scan_bytes(&bytes, false)
    }
}

fn wal_path_error(path: &Path, action: &str, error: std::io::Error) -> StorageError {
    StorageError::Io(std::io::Error::new(
        error.kind(),
        format!("{action} {}: {error}", path.display()),
    ))
}

fn scan_bytes(bytes: &[u8], strict_checksums: bool) -> StorageResult<WalScan> {
    if bytes.is_empty() {
        return Ok(WalScan {
            records: Vec::new(),
            safe_truncate_offset: 0,
        });
    }
    if bytes.len() < WalCodec::file_header_len() {
        return Err(StorageError::InvalidWalFileHeader);
    }
    WalCodec::validate_file_header(&bytes[..WalCodec::file_header_len()])?;
    let mut offset = WalCodec::file_header_len();
    let mut records = Vec::new();
    while offset < bytes.len() {
        match WalCodec::scan_next(&bytes[offset..]) {
            Ok(Some(decoded)) => {
                offset += decoded.bytes_consumed;
                records.push(decoded);
            }
            Ok(None) | Err(StorageError::IncompleteTail) | Err(StorageError::InvalidWalRecord) => {
                break;
            }
            Err(_) if !strict_checksums => break,
            Err(error) => return Err(error),
        }
    }
    Ok(WalScan {
        records,
        safe_truncate_offset: offset as u64,
    })
}
