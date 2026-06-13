use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use cortex_storage::wal::checksum::crc32c;

use crate::error::{EngineError, EngineResult};

pub(super) const BACKUP_MANIFEST_FILE: &str = "backup_manifest.tsv";
const HEADER: &str = "# cortexdb-backup-manifest-v1\n";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct BackupManifestVerification {
    pub(super) present: bool,
    pub(super) files_verified: usize,
}

pub(super) fn write_backup_manifest(root: &Path) -> EngineResult<usize> {
    let entries = collect_manifest_entries(root)?;
    let path = root.join(BACKUP_MANIFEST_FILE);
    let mut output = File::create(&path)?;
    output.write_all(HEADER.as_bytes())?;
    for entry in &entries {
        writeln!(
            output,
            "{}\t{}\t{:08x}",
            entry.relative_path, entry.bytes, entry.crc32c
        )?;
    }
    output.sync_all()?;
    super::sync_dir(root)?;
    Ok(entries.len())
}

pub(super) fn verify_backup_manifest(root: &Path) -> EngineResult<BackupManifestVerification> {
    let path = root.join(BACKUP_MANIFEST_FILE);
    if !path.exists() {
        return Ok(BackupManifestVerification::default());
    }
    let manifest = fs::read_to_string(&path)?;
    let expected = parse_manifest(&manifest)?;
    let expected_paths = expected
        .iter()
        .map(|entry| entry.relative_path.as_str())
        .collect::<BTreeSet<_>>();
    let actual = collect_manifest_entries(root)?;
    for entry in &actual {
        if !expected_paths.contains(entry.relative_path.as_str()) {
            return Err(EngineError::StorageInvariant(format!(
                "backup manifest missing file {}",
                entry.relative_path
            )));
        }
    }
    for entry in &expected {
        verify_entry(root, entry)?;
    }
    Ok(BackupManifestVerification {
        present: true,
        files_verified: expected.len(),
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ManifestEntry {
    relative_path: String,
    bytes: u64,
    crc32c: u32,
}

fn collect_manifest_entries(root: &Path) -> EngineResult<Vec<ManifestEntry>> {
    let mut entries = Vec::new();
    collect_manifest_entries_inner(root, root, &mut entries)?;
    entries.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok(entries)
}

fn collect_manifest_entries_inner(
    root: &Path,
    current: &Path,
    entries: &mut Vec<ManifestEntry>,
) -> EngineResult<()> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect_manifest_entries_inner(root, &path, entries)?;
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        let relative = relative_manifest_path(root, &path)?;
        if relative == BACKUP_MANIFEST_FILE {
            continue;
        }
        let bytes = fs::read(&path)?;
        entries.push(ManifestEntry {
            relative_path: relative,
            bytes: bytes.len() as u64,
            crc32c: crc32c(&bytes),
        });
    }
    Ok(())
}

fn parse_manifest(input: &str) -> EngineResult<Vec<ManifestEntry>> {
    let mut entries = Vec::new();
    for (index, line) in input.lines().enumerate() {
        if index == 0 && line == HEADER.trim_end() {
            continue;
        }
        if line.trim().is_empty() {
            continue;
        }
        let parts = line.split('\t').collect::<Vec<_>>();
        if parts.len() != 3 {
            return Err(EngineError::StorageInvariant(format!(
                "invalid backup manifest line {}",
                index + 1
            )));
        }
        let bytes = parts[1].parse::<u64>().map_err(|_| {
            EngineError::StorageInvariant(format!(
                "invalid backup manifest size at line {}",
                index + 1
            ))
        })?;
        let crc32c = u32::from_str_radix(parts[2], 16).map_err(|_| {
            EngineError::StorageInvariant(format!(
                "invalid backup manifest checksum at line {}",
                index + 1
            ))
        })?;
        entries.push(ManifestEntry {
            relative_path: parts[0].to_owned(),
            bytes,
            crc32c,
        });
    }
    Ok(entries)
}

fn verify_entry(root: &Path, expected: &ManifestEntry) -> EngineResult<()> {
    let path = root.join(PathBuf::from(&expected.relative_path));
    let bytes = fs::read(&path).map_err(|error| {
        EngineError::StorageInvariant(format!(
            "backup manifest file {} is unreadable: {error}",
            expected.relative_path
        ))
    })?;
    if bytes.len() as u64 != expected.bytes {
        return Err(EngineError::StorageInvariant(format!(
            "backup manifest size mismatch for {}",
            expected.relative_path
        )));
    }
    let actual = crc32c(&bytes);
    if actual != expected.crc32c {
        return Err(EngineError::StorageInvariant(format!(
            "backup manifest checksum mismatch for {}",
            expected.relative_path
        )));
    }
    Ok(())
}

fn relative_manifest_path(root: &Path, path: &Path) -> EngineResult<String> {
    let relative = path.strip_prefix(root).map_err(|_| {
        EngineError::StorageInvariant(format!(
            "backup manifest path is outside root: {}",
            path.display()
        ))
    })?;
    Ok(relative
        .components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/"))
}
