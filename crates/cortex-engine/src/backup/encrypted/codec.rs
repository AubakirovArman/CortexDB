use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::backup::sync_dir;
use crate::backup::CopyReport;
use crate::error::{EngineError, EngineResult};

use super::crypto;
use super::filesystem::{collect_archive_entries, validate_relative_archive_path};
use super::ArchiveEntry;

const ARCHIVE_MAGIC: &[u8] = b"CDBENC1\n";
const PLAINTEXT_MAGIC: &[u8] = b"CDBBACKUP1\n";

#[derive(Debug, Deserialize, Serialize)]
struct ArchiveHeader {
    schema_version: String,
    cipher_suite: String,
    kdf: String,
    nonce: u64,
    file_count: usize,
    plaintext_len: u64,
    ciphertext_len: u64,
    plaintext_hash: String,
    ciphertext_hash: String,
    auth_tag: String,
}

pub(super) struct DecodedArchive {
    pub(super) plaintext: Vec<u8>,
    pub(super) ciphertext_len: u64,
}

pub(super) fn write_encrypted_archive(
    source: &Path,
    archive_path: &Path,
    passphrase: &str,
) -> EngineResult<CopyReport> {
    let entries = collect_archive_entries(source)?;
    let plaintext = encode_plaintext_archive(&entries)?;
    let nonce = crypto::archive_nonce(source, archive_path, entries.len(), plaintext.len());
    let ciphertext = crypto::apply_keystream(&plaintext, passphrase, nonce);
    let header = ArchiveHeader {
        schema_version: crypto::SCHEMA_VERSION.to_owned(),
        cipher_suite: crypto::CIPHER_SUITE.to_owned(),
        kdf: crypto::KDF.to_owned(),
        nonce,
        file_count: entries.len(),
        plaintext_len: plaintext.len() as u64,
        ciphertext_len: ciphertext.len() as u64,
        plaintext_hash: crypto::hash_hex(&plaintext),
        ciphertext_hash: crypto::hash_hex(&ciphertext),
        auth_tag: crypto::auth_tag(passphrase, nonce, &plaintext, &ciphertext),
    };
    write_archive_file(archive_path, &header, &ciphertext)?;
    Ok(CopyReport {
        files_copied: entries.len(),
        bytes_copied: plaintext.len() as u64,
    })
}

pub(super) fn read_encrypted_archive(
    path: &Path,
    passphrase: &str,
) -> EngineResult<DecodedArchive> {
    let raw = fs::read(path)?;
    if !raw.starts_with(ARCHIVE_MAGIC) {
        return Err(EngineError::StorageInvariant(
            "encrypted backup archive has invalid magic".to_owned(),
        ));
    }
    let header_len_start = ARCHIVE_MAGIC.len();
    let header_len_end = header_len_start + 4;
    if raw.len() < header_len_end {
        return Err(EngineError::StorageInvariant(
            "encrypted backup archive header is truncated".to_owned(),
        ));
    }
    let header_len = read_fixed_u32(&raw[header_len_start..header_len_end])? as usize;
    let header_end = header_len_end.checked_add(header_len).ok_or_else(|| {
        EngineError::StorageInvariant("encrypted backup header is too large".to_owned())
    })?;
    if raw.len() < header_end {
        return Err(EngineError::StorageInvariant(
            "encrypted backup archive header length exceeds file size".to_owned(),
        ));
    }
    let header = serde_json::from_slice::<ArchiveHeader>(&raw[header_len_end..header_end])
        .map_err(|error| {
            EngineError::StorageInvariant(format!("invalid encrypted backup header: {error}"))
        })?;
    validate_header(&header)?;
    let ciphertext = &raw[header_end..];
    validate_ciphertext(&header, ciphertext)?;
    let plaintext = crypto::apply_keystream(ciphertext, passphrase, header.nonce);
    validate_plaintext(passphrase, &header, &plaintext, ciphertext)?;
    Ok(DecodedArchive {
        plaintext,
        ciphertext_len: ciphertext.len() as u64,
    })
}

pub(super) fn decode_plaintext_archive(plaintext: &[u8]) -> EngineResult<Vec<ArchiveEntry>> {
    if !plaintext.starts_with(PLAINTEXT_MAGIC) {
        return Err(EngineError::StorageInvariant(
            "encrypted backup plaintext has invalid magic".to_owned(),
        ));
    }
    let mut offset = PLAINTEXT_MAGIC.len();
    let count = read_u64(plaintext, &mut offset)? as usize;
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        let path_len = read_u32(plaintext, &mut offset)? as usize;
        let data_len = read_u64(plaintext, &mut offset)? as usize;
        let path_end = offset.checked_add(path_len).ok_or_else(|| {
            EngineError::StorageInvariant("encrypted backup path length overflow".to_owned())
        })?;
        if path_end > plaintext.len() {
            return Err(EngineError::StorageInvariant(
                "encrypted backup path exceeds archive length".to_owned(),
            ));
        }
        let path = std::str::from_utf8(&plaintext[offset..path_end])
            .map_err(|_| {
                EngineError::StorageInvariant("encrypted backup path is not UTF-8".to_owned())
            })?
            .to_owned();
        validate_relative_archive_path(&path)?;
        offset = path_end;
        let data_end = offset.checked_add(data_len).ok_or_else(|| {
            EngineError::StorageInvariant("encrypted backup data length overflow".to_owned())
        })?;
        if data_end > plaintext.len() {
            return Err(EngineError::StorageInvariant(
                "encrypted backup data exceeds archive length".to_owned(),
            ));
        }
        entries.push(ArchiveEntry {
            path,
            bytes: plaintext[offset..data_end].to_vec(),
        });
        offset = data_end;
    }
    if offset != plaintext.len() {
        return Err(EngineError::StorageInvariant(
            "encrypted backup has trailing plaintext bytes".to_owned(),
        ));
    }
    Ok(entries)
}

