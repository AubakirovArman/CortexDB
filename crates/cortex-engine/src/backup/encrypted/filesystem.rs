use std::fs::{self, File};
use std::io::Write;
use std::path::{Component, Path};

use crate::backup::{is_known_temp_name, sync_dir, CopyReport};
use crate::error::{EngineError, EngineResult};

use super::codec::decode_plaintext_archive;
use super::ArchiveEntry;

pub(super) fn collect_archive_entries(root: &Path) -> EngineResult<Vec<ArchiveEntry>> {
    let mut entries = Vec::new();
    collect_archive_entries_inner(root, root, &mut entries)?;
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(entries)
}

pub(super) fn restore_plaintext_archive(
    plaintext: &[u8],
    target: &Path,
) -> EngineResult<CopyReport> {
    let entries = decode_plaintext_archive(plaintext)?;
    fs::create_dir(target)?;
    let result = write_restored_entries(target, &entries);
    if result.is_err() {
        let _ = fs::remove_dir_all(target);
    }
    result
}

pub(super) fn validate_relative_archive_path(path: &str) -> EngineResult<()> {
    if path.is_empty() || path.starts_with('/') || path.contains('\\') {
        return Err(EngineError::StorageInvariant(
            "encrypted backup contains unsafe path".to_owned(),
        ));
    }
    for component in Path::new(path).components() {
        if matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        ) {
            return Err(EngineError::StorageInvariant(
                "encrypted backup contains unsafe path".to_owned(),
            ));
        }
    }
    Ok(())
}

fn collect_archive_entries_inner(
    root: &Path,
    current: &Path,
    entries: &mut Vec<ArchiveEntry>,
) -> EngineResult<()> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name == "db.lock" || is_known_temp_name(&name) {
            continue;
        }
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            collect_archive_entries_inner(root, &path, entries)?;
        } else if file_type.is_file() {
            let rel = relative_archive_path(root, &path)?;
            entries.push(ArchiveEntry {
                path: rel,
                bytes: fs::read(path)?,
            });
        } else {
            return Err(EngineError::StorageInvariant(format!(
                "encrypted backup refuses non-regular file: {}",
                path.display()
            )));
        }
    }
    Ok(())
}

fn write_restored_entries(target: &Path, entries: &[ArchiveEntry]) -> EngineResult<CopyReport> {
    let mut report = CopyReport::default();
    for entry in entries {
        let path = target.join(&entry.path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = File::create(&path)?;
        file.write_all(&entry.bytes)?;
        file.sync_all()?;
        report.files_copied += 1;
        report.bytes_copied += entry.bytes.len() as u64;
    }
    sync_restored_dirs(target)?;
    Ok(report)
}

fn sync_restored_dirs(path: &Path) -> EngineResult<()> {
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            sync_restored_dirs(&entry.path())?;
        }
    }
    sync_dir(path)
}

fn relative_archive_path(root: &Path, path: &Path) -> EngineResult<String> {
    let rel = path.strip_prefix(root).map_err(|_| {
        EngineError::StorageInvariant("encrypted backup path escaped source root".to_owned())
    })?;
    let mut parts = Vec::new();
    for component in rel.components() {
        match component {
            Component::Normal(value) => parts.push(value.to_string_lossy().into_owned()),
            _ => {
                return Err(EngineError::StorageInvariant(
                    "encrypted backup contains unsafe source path".to_owned(),
                ));
            }
        }
    }
    Ok(parts.join("/"))
}
