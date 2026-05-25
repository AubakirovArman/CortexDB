use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::error::{StorageError, StorageResult};

use super::codec::WalCodec;
use super::record::DecodedWalRecord;

#[derive(Clone, Debug)]
pub struct WalReader;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalScan {
    pub records: Vec<DecodedWalRecord>,
    pub safe_truncate_offset: u64,
}

impl WalReader {
    pub fn scan(path: &Path) -> StorageResult<WalScan> {
        let mut file = File::open(path)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
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
            match WalCodec::decode_record(&bytes[offset..]) {
                Ok(decoded) => {
                    offset += decoded.bytes_consumed;
                    records.push(decoded);
                }
                Err(_) => break,
            }
        }
        Ok(WalScan {
            records,
            safe_truncate_offset: offset as u64,
        })
    }
}
