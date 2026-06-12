use cortex_engine::Database;

use crate::{manifest, wal};

use super::common::{fmt_engine_error, open_database};

pub fn gc_retired(path: &str) -> Result<String, String> {
    let mut db = open_database(path, false)?;
    let report = db
        .garbage_collect_retired_segments()
        .map_err(fmt_engine_error)?;
    Ok(format!(
        "retired_segments_removed={} files_removed={}",
        report.retired_segments_removed, report.files_removed
    ))
}

pub fn unlock(path: &str, force: bool) -> Result<String, String> {
    if !force {
        return Err("unlock requires --force. Warning: this may corrupt data if another process is using the database.\n  → try: cortexdb unlock <path> --force".to_owned());
    }
    Database::break_stale_lock(path).map_err(fmt_engine_error)?;
    Ok("stale lock removed".to_owned())
}

pub fn wal_validate(path: &str) -> Result<String, String> {
    wal::validate(path)
}

pub fn wal_dump(path: &str) -> Result<String, String> {
    wal::dump(path)
}

pub fn wal_truncate(path: &str) -> Result<String, String> {
    wal::truncate(path)
}

pub fn manifest_dump(path: &str) -> Result<String, String> {
    manifest::dump(path)
}

pub fn manifest_validate(path: &str) -> Result<String, String> {
    manifest::validate(path)
}
