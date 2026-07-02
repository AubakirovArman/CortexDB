use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[cfg(test)]
use crate::config::ServerOptions;

const DATABASE_INSTANCE_ID_SCHEMA: &str = "cortexdb.database_instance_identity.v1";
const DATABASE_INSTANCE_ID_FILE: &str = "cortexdb.database_instance_identity.json";
const DATABASE_INSTANCE_ID_PREFIX: &str = "dbi_";

#[derive(Debug, Deserialize, Serialize)]
struct DatabaseInstanceIdentityFile {
    schema_version: String,
    db_instance_id: String,
}

#[cfg(test)]
pub(crate) fn with_database_instance_id(
    root: &Path,
    options: &ServerOptions,
) -> std::io::Result<ServerOptions> {
    let mut resolved = options.clone();
    if (resolved.receipt_signing_key.is_some() || resolved.receipt_external_signer.is_some())
        && resolved.db_instance_id.is_none()
    {
        resolved.db_instance_id = Some(load_or_create_database_instance_id(root)?);
    }
    Ok(resolved)
}

pub(crate) fn load_or_create_database_instance_id(root: &Path) -> std::io::Result<String> {
    let path = root.join(DATABASE_INSTANCE_ID_FILE);
    match read_database_instance_id(&path) {
        Ok(value) => return Ok(value),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }

    fs::create_dir_all(root)?;
    let record = DatabaseInstanceIdentityFile {
        schema_version: DATABASE_INSTANCE_ID_SCHEMA.to_owned(),
        db_instance_id: generate_database_instance_id()?,
    };
    let body = serde_json::to_vec_pretty(&record)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    match OpenOptions::new().write(true).create_new(true).open(&path) {
        Ok(mut file) => {
            file.write_all(&body)?;
            file.write_all(b"\n")?;
            file.sync_all()?;
            sync_parent_dir(root)?;
            Ok(record.db_instance_id)
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            read_database_instance_id(&path)
        }
        Err(error) => Err(error),
    }
}

pub(crate) fn validate_database_instance_id(value: &str) -> Result<(), String> {
    let Some(hex) = value.strip_prefix(DATABASE_INSTANCE_ID_PREFIX) else {
        return Err("database instance identity must start with dbi_".to_owned());
    };
    if hex.len() != 64 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(
            "database instance identity must be dbi_ followed by 64 hex characters".to_owned(),
        );
    }
    Ok(())
}

fn read_database_instance_id(path: &Path) -> std::io::Result<String> {
    let raw = fs::read_to_string(path)?;
    let record: DatabaseInstanceIdentityFile = serde_json::from_str(&raw)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    if record.schema_version != DATABASE_INSTANCE_ID_SCHEMA {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "database instance identity schema_version is invalid",
        ));
    }
    validate_database_instance_id(&record.db_instance_id)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    Ok(record.db_instance_id)
}

fn generate_database_instance_id() -> std::io::Result<String> {
    let seed = cortex_crypto::generate_signing_seed()
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    Ok(format!(
        "{DATABASE_INSTANCE_ID_PREFIX}{}",
        cortex_crypto::hex_lower(seed.as_bytes())
    ))
}

fn sync_parent_dir(root: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        File::open(root)?.sync_all()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_or_create_database_instance_id_reuses_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let first = load_or_create_database_instance_id(dir.path()).unwrap();
        let second = load_or_create_database_instance_id(dir.path()).unwrap();

        assert_eq!(first, second);
        assert!(first.starts_with("dbi_"));
    }

    #[test]
    fn load_or_create_database_instance_id_rejects_invalid_file() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join(DATABASE_INSTANCE_ID_FILE),
            r#"{"schema_version":"cortexdb.database_instance_identity.v1","db_instance_id":"local:default"}"#,
        )
        .unwrap();

        let error = load_or_create_database_instance_id(dir.path()).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    }
}
