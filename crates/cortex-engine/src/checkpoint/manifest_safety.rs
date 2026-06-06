use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use crate::error::{EngineError, EngineResult};

use super::paths::{manifest_path, segments_path};

pub(crate) fn reject_missing_manifest_with_storage(root: &Path) -> EngineResult<()> {
    if manifest_path(root).try_exists()? {
        return Ok(());
    }
    let segments = segments_path(root);
    let entries = match fs::read_dir(&segments) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    for entry in entries {
        let path = entry?.path();
        if path.is_file() && is_storage_bundle_file(&path) {
            return Err(EngineError::StorageInvariant(
                "missing manifest.acm while persisted segment files exist".to_owned(),
            ));
        }
    }
    Ok(())
}

fn is_storage_bundle_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("acs" | "acb" | "aci" | "acv" | "ach")
    )
}
