use std::fs::{self, File, OpenOptions};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

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
            Ok(file) => Ok(Self { path, _file: file }),
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
