use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use crate::error::EngineResult;

pub(crate) fn cleanup_orphans(root: &Path) -> EngineResult<()> {
    cleanup_dir(root)?;
    cleanup_dir(&root.join("segments"))?;
    Ok(())
}

fn cleanup_dir(path: &Path) -> EngineResult<()> {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    for entry in entries {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_file() && is_known_temp(&entry.path()) {
            fs::remove_file(entry.path())?;
        }
    }
    Ok(())
}

fn is_known_temp(path: &Path) -> bool {
    let Some(name) = file_name(path) else {
        return false;
    };
    name.ends_with(".tmp")
        && [".acs.tmp", ".acb.tmp", ".aci.tmp", ".acm.tmp", ".aclog.tmp"]
            .iter()
            .any(|suffix| name.ends_with(suffix))
}

fn file_name(path: &Path) -> Option<String> {
    path.file_name()
        .and_then(|value| value.to_str())
        .map(ToOwned::to_owned)
}

pub(crate) fn lock_path(root: &Path) -> PathBuf {
    root.join("db.lock")
}
