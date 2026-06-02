use std::fs;
use std::path::Path;

use cortex_storage::wal::WalWriter;

use crate::backup::{reject_existing_target, reject_target_inside_source};
use crate::database::Database;
use crate::error::{EngineError, EngineResult};
use crate::validation::StorageValidation;

mod codec;
mod crypto;
mod filesystem;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EncryptedBackupReport {
    pub files_archived: usize,
    pub plaintext_bytes: u64,
    pub ciphertext_bytes: u64,
    pub source_validation: StorageValidation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EncryptedRestoreReport {
    pub files_restored: usize,
    pub plaintext_bytes: u64,
    pub ciphertext_bytes: u64,
    pub restored_validation: StorageValidation,
}

pub(super) struct ArchiveEntry {
    path: String,
    bytes: Vec<u8>,
}

impl Database {
    pub fn encrypted_backup_path(
        source_path: impl AsRef<Path>,
        archive_path: impl AsRef<Path>,
        passphrase: &str,
    ) -> EngineResult<EncryptedBackupReport> {
        let mut db = Database::open(source_path)?;
        let report = db.encrypted_backup_to(archive_path, passphrase)?;
        db.close()?;
        Ok(report)
    }

    pub fn encrypted_backup_to(
        &mut self,
        archive_path: impl AsRef<Path>,
        passphrase: &str,
    ) -> EngineResult<EncryptedBackupReport> {
        validate_passphrase(passphrase)?;
        reject_target_inside_source(&self.root_path.canonicalize()?, archive_path.as_ref())?;
        reject_existing_target(archive_path.as_ref())?;
        let source_validation = self.validate_storage()?;
        self.writer.shutdown()?;

        let backup_result =
            codec::write_encrypted_archive(&self.root_path, archive_path.as_ref(), passphrase);
        let restart_result = WalWriter::start(&self.wal_path, self.durability_mode);
        self.writer = restart_result?;

        let copied = backup_result?;
        Ok(EncryptedBackupReport {
            files_archived: copied.files_copied,
            plaintext_bytes: copied.bytes_copied,
            ciphertext_bytes: fs::metadata(archive_path)?.len(),
            source_validation,
        })
    }

    pub fn restore_from_encrypted_backup(
        archive_path: impl AsRef<Path>,
        target_path: impl AsRef<Path>,
        passphrase: &str,
    ) -> EngineResult<EncryptedRestoreReport> {
        validate_passphrase(passphrase)?;
        reject_existing_target(target_path.as_ref())?;
        let decoded = codec::read_encrypted_archive(archive_path.as_ref(), passphrase)?;
        let report =
            filesystem::restore_plaintext_archive(&decoded.plaintext, target_path.as_ref())?;
        let db = Database::open(target_path)?;
        let restored_validation = db.validate_storage()?;
        db.close()?;
        Ok(EncryptedRestoreReport {
            files_restored: report.files_copied,
            plaintext_bytes: decoded.plaintext.len() as u64,
            ciphertext_bytes: decoded.ciphertext_len,
            restored_validation,
        })
    }
}

pub(super) fn validate_passphrase(passphrase: &str) -> EngineResult<()> {
    if passphrase.len() < 12 {
        return Err(EngineError::StorageInvariant(
            "encrypted backup passphrase must be at least 12 bytes".to_owned(),
        ));
    }
    Ok(())
}
