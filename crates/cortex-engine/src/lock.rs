use std::fs::{self, File, OpenOptions};
use std::io::ErrorKind;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::cleanup::lock_path;
use crate::error::{EngineError, EngineResult};

#[derive(Debug)]
pub(crate) struct DatabaseLock {
    path: PathBuf,
    _file: File,
}

impl DatabaseLock {
    pub(crate) fn acquire(root: &Path) -> EngineResult<Self> {
        let path = lock_path(root);
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                if let Err(error) = file.write_all(lock_metadata(root).as_bytes()) {
                    let _ = fs::remove_file(&path);
                    return Err(error.into());
                }
                if let Err(error) = file.sync_data() {
                    let _ = fs::remove_file(&path);
                    return Err(error.into());
                }
                Ok(Self { path, _file: file })
            }
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                Err(EngineError::DatabaseAlreadyOpen(path))
            }
            Err(error) => Err(error.into()),
        }
    }
}

impl Drop for DatabaseLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn lock_metadata(root: &Path) -> String {
    let created_unix_seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    format!(
        "format=cortexdb-lock-v1\npid={}\ncreated_unix_seconds={created_unix_seconds}\nroot={}\n",
        std::process::id(),
        root.display()
    )
}
