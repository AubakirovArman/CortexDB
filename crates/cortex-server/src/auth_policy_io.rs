use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

pub(crate) fn atomic_write_text(path: &Path, text: &str) -> Result<(), String> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)
        .map_err(|error| format!("failed to create policy store directory: {error}"))?;
    let tmp_path = tmp_path(path);
    {
        let mut file = File::create(&tmp_path)
            .map_err(|error| format!("failed to create temp policy store: {error}"))?;
        file.write_all(text.as_bytes())
            .map_err(|error| format!("failed to write temp policy store: {error}"))?;
        file.sync_all()
            .map_err(|error| format!("failed to sync temp policy store: {error}"))?;
    }
    fs::rename(&tmp_path, path)
        .map_err(|error| format!("failed to publish policy store: {error}"))?;
    if let Ok(directory) = File::open(parent) {
        let _ = directory.sync_all();
    }
    Ok(())
}

pub(crate) fn rollback_path(path: &Path) -> PathBuf {
    path.with_extension(format!(
        "{}.rollback",
        path.extension()
            .and_then(|value| value.to_str())
            .unwrap_or("json")
    ))
}

fn tmp_path(path: &Path) -> PathBuf {
    path.with_extension(format!(
        "{}.tmp",
        path.extension()
            .and_then(|value| value.to_str())
            .unwrap_or("json")
    ))
}
