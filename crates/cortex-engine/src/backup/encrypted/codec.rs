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
    kdf_params: ArchiveKdfParams,
    salt: String,
    nonce: String,
    file_count: usize,
    plaintext_len: u64,
    ciphertext_len: u64,
    aead_tag: String,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
struct ArchiveKdfParams {
    memory_cost_kib: u32,
    time_cost: u32,
    parallelism: u32,
    output_len: usize,
}

#[derive(Debug, Deserialize)]
struct ArchiveHeaderProbe {
    schema_version: Option<String>,
}

#[derive(Serialize)]
struct ArchiveHeaderAad<'a> {
    schema_version: &'a str,
    cipher_suite: &'a str,
    kdf: &'a str,
    kdf_params: ArchiveKdfParams,
    salt: &'a str,
    nonce: &'a str,
    file_count: usize,
    plaintext_len: u64,
    ciphertext_len: u64,
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
    let mut header = ArchiveHeader {
        schema_version: crypto::SCHEMA_VERSION.to_owned(),
        cipher_suite: crypto::CIPHER_SUITE.to_owned(),
        kdf: crypto::KDF.to_owned(),
        kdf_params: ArchiveKdfParams::from_crypto(crypto::kdf_params()),
        salt: crypto::generate_salt_hex()?,
        nonce: crypto::generate_nonce_hex()?,
        file_count: entries.len(),
        plaintext_len: plaintext.len() as u64,
        ciphertext_len: plaintext.len() as u64,
        aead_tag: String::new(),
    };
    let aad = header_aad(&header)?;
    let sealed = crypto::seal_archive(passphrase, &header.salt, &header.nonce, &aad, &plaintext)?;
    header.ciphertext_len = sealed.ciphertext.len() as u64;
    header.aead_tag = sealed.tag_hex;
    write_archive_file(archive_path, &header, &sealed.ciphertext)?;
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
    let header = parse_archive_header(&raw[header_len_end..header_end])?;
    validate_header(&header)?;
    let ciphertext = &raw[header_end..];
    validate_ciphertext(&header, ciphertext)?;
    let aad = header_aad(&header)?;
    let plaintext = crypto::open_archive(
        passphrase,
        &header.salt,
        &header.nonce,
        &header.aead_tag,
        &aad,
        ciphertext,
    )?;
    validate_plaintext(&header, &plaintext)?;
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

fn parse_archive_header(bytes: &[u8]) -> EngineResult<ArchiveHeader> {
    let value = serde_json::from_slice::<serde_json::Value>(bytes).map_err(|error| {
        EngineError::StorageInvariant(format!("invalid encrypted backup header: {error}"))
    })?;
    let probe = serde_json::from_value::<ArchiveHeaderProbe>(value.clone()).map_err(|error| {
        EngineError::StorageInvariant(format!("invalid encrypted backup header: {error}"))
    })?;
    if probe.schema_version.as_deref() == Some(crypto::LEGACY_SCHEMA_VERSION) {
        return Err(EngineError::StorageInvariant(
            "legacy encrypted backup v1 is refused; recreate the backup with encrypted backup v2"
                .to_owned(),
        ));
    }
    serde_json::from_value::<ArchiveHeader>(value).map_err(|error| {
        EngineError::StorageInvariant(format!("invalid encrypted backup header: {error}"))
    })
}

fn header_aad(header: &ArchiveHeader) -> EngineResult<Vec<u8>> {
    let aad = ArchiveHeaderAad {
        schema_version: &header.schema_version,
        cipher_suite: &header.cipher_suite,
        kdf: &header.kdf,
        kdf_params: header.kdf_params,
        salt: &header.salt,
        nonce: &header.nonce,
        file_count: header.file_count,
        plaintext_len: header.plaintext_len,
        ciphertext_len: header.ciphertext_len,
    };
    serde_json::to_vec(&aad).map_err(|error| {
        EngineError::StorageInvariant(format!("encrypted backup AAD failed: {error}"))
    })
}

fn validate_header(header: &ArchiveHeader) -> EngineResult<()> {
    if header.schema_version != crypto::SCHEMA_VERSION
        || header.cipher_suite != crypto::CIPHER_SUITE
        || header.kdf != crypto::KDF
        || header.kdf_params != ArchiveKdfParams::from_crypto(crypto::kdf_params())
    {
        return Err(EngineError::StorageInvariant(
            "unsupported encrypted backup header".to_owned(),
        ));
    }
    validate_hex_len("encrypted backup salt", &header.salt, 16)?;
    validate_hex_len("encrypted backup nonce", &header.nonce, 24)?;
    validate_hex_len("encrypted backup tag", &header.aead_tag, 16)?;
    Ok(())
}

fn validate_ciphertext(header: &ArchiveHeader, ciphertext: &[u8]) -> EngineResult<()> {
    if ciphertext.len() as u64 != header.ciphertext_len {
        return Err(EngineError::StorageInvariant(
            "encrypted backup ciphertext length mismatch".to_owned(),
        ));
    }
    Ok(())
}

fn validate_plaintext(header: &ArchiveHeader, plaintext: &[u8]) -> EngineResult<()> {
    if plaintext.len() as u64 != header.plaintext_len {
        return Err(EngineError::StorageInvariant(
            "encrypted backup plaintext length mismatch".to_owned(),
        ));
    }
    Ok(())
}

fn validate_hex_len(name: &'static str, value: &str, bytes: usize) -> EngineResult<()> {
    if value.len() != bytes * 2 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(EngineError::StorageInvariant(format!(
            "{name} has invalid hex encoding"
        )));
    }
    Ok(())
}

impl ArchiveKdfParams {
    fn from_crypto(params: cortex_crypto::KdfParams) -> Self {
        Self {
            memory_cost_kib: params.memory_cost_kib,
            time_cost: params.time_cost,
            parallelism: params.parallelism,
            output_len: params.output_len,
        }
    }
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