fn encode_plaintext_archive(entries: &[ArchiveEntry]) -> EngineResult<Vec<u8>> {
    let mut out = Vec::new();
    out.extend_from_slice(PLAINTEXT_MAGIC);
    out.extend_from_slice(&(entries.len() as u64).to_le_bytes());
    for entry in entries {
        validate_relative_archive_path(&entry.path)?;
        let path = entry.path.as_bytes();
        let path_len = u32::try_from(path.len()).map_err(|_| {
            EngineError::StorageInvariant("encrypted backup path length overflow".to_owned())
        })?;
        out.extend_from_slice(&path_len.to_le_bytes());
        out.extend_from_slice(&(entry.bytes.len() as u64).to_le_bytes());
        out.extend_from_slice(path);
        out.extend_from_slice(&entry.bytes);
    }
    Ok(out)
}

fn write_archive_file(path: &Path, header: &ArchiveHeader, ciphertext: &[u8]) -> EngineResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let header = serde_json::to_vec(header).map_err(|error| {
        EngineError::StorageInvariant(format!("encrypted backup header failed: {error}"))
    })?;
    let header_len = u32::try_from(header.len()).map_err(|_| {
        EngineError::StorageInvariant("encrypted backup header length overflow".to_owned())
    })?;
    let mut file = File::create(path)?;
    file.write_all(ARCHIVE_MAGIC)?;
    file.write_all(&header_len.to_le_bytes())?;
    file.write_all(&header)?;
    file.write_all(ciphertext)?;
    file.sync_all()?;
    if let Some(parent) = path.parent() {
        sync_dir(parent)?;
    }
    Ok(())
}

fn validate_header(header: &ArchiveHeader) -> EngineResult<()> {
    if header.schema_version != crypto::SCHEMA_VERSION
        || header.cipher_suite != crypto::CIPHER_SUITE
        || header.kdf != crypto::KDF
    {
        return Err(EngineError::StorageInvariant(
            "unsupported encrypted backup header".to_owned(),
        ));
    }
    Ok(())
}

fn validate_ciphertext(header: &ArchiveHeader, ciphertext: &[u8]) -> EngineResult<()> {
    if ciphertext.len() as u64 != header.ciphertext_len {
        return Err(EngineError::StorageInvariant(
            "encrypted backup ciphertext length mismatch".to_owned(),
        ));
    }
    if crypto::hash_hex(ciphertext) != header.ciphertext_hash {
        return Err(EngineError::StorageInvariant(
            "encrypted backup ciphertext checksum mismatch".to_owned(),
        ));
    }
    Ok(())
}

fn validate_plaintext(
    passphrase: &str,
    header: &ArchiveHeader,
    plaintext: &[u8],
    ciphertext: &[u8],
) -> EngineResult<()> {
    if plaintext.len() as u64 != header.plaintext_len
        || crypto::hash_hex(plaintext) != header.plaintext_hash
    {
        return Err(EngineError::StorageInvariant(
            "encrypted backup passphrase or plaintext checksum is invalid".to_owned(),
        ));
    }
    if crypto::auth_tag(passphrase, header.nonce, plaintext, ciphertext) != header.auth_tag {
        return Err(EngineError::StorageInvariant(
            "encrypted backup authentication tag mismatch".to_owned(),
        ));
    }
    Ok(())
}

fn read_u32(bytes: &[u8], offset: &mut usize) -> EngineResult<u32> {
    let end = offset.checked_add(4).ok_or_else(|| {
        EngineError::StorageInvariant("encrypted backup offset overflow".to_owned())
    })?;
    if end > bytes.len() {
        return Err(EngineError::StorageInvariant(
            "encrypted backup u32 field is truncated".to_owned(),
        ));
    }
    let value = read_fixed_u32(&bytes[*offset..end])?;
    *offset = end;
    Ok(value)
}

fn read_u64(bytes: &[u8], offset: &mut usize) -> EngineResult<u64> {
    let end = offset.checked_add(8).ok_or_else(|| {
        EngineError::StorageInvariant("encrypted backup offset overflow".to_owned())
    })?;
    if end > bytes.len() {
        return Err(EngineError::StorageInvariant(
            "encrypted backup u64 field is truncated".to_owned(),
        ));
    }
    let value = read_fixed_u64(&bytes[*offset..end])?;
    *offset = end;
    Ok(value)
}

fn read_fixed_u32(bytes: &[u8]) -> EngineResult<u32> {
    let array: [u8; 4] = bytes.try_into().map_err(|_| {
        EngineError::StorageInvariant("encrypted backup u32 field is malformed".to_owned())
    })?;
    Ok(u32::from_le_bytes(array))
}

fn read_fixed_u64(bytes: &[u8]) -> EngineResult<u64> {
    let array: [u8; 8] = bytes.try_into().map_err(|_| {
        EngineError::StorageInvariant("encrypted backup u64 field is malformed".to_owned())
    })?;
    Ok(u64::from_le_bytes(array))
}
