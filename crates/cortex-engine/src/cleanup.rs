use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use crate::error::{EngineError, EngineResult};

pub(crate) fn cleanup_orphans(root: &Path) -> EngineResult<usize> {
    Ok(cleanup_dir(root, true)?
        + cleanup_dir(&root.join("segments"), true)?
        + cleanup_dir(&root.join("agent_views"), true)?)
}

pub(crate) fn count_orphans(root: &Path) -> EngineResult<usize> {
    Ok(cleanup_dir(root, false)?
        + cleanup_dir(&root.join("segments"), false)?
        + cleanup_dir(&root.join("agent_views"), false)?)
}

fn cleanup_dir(path: &Path, remove: bool) -> EngineResult<usize> {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(0),
        Err(error) => return Err(error.into()),
    };
    let mut matched = 0;
    for entry in entries {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_file() && is_known_temp(&entry.path()) {
            if remove {
                fs::remove_file(entry.path())?;
            }
            matched += 1;
        }
    }
    Ok(matched)
}

fn is_known_temp(path: &Path) -> bool {
    let Some(name) = file_name(path) else {
        return false;
    };
    [
        ".acs.tmp",
        ".acb.tmp",
        ".aci.tmp",
        ".acv.tmp",
        ".acm.tmp",
        ".aclog.tmp",
        ".view.tmp",
    ]
    .iter()
    .any(|marker| name.ends_with(marker) || name.contains(&format!("{marker}.")))
}

fn file_name(path: &Path) -> Option<String> {
    path.file_name()
        .and_then(|value| value.to_str())
        .map(ToOwned::to_owned)
}

pub(crate) fn lock_path(root: &Path) -> PathBuf {
    root.join("db.lock")
}

pub(crate) fn remove_lock_file(root: &Path) -> EngineResult<()> {
    match fs::remove_file(lock_path(root)) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(EngineError::from(error)),
    }
}
